package controlplane

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
)

func (repository *PostgreSQLRepository) EnqueueSubmission(
	ctx context.Context,
	principal Principal,
	idempotencyKey string,
	submission normalizedSubmission,
) (Submission, bool, error) {
	return repository.EnqueueSubmissionWithDelivery(ctx, principal, idempotencyKey, submission, "direct_mx")
}

func (repository *PostgreSQLRepository) EnqueueSubmissionWithRelaySender(
	ctx context.Context,
	principal Principal,
	idempotencyKey string,
	submission normalizedSubmission,
	_ string,
) (Submission, bool, error) {
	return repository.enqueueSubmission(ctx, principal, idempotencyKey, submission, "relay")
}

// EnqueueSubmissionWithDelivery records the selected transport with the queued
// message. The value is selected from server configuration, never from client input.
func (repository *PostgreSQLRepository) EnqueueSubmissionWithDelivery(
	ctx context.Context,
	principal Principal,
	idempotencyKey string,
	submission normalizedSubmission,
	deliveryMode string,
) (Submission, bool, error) {
	if deliveryMode != "direct_mx" && deliveryMode != "relay" && deliveryMode != "mailpit" {
		return Submission{}, false, ErrInvalid
	}
	return repository.enqueueSubmission(ctx, principal, idempotencyKey, submission, deliveryMode)
}

func (repository *PostgreSQLRepository) enqueueSubmission(
	ctx context.Context,
	principal Principal,
	idempotencyKey string,
	submission normalizedSubmission,
	deliveryMode string,
) (Submission, bool, error) {
	scope := "api:" + principal.TenantID
	if scope == "api:" || len(scope) > 128 || !validIdempotencyKey(idempotencyKey) {
		return Submission{}, false, ErrInvalid
	}
	fingerprint, err := submissionFingerprint(submission)
	if err != nil {
		return Submission{}, false, err
	}

	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return Submission{}, false, fmt.Errorf("begin submission transaction: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()

	var existingID string
	var existingFingerprint []byte
	err = tx.QueryRow(ctx, `
		SELECT message_id::text, fingerprint
		FROM mail_idempotency
		WHERE scope = $1 AND idempotency_key = $2
		FOR UPDATE
	`, scope, idempotencyKey).Scan(&existingID, &existingFingerprint)
	if err == nil {
		if !equalFingerprint(existingFingerprint, fingerprint) {
			return Submission{}, false, errIdempotencyConflict
		}
		existing, err := readSubmission(ctx, tx, scope, existingID)
		if err != nil {
			return Submission{}, false, err
		}
		if err := tx.Commit(ctx); err != nil {
			return Submission{}, false, fmt.Errorf("complete idempotent submission lookup: %w", err)
		}
		return existing, false, nil
	}
	if !errors.Is(err, pgx.ErrNoRows) {
		return Submission{}, false, fmt.Errorf("look up submission idempotency key: %w", err)
	}
	if principal.durable {
		if err := requireVerifiedSenderDomain(ctx, tx, principal, submission.From); err != nil {
			return Submission{}, false, err
		}
	}
	recipients := append(append(append([]string{}, submission.To...), submission.Cc...), submission.Bcc...)
	for _, recipient := range recipients {
		var suppressed bool
		if err := tx.QueryRow(ctx, `
			SELECT EXISTS (
				SELECT 1
				FROM recipient_suppressions
				WHERE tenant_id = $1 AND lower(recipient) = lower($2) AND active = TRUE
			)
		`, principal.TenantID, recipient).Scan(&suppressed); err != nil {
			return Submission{}, false, fmt.Errorf("check recipient suppression: %w", err)
		}
		if suppressed {
			return Submission{}, false, ErrSuppressed
		}
	}
	senderDomain := strings.ToLower(submission.From[strings.LastIndexByte(submission.From, '@')+1:])
	dkimKeyRef := "secret-manager://dkim/" + senderDomain

	now := time.Now().UTC()
	var messageID string
	if err := tx.QueryRow(ctx, `
		INSERT INTO mail_messages
		  (id, envelope_sender, body, body_size, tenant_id, sender_domain, delivery_mode, dkim_key_ref, created_at)
		VALUES (gen_random_uuid(), $1, '', 0, $2, $3, $4, $5, $6)
		RETURNING id::text
	`, submission.From, principal.TenantID, senderDomain, deliveryMode, dkimKeyRef, now).Scan(&messageID); err != nil {
		return Submission{}, false, fmt.Errorf("allocate queued submission: %w", err)
	}
	body := submissionMessage(submission, messageID, now)
	if len(body) > maxSubmissionBytes {
		return Submission{}, false, ErrInvalid
	}
	indexedMetadata, err := indexedSubmissionMetadata(body)
	if err != nil {
		return Submission{}, false, fmt.Errorf("parse generated canonical submission: %w", err)
	}
	metadata, err := json.Marshal(indexedMetadata)
	if err != nil {
		return Submission{}, false, fmt.Errorf("encode canonical submission metadata: %w", err)
	}
	if len(metadata) > maxIndexedMetadataBytes {
		return Submission{}, false, ErrInvalid
	}
	if _, err := tx.Exec(ctx, `
		UPDATE mail_messages
		SET body = $1, body_size = $2, message_schema_version = 1, parsed_metadata = $3
		WHERE id = $4
	`, body, len(body), metadata, messageID); err != nil {
		return Submission{}, false, fmt.Errorf("store queued submission body: %w", err)
	}
	for _, recipient := range submission.To {
		if _, err := tx.Exec(ctx, `
			INSERT INTO mail_recipients (message_id, recipient, recipient_kind)
			VALUES ($1, $2, 'to')
		`, messageID, recipient); err != nil {
			return Submission{}, false, fmt.Errorf("store queued submission recipient: %w", err)
		}
	}
	for _, recipient := range submission.Cc {
		if _, err := tx.Exec(ctx, `
			INSERT INTO mail_recipients (message_id, recipient, recipient_kind)
			VALUES ($1, $2, 'cc')
		`, messageID, recipient); err != nil {
			return Submission{}, false, fmt.Errorf("store queued submission cc recipient: %w", err)
		}
	}
	for _, recipient := range submission.Bcc {
		if _, err := tx.Exec(ctx, `
			INSERT INTO mail_recipients (message_id, recipient, recipient_kind)
			VALUES ($1, $2, 'bcc')
		`, messageID, recipient); err != nil {
			return Submission{}, false, fmt.Errorf("store queued submission bcc recipient: %w", err)
		}
	}
	if _, err := tx.Exec(ctx, `
		INSERT INTO mail_queue
		  (message_id, state, next_attempt_at, attempt_count, created_at, updated_at)
		VALUES ($1, 'queued', NOW(), 0, NOW(), NOW())
	`, messageID); err != nil {
		return Submission{}, false, fmt.Errorf("enqueue submitted message: %w", err)
	}

	inserted, err := tx.Exec(ctx, `
		INSERT INTO mail_idempotency (scope, idempotency_key, fingerprint, message_id, created_at)
		VALUES ($1, $2, $3, $4, NOW())
		ON CONFLICT (scope, idempotency_key) DO NOTHING
	`, scope, idempotencyKey, fingerprint, messageID)
	if err != nil {
		return Submission{}, false, fmt.Errorf("record submission idempotency key: %w", err)
	}
	if inserted.RowsAffected() == 0 {
		var concurrentID string
		var concurrentFingerprint []byte
		if err := tx.QueryRow(ctx, `
			SELECT message_id::text, fingerprint
			FROM mail_idempotency WHERE scope = $1 AND idempotency_key = $2
		`, scope, idempotencyKey).Scan(&concurrentID, &concurrentFingerprint); err != nil {
			return Submission{}, false, fmt.Errorf("resolve concurrent submission: %w", err)
		}
		if !equalFingerprint(concurrentFingerprint, fingerprint) {
			return Submission{}, false, errIdempotencyConflict
		}
		if err := tx.Rollback(ctx); err != nil && !errors.Is(err, pgx.ErrTxClosed) {
			return Submission{}, false, fmt.Errorf("discard duplicate concurrent submission: %w", err)
		}
		existing, err := repository.GetSubmission(ctx, principal, concurrentID)
		return existing, false, err
	}

	item := Submission{
		ID:        messageID,
		From:      submission.From,
		To:        append([]string{}, submission.To...),
		Cc:        append([]string{}, submission.Cc...),
		Bcc:       append([]string{}, submission.Bcc...),
		Subject:   submission.Subject,
		Status:    "queued",
		CreatedAt: now,
	}
	if err := tx.Commit(ctx); err != nil {
		return Submission{}, false, fmt.Errorf("commit queued submission: %w", err)
	}
	return item, true, nil
}

func (repository *PostgreSQLRepository) ListSubmissions(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]Submission, error) {
	if limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	scope := "api:" + principal.TenantID
	rows, err := repository.pool.Query(ctx, `
		SELECT message.id::text, message.envelope_sender,
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'to' ORDER BY id)::text[],
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'cc' ORDER BY id)::text[],
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'bcc' ORDER BY id)::text[],
		       substring(message.body FROM 1 FOR 65536),
		       queue.state, message.created_at
		FROM mail_idempotency AS idempotency
		JOIN mail_messages AS message ON message.id = idempotency.message_id
		JOIN mail_queue AS queue ON queue.message_id = message.id
		WHERE idempotency.scope = $1
		ORDER BY message.created_at DESC, message.id DESC
		LIMIT $2
	`, scope, limit)
	if err != nil {
		return nil, fmt.Errorf("list queued submissions: %w", err)
	}
	defer rows.Close()

	items := make([]Submission, 0)
	for rows.Next() {
		var item Submission
		var rawHeaders []byte
		if err := rows.Scan(
			&item.ID, &item.From, &item.To, &item.Cc, &item.Bcc, &rawHeaders, &item.Status, &item.CreatedAt,
		); err != nil {
			return nil, fmt.Errorf("scan queued submission: %w", err)
		}
		item.Subject, err = submissionSubject(rawHeaders)
		if err != nil {
			return nil, err
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate queued submissions: %w", err)
	}
	return items, nil
}

func (repository *PostgreSQLRepository) GetSubmission(
	ctx context.Context,
	principal Principal,
	messageID string,
) (Submission, error) {
	scope := "api:" + principal.TenantID
	var item Submission
	var rawHeaders []byte
	err := repository.pool.QueryRow(ctx, `
		SELECT message.id::text, message.envelope_sender,
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'to' ORDER BY id)::text[],
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'cc' ORDER BY id)::text[],
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'bcc' ORDER BY id)::text[],
		       substring(message.body FROM 1 FOR 65536),
		       queue.state, message.created_at
		FROM mail_idempotency AS idempotency
		JOIN mail_messages AS message ON message.id = idempotency.message_id
		JOIN mail_queue AS queue ON queue.message_id = message.id
		WHERE idempotency.scope = $1 AND message.id = $2
	`, scope, messageID).Scan(
		&item.ID, &item.From, &item.To, &item.Cc, &item.Bcc, &rawHeaders, &item.Status, &item.CreatedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return Submission{}, ErrNotFound
	}
	if err != nil {
		return Submission{}, fmt.Errorf("get queued submission: %w", err)
	}
	item.Subject, err = submissionSubject(rawHeaders)
	if err != nil {
		return Submission{}, err
	}
	return item, nil
}

func readSubmission(
	ctx context.Context,
	tx pgx.Tx,
	scope string,
	messageID string,
) (Submission, error) {
	var item Submission
	var rawHeaders []byte
	err := tx.QueryRow(ctx, `
		SELECT message.id::text, message.envelope_sender,
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'to' ORDER BY id)::text[],
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'cc' ORDER BY id)::text[],
		       ARRAY(SELECT recipient FROM mail_recipients
		             WHERE message_id = message.id AND recipient_kind = 'bcc' ORDER BY id)::text[],
		       substring(message.body FROM 1 FOR 65536),
		       queue.state, message.created_at
		FROM mail_idempotency AS idempotency
		JOIN mail_messages AS message ON message.id = idempotency.message_id
		JOIN mail_queue AS queue ON queue.message_id = message.id
		WHERE idempotency.scope = $1 AND message.id = $2
	`, scope, messageID).Scan(
		&item.ID, &item.From, &item.To, &item.Cc, &item.Bcc, &rawHeaders, &item.Status, &item.CreatedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return Submission{}, ErrNotFound
	}
	if err != nil {
		return Submission{}, fmt.Errorf("read existing queued submission: %w", err)
	}
	item.Subject, err = submissionSubject(rawHeaders)
	if err != nil {
		return Submission{}, err
	}
	return item, nil
}

func equalFingerprint(left, right []byte) bool {
	if len(left) != len(right) {
		return false
	}
	var difference byte
	for index := range left {
		difference |= left[index] ^ right[index]
	}
	return difference == 0
}
