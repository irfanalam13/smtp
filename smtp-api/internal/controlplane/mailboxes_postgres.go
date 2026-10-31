package controlplane

import (
	"context"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"

	"github.com/jackc/pgx/v5"
)

func (repository *PostgreSQLRepository) ListMailboxes(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]Mailbox, error) {
	if !validMailboxPrincipal(principal) {
		return nil, ErrForbidden
	}
	if limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT mailbox.id::text, mailbox.tenant_id::text,
		       mailbox.local_part || '@' || domain.domain AS address,
		       mailbox.status, mailbox.created_at
		FROM mailboxes AS mailbox
		JOIN domains AS domain
		  ON domain.id = mailbox.domain_id AND domain.tenant_id = mailbox.tenant_id
		JOIN tenants AS tenant
		  ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
		JOIN users AS account ON account.id = $2 AND account.status = 'active'
		JOIN tenant_memberships AS membership
		  ON membership.tenant_id = mailbox.tenant_id
		 AND membership.user_id = account.id
		WHERE mailbox.tenant_id = $1
		  AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
		  AND (
		    mailbox.owner_user_id = account.id OR EXISTS (
		      SELECT 1 FROM mailbox_permissions AS permission
		      WHERE permission.tenant_id = mailbox.tenant_id
		        AND permission.mailbox_id = mailbox.id
		        AND permission.user_id = account.id
		        AND permission.permission IN ('read', 'read_write', 'manage')
		    )
		  )
		ORDER BY mailbox.created_at, mailbox.id
		  LIMIT $3
	`, principal.TenantID, principal.UserID, limit)
	if err != nil {
		return nil, fmt.Errorf("list authorized mailboxes: %w", err)
	}
	defer rows.Close()

	mailboxes := make([]Mailbox, 0)
	for rows.Next() {
		var mailbox Mailbox
		if err := rows.Scan(
			&mailbox.ID,
			&mailbox.TenantID,
			&mailbox.Address,
			&mailbox.Status,
			&mailbox.CreatedAt,
		); err != nil {
			return nil, fmt.Errorf("scan mailbox row: %w", err)
		}
		mailboxes = append(mailboxes, mailbox)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate mailbox rows: %w", err)
	}
	return mailboxes, nil
}

func (repository *PostgreSQLRepository) ListMessages(
	ctx context.Context,
	principal Principal,
	mailboxID string,
	limit int,
) ([]MessageSummary, error) {
	if !validMailboxPrincipal(principal) {
		return nil, ErrForbidden
	}
	if !validUUID(mailboxID) || limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	if err := repository.authorizeMailboxRead(ctx, principal, mailboxID); err != nil {
		return nil, err
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT message.id::text, message.mailbox_id::text,
		       COALESCE(message.sender_address, ''), COALESCE(message.subject, ''),
		       message.raw_size, message.internal_date
		FROM mailboxes AS mailbox
		JOIN tenants AS tenant
		  ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
		JOIN users AS account ON account.id = $3 AND account.status = 'active'
		JOIN tenant_memberships AS membership
		  ON membership.tenant_id = mailbox.tenant_id
		 AND membership.user_id = account.id
		JOIN mailbox_messages AS message ON message.mailbox_id = mailbox.id
		WHERE mailbox.tenant_id = $1 AND mailbox.id = $2
		  AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
		  AND message.deleted_at IS NULL
		  AND (
		    mailbox.owner_user_id = account.id OR EXISTS (
		      SELECT 1 FROM mailbox_permissions AS permission
		      WHERE permission.tenant_id = mailbox.tenant_id
		        AND permission.mailbox_id = mailbox.id
		        AND permission.user_id = account.id
		        AND permission.permission IN ('read', 'read_write', 'manage')
		    )
		  )
		ORDER BY message.internal_date, message.id
		LIMIT $4
	`, principal.TenantID, mailboxID, principal.UserID, limit)
	if err != nil {
		return nil, fmt.Errorf("list authorized mailbox messages: %w", err)
	}
	defer rows.Close()

	messages := make([]MessageSummary, 0)
	for rows.Next() {
		message, err := scanMessageSummary(rows)
		if err != nil {
			return nil, err
		}
		messages = append(messages, message)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate mailbox message rows: %w", err)
	}
	return messages, nil
}

func (repository *PostgreSQLRepository) GetMessage(
	ctx context.Context,
	principal Principal,
	mailboxID string,
	messageID string,
) (MessageSummary, error) {
	if !validMailboxPrincipal(principal) {
		return MessageSummary{}, ErrForbidden
	}
	if !validUUID(mailboxID) || !validUUID(messageID) {
		return MessageSummary{}, ErrNotFound
	}
	row := repository.pool.QueryRow(ctx, `
		SELECT message.id::text, message.mailbox_id::text,
		       COALESCE(message.sender_address, ''), COALESCE(message.subject, ''),
		       message.raw_size, message.internal_date
		FROM mailboxes AS mailbox
		JOIN tenants AS tenant
		  ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
		JOIN users AS account ON account.id = $4 AND account.status = 'active'
		JOIN tenant_memberships AS membership
		  ON membership.tenant_id = mailbox.tenant_id
		 AND membership.user_id = account.id
		JOIN mailbox_messages AS message
		  ON message.mailbox_id = mailbox.id AND message.id = $3
		WHERE mailbox.tenant_id = $1 AND mailbox.id = $2
		  AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
		  AND message.deleted_at IS NULL
		  AND (
		    mailbox.owner_user_id = account.id OR EXISTS (
		      SELECT 1 FROM mailbox_permissions AS permission
		      WHERE permission.tenant_id = mailbox.tenant_id
		        AND permission.mailbox_id = mailbox.id
		        AND permission.user_id = account.id
		        AND permission.permission IN ('read', 'read_write', 'manage')
		    )
		  )
	`, principal.TenantID, mailboxID, messageID, principal.UserID)
	message, err := scanMessageSummary(row)
	if errors.Is(err, pgx.ErrNoRows) {
		return MessageSummary{}, ErrNotFound
	}
	return message, err
}

func (repository *PostgreSQLRepository) authorizeMailboxRead(
	ctx context.Context,
	principal Principal,
	mailboxID string,
) error {
	var allowed bool
	err := repository.pool.QueryRow(ctx, `
		SELECT EXISTS (
			SELECT 1
			FROM mailboxes AS mailbox
			JOIN tenants AS tenant
			  ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
			JOIN users AS account
			  ON account.id = $3 AND account.status = 'active'
			JOIN tenant_memberships AS membership
			  ON membership.tenant_id = mailbox.tenant_id
			 AND membership.user_id = account.id
			WHERE mailbox.tenant_id = $1 AND mailbox.id = $2
			  AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
			  AND (
			    mailbox.owner_user_id = account.id OR EXISTS (
			      SELECT 1 FROM mailbox_permissions AS permission
			      WHERE permission.tenant_id = mailbox.tenant_id
			        AND permission.mailbox_id = mailbox.id
			        AND permission.user_id = account.id
			        AND permission.permission IN ('read', 'read_write', 'manage')
			    )
			  )
		)
	`, principal.TenantID, mailboxID, principal.UserID).Scan(&allowed)
	if err != nil {
		return fmt.Errorf("authorize mailbox read: %w", err)
	}
	if !allowed {
		return ErrNotFound
	}
	return nil
}

func validMailboxPrincipal(principal Principal) bool {
	return principal.durable &&
		validUUID(principal.TenantID) &&
		validUUID(principal.UserID)
}

type rowScanner interface {
	Scan(dest ...any) error
}

func scanMessageSummary(row rowScanner) (MessageSummary, error) {
	var (
		message MessageSummary
		size    int64
	)
	if err := row.Scan(
		&message.ID,
		&message.MailboxID,
		&message.Sender,
		&message.Subject,
		&size,
		&message.ReceivedAt,
	); err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return MessageSummary{}, ErrNotFound
		}
		return MessageSummary{}, fmt.Errorf("scan mailbox message: %w", err)
	}
	maxInt := int64(^uint(0) >> 1)
	if size < 0 || size > maxInt {
		return MessageSummary{}, errors.New("mailbox message size is outside API range")
	}
	message.Size = int(size)
	return message, nil
}

func validUUID(value string) bool {
	if len(value) != 36 || value[8] != '-' || value[13] != '-' ||
		value[18] != '-' || value[23] != '-' {
		return false
	}
	compact := strings.ReplaceAll(value, "-", "")
	decoded := make([]byte, hex.DecodedLen(len(compact)))
	_, err := hex.Decode(decoded, []byte(compact))
	return err == nil && len(decoded) == 16
}
