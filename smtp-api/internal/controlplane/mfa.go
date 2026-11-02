package controlplane

import (
	"context"
	"crypto/sha256"
	"errors"
	"fmt"
	"net/http"
	"strings"
	"time"

	smtpAuth "example.com/smtp-platform/smtp-auth"
	"github.com/jackc/pgx/v5"
)

type mfaConfirmRequest struct {
	Code string `json:"code"`
}

type mfaDisableRequest struct {
	CurrentPassword string `json:"current_password"`
}

type mfaChallengeVerifyRequest struct {
	ChallengeToken string `json:"challenge_token"`
	Code           string `json:"code"`
	RecoveryCode   string `json:"recovery_code"`
}

func (h Handler) encryptionKey() []byte {
	if len(h.AuthEmailKey) >= 16 {
		return h.AuthEmailKey
	}
	if len(h.RateLimitHMACKey) >= 16 {
		return h.RateLimitHMACKey
	}
	return []byte("default-development-encryption-key-32b!")
}

func (repository *PostgreSQLRepository) SetupTOTP(ctx context.Context, userID string, encryptedSecret []byte) error {
	if !validUUID(userID) || len(encryptedSecret) == 0 {
		return ErrInvalid
	}
	result, err := repository.pool.Exec(ctx, `
		UPDATE users
		SET totp_secret_encrypted = $2, totp_confirmed_at = NULL
		WHERE id = $1 AND status = 'active'
	`, userID, encryptedSecret)
	if err != nil {
		return fmt.Errorf("setup TOTP: %w", err)
	}
	if result.RowsAffected() == 0 {
		return ErrNotFound
	}
	return nil
}

func (repository *PostgreSQLRepository) ConfirmTOTP(ctx context.Context, userID string, recoveryCodeHashes [][sha256.Size]byte) error {
	if !validUUID(userID) || len(recoveryCodeHashes) == 0 {
		return ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin confirm TOTP tx: %w", err)
	}
	defer func() { _ = tx.Rollback(ctx) }()

	result, err := tx.Exec(ctx, `
		UPDATE users
		SET totp_confirmed_at = NOW()
		WHERE id = $1 AND status = 'active' AND totp_secret_encrypted IS NOT NULL
	`, userID)
	if err != nil {
		return fmt.Errorf("confirm TOTP secret: %w", err)
	}
	if result.RowsAffected() == 0 {
		return ErrNotFound
	}

	// Remove old recovery codes if any
	if _, err := tx.Exec(ctx, `DELETE FROM auth_mfa_recovery_codes WHERE user_id = $1`, userID); err != nil {
		return fmt.Errorf("clean old recovery codes: %w", err)
	}

	for _, hash := range recoveryCodeHashes {
		hashCopy := hash
		if _, err := tx.Exec(ctx, `
			INSERT INTO auth_mfa_recovery_codes (user_id, code_hash)
			VALUES ($1, $2)
		`, userID, hashCopy[:]); err != nil {
			return fmt.Errorf("store recovery code hash: %w", err)
		}
	}

	return tx.Commit(ctx)
}

func (repository *PostgreSQLRepository) DisableTOTP(ctx context.Context, userID string) error {
	if !validUUID(userID) {
		return ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin disable TOTP tx: %w", err)
	}
	defer func() { _ = tx.Rollback(ctx) }()

	if _, err := tx.Exec(ctx, `
		UPDATE users
		SET totp_secret_encrypted = NULL, totp_confirmed_at = NULL
		WHERE id = $1
	`, userID); err != nil {
		return fmt.Errorf("clear user TOTP: %w", err)
	}

	if _, err := tx.Exec(ctx, `DELETE FROM auth_mfa_recovery_codes WHERE user_id = $1`, userID); err != nil {
		return fmt.Errorf("delete recovery codes: %w", err)
	}

	return tx.Commit(ctx)
}

func (repository *PostgreSQLRepository) GetEncryptedTOTPSecret(ctx context.Context, userID string) ([]byte, bool, error) {
	if !validUUID(userID) {
		return nil, false, ErrInvalid
	}
	var secret []byte
	var confirmed bool
	err := repository.pool.QueryRow(ctx, `
		SELECT totp_secret_encrypted, totp_confirmed_at IS NOT NULL
		FROM users
		WHERE id = $1 AND status = 'active'
	`, userID).Scan(&secret, &confirmed)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, false, ErrNotFound
	}
	if err != nil {
		return nil, false, fmt.Errorf("get TOTP secret: %w", err)
	}
	return secret, confirmed, nil
}

func (repository *PostgreSQLRepository) CreateMFAChallenge(
	ctx context.Context,
	userID, tenantID, purpose string,
	ttl time.Duration,
) (string, error) {
	if !validUUID(userID) || !validUUID(tenantID) || purpose == "" {
		return "", ErrInvalid
	}
	rawToken, tokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		return "", err
	}
	hashSlice := tokenHash[:]
	expiresAt := time.Now().UTC().Add(ttl)
	_, err = repository.pool.Exec(ctx, `
		INSERT INTO auth_mfa_challenges (user_id, tenant_id, token_hash, purpose, expires_at)
		VALUES ($1, $2, $3, $4, $5)
	`, userID, tenantID, hashSlice, purpose, expiresAt)
	if err != nil {
		return "", fmt.Errorf("create MFA challenge: %w", err)
	}
	return rawToken, nil
}

func (repository *PostgreSQLRepository) ConsumeMFAChallenge(
	ctx context.Context,
	rawToken string,
) (userID string, tenantID string, purpose string, err error) {
	rawToken = strings.TrimSpace(rawToken)
	if rawToken == "" {
		return "", "", "", ErrUnauthorized
	}
	tokenHash := smtpAuth.HashSecret(rawToken)
	hashSlice := tokenHash[:]
	err = repository.pool.QueryRow(ctx, `
		UPDATE auth_mfa_challenges
		SET consumed_at = NOW()
		WHERE token_hash = $1 AND consumed_at IS NULL AND expires_at > NOW()
		RETURNING user_id::text, tenant_id::text, purpose
	`, hashSlice).Scan(&userID, &tenantID, &purpose)
	if errors.Is(err, pgx.ErrNoRows) {
		return "", "", "", ErrUnauthorized
	}
	if err != nil {
		return "", "", "", fmt.Errorf("consume MFA challenge: %w", err)
	}
	return userID, tenantID, purpose, nil
}

func (repository *PostgreSQLRepository) VerifyAndConsumeRecoveryCode(
	ctx context.Context,
	userID string,
	recoveryCode string,
) (bool, error) {
	if !validUUID(userID) {
		return false, ErrInvalid
	}
	normalized := smtpAuth.NormalizeRecoveryCode(recoveryCode)
	if len(normalized) != 10 {
		return false, nil
	}
	codeHash := smtpAuth.HashSecret(normalized)
	hashSlice := codeHash[:]
	result, err := repository.pool.Exec(ctx, `
		UPDATE auth_mfa_recovery_codes
		SET used_at = NOW()
		WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL
	`, userID, hashSlice)
	if err != nil {
		return false, fmt.Errorf("verify recovery code: %w", err)
	}
	return result.RowsAffected() == 1, nil
}

func (h Handler) setupMFA(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "database is not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok || !principal.durable {
		return
	}
	secretBase32, rawSecret, err := smtpAuth.NewTOTPSecret()
	if err != nil {
		h.databaseError(w, "generate TOTP secret", err)
		return
	}
	encrypted, err := smtpAuth.EncryptSecret(h.encryptionKey(), rawSecret)
	if err != nil {
		h.databaseError(w, "encrypt TOTP secret", err)
		return
	}
	if err := h.Postgres.SetupTOTP(r.Context(), principal.UserID, encrypted); err != nil {
		h.databaseError(w, "store TOTP secret", err)
		return
	}
	accountName := principal.Subject
	if accountName == "" {
		accountName = "user"
	}
	JSON(w, http.StatusOK, map[string]any{
		"secret": secretBase32,
		"uri":    smtpAuth.TOTPURI("SMTP Platform", accountName, secretBase32),
	})
}

func (h Handler) confirmMFA(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "database is not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok || !principal.durable {
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var req mfaConfirmRequest
	if err := decodeJSON(r, &req); err != nil || len(strings.TrimSpace(req.Code)) != 6 {
		Error(w, http.StatusBadRequest, "invalid_request", "a 6-digit TOTP code is required")
		return
	}
	encrypted, _, err := h.Postgres.GetEncryptedTOTPSecret(r.Context(), principal.UserID)
	if err != nil {
		h.databaseError(w, "fetch TOTP secret", err)
		return
	}
	if len(encrypted) == 0 {
		Error(w, http.StatusBadRequest, "mfa_not_started", "MFA setup has not been initiated")
		return
	}
	rawSecret, err := smtpAuth.DecryptSecret(h.encryptionKey(), encrypted)
	if err != nil {
		h.databaseError(w, "decrypt TOTP secret", err)
		return
	}
	if !smtpAuth.VerifyTOTP(rawSecret, req.Code, time.Now()) {
		Error(w, http.StatusBadRequest, "invalid_totp_code", "the provided code is invalid or expired")
		return
	}
	recoveryCodes, hashes, err := smtpAuth.NewRecoveryCodes()
	if err != nil {
		h.databaseError(w, "generate recovery codes", err)
		return
	}
	if err := h.Postgres.ConfirmTOTP(r.Context(), principal.UserID, hashes); err != nil {
		h.databaseError(w, "confirm TOTP", err)
		return
	}
	_ = h.Postgres.RecordAuditEvent(
		r.Context(), principal.TenantID, principal.UserID, "auth.mfa_enrolled",
		"user", principal.UserID, map[string]any{"type": "totp"},
	)
	JSON(w, http.StatusOK, map[string]any{
		"status":         "confirmed",
		"recovery_codes": recoveryCodes,
	})
}

func (h Handler) disableMFA(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "database is not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok || !principal.durable {
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var req mfaDisableRequest
	if err := decodeJSON(r, &req); err != nil || req.CurrentPassword == "" {
		Error(w, http.StatusBadRequest, "invalid_request", "current password is required")
		return
	}
	hash, err := h.Postgres.PasswordHashForUser(r.Context(), principal.UserID)
	if err != nil {
		h.databaseError(w, "fetch user password", err)
		return
	}
	valid, err := smtpAuth.VerifyPassword(hash, req.CurrentPassword)
	if err != nil || !valid {
		Error(w, http.StatusUnauthorized, "unauthorized", "password is incorrect")
		return
	}
	if err := h.Postgres.DisableTOTP(r.Context(), principal.UserID); err != nil {
		h.databaseError(w, "disable TOTP", err)
		return
	}
	_ = h.Postgres.RecordAuditEvent(
		r.Context(), principal.TenantID, principal.UserID, "auth.mfa_disabled",
		"user", principal.UserID, nil,
	)
	JSON(w, http.StatusOK, map[string]string{"status": "disabled"})
}

func (h Handler) verifyMFAChallenge(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "database is not configured")
		return
	}
	if !h.allow(r.Context(), "mfa-challenge:"+requestClientIP(r), 10, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many verification attempts")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var req mfaChallengeVerifyRequest
	if err := decodeJSON(r, &req); err != nil || req.ChallengeToken == "" {
		Error(w, http.StatusBadRequest, "invalid_request", "challenge_token is required")
		return
	}
	userID, tenantID, purpose, err := h.Postgres.ConsumeMFAChallenge(r.Context(), req.ChallengeToken)
	if err != nil {
		Error(w, http.StatusUnauthorized, "unauthorized", "challenge is invalid, expired, or already used")
		return
	}
	if purpose != "login" {
		Error(w, http.StatusBadRequest, "invalid_challenge_purpose", "challenge purpose mismatch")
		return
	}

	var verified bool
	if strings.TrimSpace(req.Code) != "" {
		encrypted, confirmed, err := h.Postgres.GetEncryptedTOTPSecret(r.Context(), userID)
		if err != nil || !confirmed || len(encrypted) == 0 {
			Error(w, http.StatusUnauthorized, "unauthorized", "TOTP is not configured for this user")
			return
		}
		rawSecret, err := smtpAuth.DecryptSecret(h.encryptionKey(), encrypted)
		if err != nil {
			h.databaseError(w, "decrypt TOTP secret", err)
			return
		}
		verified = smtpAuth.VerifyTOTP(rawSecret, req.Code, time.Now())
	} else if strings.TrimSpace(req.RecoveryCode) != "" {
		match, err := h.Postgres.VerifyAndConsumeRecoveryCode(r.Context(), userID, req.RecoveryCode)
		if err != nil {
			h.databaseError(w, "verify recovery code", err)
			return
		}
		verified = match
	} else {
		Error(w, http.StatusBadRequest, "invalid_request", "code or recovery_code is required")
		return
	}

	if !verified {
		Error(w, http.StatusUnauthorized, "unauthorized", "the provided verification code or recovery code is invalid")
		return
	}

	// Resolve principal permissions for (userID, tenantID)
	principal, err := h.resolvePrincipalForUserTenant(r.Context(), userID, tenantID)
	if err != nil {
		h.databaseError(w, "resolve user membership", err)
		return
	}

	session, err := h.issueBrowserSession(w, r, principal)
	if err != nil {
		h.databaseError(w, "create session after MFA", err)
		return
	}

	response := map[string]any{
		"status":     "authenticated",
		"user_id":    principal.UserID,
		"tenant_id":  principal.TenantID,
		"expires_at": session.ExpiresAt,
	}

	if h.JWT.valid() {
		token, expiresAt, err := h.JWT.Issue(principal)
		if err != nil {
			h.databaseError(w, "issue JWT access token", err)
			return
		}
		response["access_token"] = token
		response["token_type"] = "Bearer"
		response["access_token_expires_at"] = expiresAt

		sessionIDStr := session.ID
		rawRefresh, _, refreshExpiresAt, err := h.Postgres.CreateRefreshToken(
			r.Context(), principal, &sessionIDStr, nil, 30*24*time.Hour,
		)
		if err == nil {
			response["refresh_token"] = rawRefresh
			response["refresh_token_expires_at"] = refreshExpiresAt
		}
	}

	_ = h.Postgres.RecordAuditEvent(
		r.Context(), principal.TenantID, principal.UserID, "auth.login_mfa_success",
		"user", principal.UserID, nil,
	)

	JSON(w, http.StatusOK, response)
}

func (h Handler) resolvePrincipalForUserTenant(ctx context.Context, userID, tenantID string) (Principal, error) {
	var role Role
	var email string
	err := h.Postgres.pool.QueryRow(ctx, `
		SELECT m.role, COALESCE(u.email, u.external_subject)
		FROM tenant_memberships m
		JOIN users u ON u.id = m.user_id AND u.status = 'active'
		JOIN tenants t ON t.id = m.tenant_id AND t.status = 'active'
		WHERE m.user_id = $1 AND m.tenant_id = $2
	`, userID, tenantID).Scan(&role, &email)
	if err != nil {
		return Principal{}, err
	}
	scopes := smtpAuth.PermissionsForRole(string(role))
	return Principal{
		UserID:   userID,
		TenantID: tenantID,
		Role:     role,
		Subject:  email,
		Scopes:   scopes,
		durable:  true,
	}, nil
}
