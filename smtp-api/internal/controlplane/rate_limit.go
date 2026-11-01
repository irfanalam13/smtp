package controlplane

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"errors"
	"fmt"
	"time"
)

func (h Handler) allow(ctx context.Context, key string, limit int, period time.Duration) bool {
	if limit <= 0 || period <= 0 {
		return false
	}
	if h.DevMode && h.Store != nil {
		return h.Store.Allow(key, limit, period)
	}
	if h.Postgres != nil {
		if len(h.RateLimitHMACKey) < 32 {
			return false
		}
		allowed, err := h.Postgres.AllowRateLimit(ctx, h.RateLimitHMACKey, key, limit, period)
		return err == nil && allowed
	}
	return false
}

func rateLimitHash(key []byte, dimension string) ([]byte, error) {
	if len(key) < 32 {
		return nil, errors.New("rate-limit HMAC key is too short")
	}
	mac := hmac.New(sha256.New, key)
	_, _ = mac.Write([]byte("smtp-platform/api-rate/v1\x00"))
	_, _ = mac.Write([]byte(dimension))
	return mac.Sum(nil), nil
}

func (repository *PostgreSQLRepository) AllowRateLimit(
	ctx context.Context,
	hmacKey []byte,
	key string,
	limit int,
	period time.Duration,
) (bool, error) {
	if repository == nil || limit <= 0 || period <= 0 || period > 24*time.Hour {
		return false, errors.New("invalid rate-limit configuration")
	}
	hash, err := rateLimitHash(hmacKey, key)
	if err != nil {
		return false, err
	}
	seconds := int64(period / time.Second)
	if seconds < 1 {
		return false, errors.New("rate-limit period is too short")
	}
	var count int
	err = repository.pool.QueryRow(ctx, `
		INSERT INTO smtp_auth_rate_limits
			(key_hash, dimension, failures, window_started_at, updated_at)
		VALUES ($1, 'ip', 1, NOW(), NOW())
		ON CONFLICT (key_hash) DO UPDATE SET
			failures = CASE
				WHEN smtp_auth_rate_limits.window_started_at <= NOW() - ($2 * INTERVAL '1 second')
				THEN 1
				ELSE smtp_auth_rate_limits.failures + 1
			END,
			window_started_at = CASE
				WHEN smtp_auth_rate_limits.window_started_at <= NOW() - ($2 * INTERVAL '1 second')
				THEN NOW()
				ELSE smtp_auth_rate_limits.window_started_at
			END,
			updated_at = NOW()
		RETURNING failures
	`, hash, seconds).Scan(&count)
	if err != nil {
		return false, fmt.Errorf("update shared API rate limit: %w", err)
	}
	return count <= limit, nil
}
