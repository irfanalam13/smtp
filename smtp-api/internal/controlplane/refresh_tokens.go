package controlplane

import (
	"context"
	"errors"
	"fmt"
	"net/http"
	"strings"
	"time"

	smtpAuth "example.com/smtp-platform/smtp-auth"
	"github.com/jackc/pgx/v5"
)

type refreshTokenRequest struct {
	RefreshToken string `json:"refresh_token"`
}

func (repository *PostgreSQLRepository) CreateRefreshToken(
	ctx context.Context,
	principal Principal,
	sessionID *string,
	familyID *string,
	ttl time.Duration,
) (rawToken string, outFamilyID string, expiresAt time.Time, err error) {
	if !validUUID(principal.UserID) || !validUUID(principal.TenantID) {
		return "", "", time.Time{}, ErrInvalid
	}
	if ttl <= 0 {
		ttl = 30 * 24 * time.Hour
	}
	rawToken, tokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		return "", "", time.Time{}, err
	}
	var fID string
	if familyID != nil && *familyID != "" && validUUID(*familyID) {
		fID = *familyID
	} else {
		generated, err := newUUID()
		if err != nil {
			return "", "", time.Time{}, err
		}
		fID = generated
	}
	expiresAt = time.Now().UTC().Add(ttl)
	hashSlice := tokenHash[:]

	_, err = repository.pool.Exec(ctx, `
		INSERT INTO auth_refresh_tokens
			(family_id, tenant_id, user_id, session_id, token_hash, expires_at)
		VALUES ($1, $2, $3, $4, $5, $6)
	`, fID, principal.TenantID, principal.UserID, sessionID, hashSlice, expiresAt)
	if err != nil {
		return "", "", time.Time{}, fmt.Errorf("create refresh token: %w", err)
	}
	return rawToken, fID, expiresAt, nil
}

func (repository *PostgreSQLRepository) RotateRefreshToken(
	ctx context.Context,
	rawToken string,
	ttl time.Duration,
) (newRawToken string, newFamilyID string, principal Principal, newExpiresAt time.Time, err error) {
	rawToken = strings.TrimSpace(rawToken)
	if rawToken == "" {
		return "", "", Principal{}, time.Time{}, ErrUnauthorized
	}
	tokenHash := smtpAuth.HashSecret(rawToken)
	hashSlice := tokenHash[:]

	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return "", "", Principal{}, time.Time{}, fmt.Errorf("begin refresh token tx: %w", err)
	}
	defer func() { _ = tx.Rollback(ctx) }()

	var (
		tokenID    string
		familyID   string
		tenantID   string
		userID     string
		sessionID  *string
		expiresAt  time.Time
		consumedAt *time.Time
		reusedAt   *time.Time
	)

	err = tx.QueryRow(ctx, `
		SELECT id::text, family_id::text, tenant_id::text, user_id::text,
		       session_id::text, expires_at, consumed_at, reused_at
		FROM auth_refresh_tokens
		WHERE token_hash = $1
		FOR UPDATE
	`, hashSlice).Scan(&tokenID, &familyID, &tenantID, &userID, &sessionID, &expiresAt, &consumedAt, &reusedAt)
	if errors.Is(err, pgx.ErrNoRows) {
		return "", "", Principal{}, time.Time{}, ErrUnauthorized
	}
	if err != nil {
		return "", "", Principal{}, time.Time{}, fmt.Errorf("lookup refresh token: %w", err)
	}

	// Replay detection: if token was already consumed or family has had a reuse attempt:
	if consumedAt != nil || reusedAt != nil {
		// Invalidate the entire family!
		_, _ = tx.Exec(ctx, `
			UPDATE auth_refresh_tokens
			SET reused_at = NOW()
			WHERE family_id = $1
		`, familyID)
		if sessionID != nil && *sessionID != "" {
			_, _ = tx.Exec(ctx, `DELETE FROM auth_sessions WHERE id = $1`, *sessionID)
		}
		_ = repository.RecordAuditEvent(
			ctx, tenantID, userID, "auth.refresh_token_replay",
			"refresh_token_family", familyID, nil,
		)
		_ = tx.Commit(ctx)
		return "", "", Principal{}, time.Time{}, ErrUnauthorized
	}

	// Expiry check
	if time.Now().UTC().After(expiresAt) {
		return "", "", Principal{}, time.Time{}, ErrUnauthorized
	}

	// Mark consumed
	if _, err := tx.Exec(ctx, `
		UPDATE auth_refresh_tokens
		SET consumed_at = NOW()
		WHERE id = $1
	`, tokenID); err != nil {
		return "", "", Principal{}, time.Time{}, fmt.Errorf("consume refresh token: %w", err)
	}

	// Generate new token in the same family
	newRaw, newTokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		return "", "", Principal{}, time.Time{}, err
	}
	if ttl <= 0 {
		ttl = 30 * 24 * time.Hour
	}
	newExpiresAt = time.Now().UTC().Add(ttl)
	newHashSlice := newTokenHash[:]

	if _, err := tx.Exec(ctx, `
		INSERT INTO auth_refresh_tokens
			(family_id, tenant_id, user_id, session_id, token_hash, expires_at)
		VALUES ($1, $2, $3, $4, $5, $6)
	`, familyID, tenantID, userID, sessionID, newHashSlice, newExpiresAt); err != nil {
		return "", "", Principal{}, time.Time{}, fmt.Errorf("insert rotated refresh token: %w", err)
	}

	// Look up user role and subject
	var role Role
	var email string
	err = tx.QueryRow(ctx, `
		SELECT m.role, COALESCE(u.email, u.external_subject)
		FROM tenant_memberships m
		JOIN users u ON u.id = m.user_id AND u.status = 'active'
		JOIN tenants t ON t.id = m.tenant_id AND t.status = 'active'
		WHERE m.user_id = $1 AND m.tenant_id = $2
	`, userID, tenantID).Scan(&role, &email)
	if err != nil {
		return "", "", Principal{}, time.Time{}, ErrUnauthorized
	}

	if err := tx.Commit(ctx); err != nil {
		return "", "", Principal{}, time.Time{}, fmt.Errorf("commit refresh rotation tx: %w", err)
	}

	scopes := smtpAuth.PermissionsForRole(string(role))
	principal = Principal{
		UserID:   userID,
		TenantID: tenantID,
		Role:     role,
		Subject:  email,
		Scopes:   scopes,
		durable:  true,
	}

	_ = repository.RecordAuditEvent(
		ctx, tenantID, userID, "auth.refresh_token_rotated",
		"refresh_token_family", familyID, nil,
	)

	return newRaw, familyID, principal, newExpiresAt, nil
}

func (repository *PostgreSQLRepository) RevokeRefreshToken(ctx context.Context, rawToken string) error {
	rawToken = strings.TrimSpace(rawToken)
	if rawToken == "" {
		return ErrInvalid
	}
	tokenHash := smtpAuth.HashSecret(rawToken)
	hashSlice := tokenHash[:]

	var familyID string
	var tenantID string
	var userID string
	err := repository.pool.QueryRow(ctx, `
		SELECT family_id::text, tenant_id::text, user_id::text
		FROM auth_refresh_tokens
		WHERE token_hash = $1
	`, hashSlice).Scan(&familyID, &tenantID, &userID)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil
	}
	if err != nil {
		return fmt.Errorf("lookup token to revoke: %w", err)
	}

	_, err = repository.pool.Exec(ctx, `
		UPDATE auth_refresh_tokens
		SET consumed_at = COALESCE(consumed_at, NOW()),
		    reused_at = COALESCE(reused_at, NOW())
		WHERE family_id = $1
	`, familyID)
	if err != nil {
		return fmt.Errorf("revoke token family: %w", err)
	}

	_ = repository.RecordAuditEvent(
		ctx, tenantID, userID, "auth.refresh_token_revoked",
		"refresh_token_family", familyID, nil,
	)
	return nil
}

func (h Handler) refreshTokens(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "database is not configured")
		return
	}
	if !h.allow(r.Context(), "refresh-token:"+requestClientIP(r), 30, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many token refresh attempts")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var req refreshTokenRequest
	_ = decodeJSON(r, &req)

	tokenToRotate := strings.TrimSpace(req.RefreshToken)
	if tokenToRotate == "" {
		// Check Authorization Bearer header
		authHeader := r.Header.Get("Authorization")
		if strings.HasPrefix(authHeader, "Bearer ") {
			tokenToRotate = strings.TrimSpace(strings.TrimPrefix(authHeader, "Bearer "))
		}
	}

	if tokenToRotate == "" {
		Error(w, http.StatusBadRequest, "invalid_request", "refresh_token is required")
		return
	}

	newRefreshToken, _, principal, refreshExpiresAt, err := h.Postgres.RotateRefreshToken(
		r.Context(), tokenToRotate, 30*24*time.Hour,
	)
	if err != nil {
		if errors.Is(err, ErrUnauthorized) {
			Error(w, http.StatusUnauthorized, "unauthorized", "refresh token is invalid, expired, or already consumed")
			return
		}
		h.databaseError(w, "rotate refresh token", err)
		return
	}

	response := map[string]any{
		"token_type":               "Bearer",
		"refresh_token":            newRefreshToken,
		"refresh_token_expires_at": refreshExpiresAt,
	}

	if h.JWT.valid() {
		accessToken, accessExpiresAt, err := h.JWT.Issue(principal)
		if err != nil {
			h.databaseError(w, "issue JWT access token", err)
			return
		}
		response["access_token"] = accessToken
		response["access_token_expires_at"] = accessExpiresAt
	}

	JSON(w, http.StatusOK, response)
}

func (h Handler) revokeRefreshToken(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "database is not configured")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var req refreshTokenRequest
	_ = decodeJSON(r, &req)
	token := strings.TrimSpace(req.RefreshToken)
	if token == "" {
		authHeader := r.Header.Get("Authorization")
		if strings.HasPrefix(authHeader, "Bearer ") {
			token = strings.TrimSpace(strings.TrimPrefix(authHeader, "Bearer "))
		}
	}
	if token != "" {
		_ = h.Postgres.RevokeRefreshToken(r.Context(), token)
	}
	JSON(w, http.StatusOK, map[string]string{"status": "revoked"})
}
