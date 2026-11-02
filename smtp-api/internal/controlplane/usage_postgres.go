package controlplane

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/jackc/pgx/v5"
)

const (
	defaultUsageWindowSeconds   = 3600
	defaultUsageMessageLimit    = 1000
	defaultUsageMaxMessageBytes = 10 * 1024 * 1024 // 10 MiB
)

func (repository *PostgreSQLRepository) Usage(ctx context.Context, principal Principal) (Usage, error) {
	if !validUUID(principal.TenantID) {
		return Usage{}, ErrInvalid
	}

	now := time.Now().UTC()
	var (
		windowStarted   time.Time
		windowSeconds   int
		messagesUsed    int
		messageLimit    int
		maxMessageBytes int
	)

	err := repository.pool.QueryRow(ctx, `
		SELECT window_started_at, window_seconds, messages_used, message_limit, max_message_bytes
		FROM tenant_usage_windows
		WHERE tenant_id = $1
	`, principal.TenantID).Scan(&windowStarted, &windowSeconds, &messagesUsed, &messageLimit, &maxMessageBytes)

	if errors.Is(err, pgx.ErrNoRows) {
		// Initialize the window for this tenant
		windowStarted = now
		windowSeconds = defaultUsageWindowSeconds
		messagesUsed = 0
		messageLimit = defaultUsageMessageLimit
		maxMessageBytes = defaultUsageMaxMessageBytes

		_, insertErr := repository.pool.Exec(ctx, `
			INSERT INTO tenant_usage_windows
				(tenant_id, window_started_at, window_seconds, messages_used, message_limit, max_message_bytes)
			VALUES ($1, $2, $3, $4, $5, $6)
			ON CONFLICT (tenant_id) DO NOTHING
		`, principal.TenantID, windowStarted, windowSeconds, messagesUsed, messageLimit, maxMessageBytes)
		if insertErr != nil {
			return Usage{}, fmt.Errorf("initialize tenant usage window: %w", insertErr)
		}

		return Usage{
			WindowStarted:   windowStarted,
			WindowSeconds:   windowSeconds,
			MessagesUsed:    messagesUsed,
			MessageLimit:    messageLimit,
			MaxMessageBytes: maxMessageBytes,
		}, nil
	}
	if err != nil {
		return Usage{}, fmt.Errorf("query tenant usage window: %w", err)
	}

	// Check if window has elapsed
	windowDuration := time.Duration(windowSeconds) * time.Second
	if now.Sub(windowStarted) >= windowDuration {
		windowStarted = now
		messagesUsed = 0
		_, updateErr := repository.pool.Exec(ctx, `
			UPDATE tenant_usage_windows
			SET window_started_at = $1, messages_used = 0, updated_at = NOW()
			WHERE tenant_id = $2
		`, windowStarted, principal.TenantID)
		if updateErr != nil {
			return Usage{}, fmt.Errorf("reset expired tenant usage window: %w", updateErr)
		}
	}

	return Usage{
		WindowStarted:   windowStarted,
		WindowSeconds:   windowSeconds,
		MessagesUsed:    messagesUsed,
		MessageLimit:    messageLimit,
		MaxMessageBytes: maxMessageBytes,
	}, nil
}

func (repository *PostgreSQLRepository) ConsumeUsageTx(
	ctx context.Context,
	tx pgx.Tx,
	tenantID string,
	messageBytes int,
) error {
	if !validUUID(tenantID) {
		return ErrInvalid
	}

	now := time.Now().UTC()
	var (
		windowStarted   time.Time
		windowSeconds   int
		messagesUsed    int
		messageLimit    int
		maxMessageBytes int
	)

	err := tx.QueryRow(ctx, `
		SELECT window_started_at, window_seconds, messages_used, message_limit, max_message_bytes
		FROM tenant_usage_windows
		WHERE tenant_id = $1
		FOR UPDATE
	`, tenantID).Scan(&windowStarted, &windowSeconds, &messagesUsed, &messageLimit, &maxMessageBytes)

	if errors.Is(err, pgx.ErrNoRows) {
		// New window: 1 message used
		if messageBytes > defaultUsageMaxMessageBytes {
			return ErrInvalid
		}
		_, insertErr := tx.Exec(ctx, `
			INSERT INTO tenant_usage_windows
				(tenant_id, window_started_at, window_seconds, messages_used, message_limit, max_message_bytes)
			VALUES ($1, $2, $3, 1, $4, $5)
		`, tenantID, now, defaultUsageWindowSeconds, defaultUsageMessageLimit, defaultUsageMaxMessageBytes)
		if insertErr != nil {
			return fmt.Errorf("insert initial usage window: %w", insertErr)
		}
		return nil
	}
	if err != nil {
		return fmt.Errorf("lock tenant usage window: %w", err)
	}

	windowDuration := time.Duration(windowSeconds) * time.Second
	if now.Sub(windowStarted) >= windowDuration {
		// Window elapsed, start fresh window with 1 used
		if messageBytes > maxMessageBytes {
			return ErrInvalid
		}
		_, updateErr := tx.Exec(ctx, `
			UPDATE tenant_usage_windows
			SET window_started_at = $1, messages_used = 1, updated_at = NOW()
			WHERE tenant_id = $2
		`, now, tenantID)
		if updateErr != nil {
			return fmt.Errorf("reset usage window: %w", updateErr)
		}
		return nil
	}

	// Window active: check limits
	if messagesUsed >= messageLimit {
		return ErrRateLimited
	}
	if messageBytes > maxMessageBytes {
		return ErrInvalid
	}

	_, updateErr := tx.Exec(ctx, `
		UPDATE tenant_usage_windows
		SET messages_used = messages_used + 1, updated_at = NOW()
		WHERE tenant_id = $1
	`, tenantID)
	if updateErr != nil {
		return fmt.Errorf("increment tenant usage: %w", updateErr)
	}

	return nil
}
