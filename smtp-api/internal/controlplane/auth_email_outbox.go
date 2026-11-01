package controlplane

import (
	"context"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha256"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"strings"
	"time"

	"example.com/smtp-platform/smtp-api/internal/authmail"
	"github.com/jackc/pgx/v5"
)

const (
	authEmailLease        = 2 * time.Minute
	authEmailMaxAttempts  = 8
	authEmailRetryCeiling = 24 * time.Hour
)

type encryptedAuthEmail struct {
	Recipient string
	Subject   string
	Body      string
}

func encryptAuthEmail(key []byte, message authmail.Message) ([]byte, error) {
	block, err := aes.NewCipher(hashEncryptionKey(key))
	if err != nil {
		return nil, err
	}
	aead, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	nonce := make([]byte, aead.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}
	plaintext := []byte(message.To + "\x00" + message.Subject + "\x00" + message.Body)
	return append(nonce, aead.Seal(nil, nonce, plaintext, nil)...), nil
}

func decryptAuthEmail(key, ciphertext []byte) (authmail.Message, error) {
	block, err := aes.NewCipher(hashEncryptionKey(key))
	if err != nil {
		return authmail.Message{}, err
	}
	aead, err := cipher.NewGCM(block)
	if err != nil || len(ciphertext) < aead.NonceSize() {
		return authmail.Message{}, errors.New("invalid encrypted auth email")
	}
	nonce, payload := ciphertext[:aead.NonceSize()], ciphertext[aead.NonceSize():]
	plaintext, err := aead.Open(nil, nonce, payload, nil)
	if err != nil {
		return authmail.Message{}, errors.New("invalid encrypted auth email")
	}
	parts := strings.SplitN(string(plaintext), "\x00", 3)
	if len(parts) != 3 || parts[0] == "" || parts[1] == "" || parts[2] == "" {
		return authmail.Message{}, errors.New("invalid encrypted auth email payload")
	}
	return authmail.Message{To: parts[0], Subject: parts[1], Body: parts[2]}, nil
}

func hashEncryptionKey(key []byte) []byte {
	sum := sha256.Sum256(key)
	return sum[:]
}

func (repository *PostgreSQLRepository) enqueueAuthEmail(
	ctx context.Context,
	tx pgx.Tx,
	encryptionKey []byte,
	deduplicationKey string,
	message authmail.Message,
) error {
	if len(encryptionKey) < 32 || len(deduplicationKey) < 16 || message.To == "" ||
		message.Subject == "" || message.Body == "" {
		return ErrInvalid
	}
	ciphertext, err := encryptAuthEmail(encryptionKey, message)
	if err != nil {
		return fmt.Errorf("encrypt authentication email: %w", err)
	}
	_, err = tx.Exec(ctx, `
		INSERT INTO auth_email_outbox
			(deduplication_key, recipient, subject, body_ciphertext)
		VALUES ($1, $2, $3, $4)
		ON CONFLICT (deduplication_key) DO NOTHING
	`, deduplicationKey, message.To, message.Subject, ciphertext)
	if err != nil {
		return fmt.Errorf("enqueue authentication email: %w", err)
	}
	return nil
}

type authEmailOutboxItem struct {
	ID         string
	Recipient  string
	Subject    string
	Ciphertext []byte
	Attempts   int
}

func (repository *PostgreSQLRepository) claimAuthEmail(ctx context.Context) (authEmailOutboxItem, error) {
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return authEmailOutboxItem{}, err
	}
	defer func() { _ = tx.Rollback(ctx) }()
	var item authEmailOutboxItem
	err = tx.QueryRow(ctx, `
		UPDATE auth_email_outbox
		SET status = 'sending', locked_until = NOW() + INTERVAL '2 minutes',
		    attempts = attempts + 1, updated_at = NOW()
		WHERE id = (
			SELECT id FROM auth_email_outbox
			WHERE (status = 'pending' AND available_at <= NOW())
			   OR (status = 'sending' AND locked_until < NOW())
			ORDER BY available_at, created_at, id
			FOR UPDATE SKIP LOCKED LIMIT 1
		)
		RETURNING id::text, recipient, subject, body_ciphertext, attempts
	`).Scan(&item.ID, &item.Recipient, &item.Subject, &item.Ciphertext, &item.Attempts)
	if errors.Is(err, pgx.ErrNoRows) {
		return authEmailOutboxItem{}, nil
	}
	if err != nil {
		return authEmailOutboxItem{}, err
	}
	if err := tx.Commit(ctx); err != nil {
		return authEmailOutboxItem{}, err
	}
	return item, nil
}

func (repository *PostgreSQLRepository) completeAuthEmail(
	ctx context.Context,
	id string,
	attempts int,
	sendErr error,
) error {
	if sendErr == nil {
		_, err := repository.pool.Exec(ctx, `
			UPDATE auth_email_outbox
			SET status = 'sent', sent_at = NOW(), locked_until = NULL,
			    last_error = NULL, updated_at = NOW()
			WHERE id = $1 AND status = 'sending'
		`, id)
		return err
	}
	if attempts < 1 {
		attempts = 1
	}
	delay := time.Second * time.Duration(1<<min(attempts, 10))
	var lastError = sendErr.Error()
	if len(lastError) > 500 {
		lastError = lastError[:500]
	}
	_, err := repository.pool.Exec(ctx, `
		UPDATE auth_email_outbox
		SET status = CASE WHEN attempts >= $2 THEN 'dead_letter' ELSE 'pending' END,
		    available_at = NOW() + ($3 * INTERVAL '1 second'),
		    locked_until = NULL, last_error = $4, updated_at = NOW()
		WHERE id = $1 AND status = 'sending'
	`, id, authEmailMaxAttempts, int64(delay/time.Second), lastError)
	return err
}

func (repository *PostgreSQLRepository) RunAuthEmailOutbox(
	ctx context.Context,
	sender authmail.Sender,
	encryptionKey []byte,
) {
	if sender == nil || len(encryptionKey) < 32 {
		return
	}
	for ctx.Err() == nil {
		item, err := repository.claimAuthEmail(ctx)
		if err != nil {
			slog.Error("claim authentication email outbox item", "error", err)
			if !sleepContext(ctx, 5*time.Second) {
				return
			}
			continue
		}
		if item.ID == "" {
			if !sleepContext(ctx, time.Second) {
				return
			}
			continue
		}
		message, err := decryptAuthEmail(encryptionKey, item.Ciphertext)
		if err == nil {
			sendCtx, cancel := context.WithTimeout(ctx, 20*time.Second)
			err = sender.Send(sendCtx, message)
			cancel()
		}
		if completeErr := repository.completeAuthEmail(ctx, item.ID, item.Attempts, err); completeErr != nil {
			slog.Error("complete authentication email outbox item", "error", completeErr)
		}
	}
}

func sleepContext(ctx context.Context, duration time.Duration) bool {
	timer := time.NewTimer(duration)
	defer timer.Stop()
	select {
	case <-ctx.Done():
		return false
	case <-timer.C:
		return true
	}
}
