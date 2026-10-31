package controlplane

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"errors"
	"fmt"
	"net/http"
	"regexp"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
	"golang.org/x/crypto/argon2"
)

const (
	smtpCredentialPasswordBytes = 32
	smtpCredentialHashMemory    = 19_456
	smtpCredentialHashTime      = 2
	smtpCredentialHashThreads   = 1
	smtpCredentialHashBytes     = 32
	smtpCredentialSaltBytes     = 16
	smtpCredentialHashParallel  = 4
)

var smtpCredentialUsernamePattern = regexp.MustCompile(`^[a-zA-Z0-9._+@-]{1,128}$`)
var smtpCredentialHashSlots = make(chan struct{}, smtpCredentialHashParallel)

type SMTPCredential struct {
	ID        string     `json:"id"`
	Username  string     `json:"username"`
	ExpiresAt *time.Time `json:"expires_at,omitempty"`
	RevokedAt *time.Time `json:"revoked_at,omitempty"`
	CreatedAt time.Time  `json:"created_at"`
}

func (h Handler) smtpCredentialPrincipal(w http.ResponseWriter, r *http.Request, scope string) (Principal, bool) {
	p, ok := h.principal(w, r, scope)
	if !ok {
		return Principal{}, false
	}
	if h.Postgres == nil || !p.durable {
		Error(w, http.StatusUnauthorized, "unauthorized", "SMTP credentials require a durable authenticated tenant")
		return Principal{}, false
	}
	return p, true
}

func (h Handler) smtpCredentials(w http.ResponseWriter, r *http.Request) {
	p, ok := h.smtpCredentialPrincipal(w, r, "smtp:credentials:read")
	if !ok {
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	items, err := h.Postgres.ListSMTPCredentials(r.Context(), p, limit)
	if err != nil {
		h.databaseError(w, "list SMTP credentials", err)
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": items})
}

func (h Handler) createSMTPCredential(w http.ResponseWriter, r *http.Request) {
	p, ok := h.smtpCredentialPrincipal(w, r, "smtp:credentials:write")
	if !ok {
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 8192)
	var req struct {
		Username  string     `json:"username"`
		ExpiresAt *time.Time `json:"expires_at"`
	}
	if err := decodeJSON(r, &req); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "invalid JSON request")
		return
	}
	credential, password, err := h.Postgres.CreateSMTPCredential(
		r.Context(), p, req.Username, credentialExpiry(req.ExpiresAt),
	)
	if h.writeSMTPCredentialError(w, err) {
		return
	}
	JSON(w, http.StatusCreated, smtpCredentialSecretResponse(p.TenantID, credential, password))
}

func (h Handler) rotateSMTPCredential(w http.ResponseWriter, r *http.Request) {
	p, ok := h.smtpCredentialPrincipal(w, r, "smtp:credentials:write")
	if !ok {
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 8192)
	var req struct {
		ExpiresAt *time.Time `json:"expires_at"`
	}
	if err := decodeJSON(r, &req); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "invalid JSON request")
		return
	}
	credential, password, err := h.Postgres.RotateSMTPCredential(
		r.Context(), p, r.PathValue("id"), credentialExpiry(req.ExpiresAt),
	)
	if h.writeSMTPCredentialError(w, err) {
		return
	}
	JSON(w, http.StatusOK, smtpCredentialSecretResponse(p.TenantID, credential, password))
}

func (h Handler) revokeSMTPCredential(w http.ResponseWriter, r *http.Request) {
	p, ok := h.smtpCredentialPrincipal(w, r, "smtp:credentials:write")
	if !ok {
		return
	}
	err := h.Postgres.RevokeSMTPCredential(r.Context(), p, r.PathValue("id"))
	if h.writeSMTPCredentialError(w, err) {
		return
	}
	JSON(w, http.StatusOK, map[string]string{"status": "revoked"})
}

func smtpCredentialSecretResponse(tenantID string, credential SMTPCredential, password string) map[string]any {
	return map[string]any{
		"id":         credential.ID,
		"username":   credential.Username,
		"authcid":    tenantID + ":" + credential.Username,
		"password":   password,
		"expires_at": credential.ExpiresAt,
		"warning":    "The password is shown once. Store it securely; it cannot be retrieved again.",
	}
}

func credentialExpiry(requested *time.Time) *time.Time {
	if requested != nil {
		return requested
	}
	expiresAt := time.Now().UTC().AddDate(0, 3, 0)
	return &expiresAt
}

func (h Handler) writeSMTPCredentialError(w http.ResponseWriter, err error) bool {
	switch {
	case err == nil:
		return false
	case errors.Is(err, ErrInvalid):
		Error(w, http.StatusBadRequest, "invalid_request", "invalid SMTP credential request")
	case errors.Is(err, ErrForbidden):
		Error(w, http.StatusForbidden, "forbidden", "permission denied")
	case errors.Is(err, ErrNotFound):
		Error(w, http.StatusNotFound, "not_found", "SMTP credential not found")
	case errors.Is(err, ErrConflict):
		Error(w, http.StatusConflict, "conflict", "an SMTP credential with this username already exists")
	case errors.Is(err, ErrRateLimited):
		w.Header().Set("Retry-After", "1")
		Error(w, http.StatusTooManyRequests, "rate_limited", "credential hashing capacity is temporarily exhausted")
	default:
		h.databaseError(w, "manage SMTP credential", err)
	}
	return true
}

func (repository *PostgreSQLRepository) CreateSMTPCredential(
	ctx context.Context,
	principal Principal,
	username string,
	expiresAt *time.Time,
) (SMTPCredential, string, error) {
	if err := authorizeSMTPCredentialManagement(principal); err != nil {
		return SMTPCredential{}, "", err
	}
	username, err := normalizeSMTPCredentialUsername(username)
	if err != nil {
		return SMTPCredential{}, "", err
	}
	if err := validateSMTPCredentialExpiry(expiresAt); err != nil {
		return SMTPCredential{}, "", err
	}
	password, err := generateSMTPCredentialPassword()
	if err != nil {
		return SMTPCredential{}, "", err
	}
	passwordHash, err := hashSMTPCredentialPasswordBounded(password)
	if err != nil {
		return SMTPCredential{}, "", err
	}
	credential, err := repository.insertSMTPCredential(ctx, principal, username, passwordHash, expiresAt)
	if err != nil {
		return SMTPCredential{}, "", err
	}
	return credential, password, nil
}

func (repository *PostgreSQLRepository) RotateSMTPCredential(
	ctx context.Context,
	principal Principal,
	credentialID string,
	expiresAt *time.Time,
) (SMTPCredential, string, error) {
	if err := authorizeSMTPCredentialManagement(principal); err != nil {
		return SMTPCredential{}, "", err
	}
	if !validUUID(credentialID) {
		return SMTPCredential{}, "", ErrNotFound
	}
	if err := validateSMTPCredentialExpiry(expiresAt); err != nil {
		return SMTPCredential{}, "", err
	}
	password, err := generateSMTPCredentialPassword()
	if err != nil {
		return SMTPCredential{}, "", err
	}
	passwordHash, err := hashSMTPCredentialPasswordBounded(password)
	if err != nil {
		return SMTPCredential{}, "", err
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return SMTPCredential{}, "", fmt.Errorf("begin SMTP credential rotation: %w", err)
	}
	defer tx.Rollback(ctx)
	var credential SMTPCredential
	err = tx.QueryRow(ctx, `
		UPDATE smtp_credentials
		SET password_hash = $3, password_hash_version = 1,
		    password_changed_at = NOW(), expires_at = $4, updated_at = NOW()
		WHERE id = $1 AND tenant_id = $2 AND revoked_at IS NULL
		  AND (expires_at IS NULL OR expires_at > NOW())
		  AND EXISTS (
		      SELECT 1 FROM users AS account
		      JOIN tenant_memberships AS membership ON membership.user_id = account.id
		      JOIN tenants AS tenant ON tenant.id = membership.tenant_id
		      WHERE account.id = $5 AND account.status = 'active'
		        AND membership.tenant_id = $2 AND tenant.status = 'active'
		  )
		RETURNING id::text, username, expires_at, revoked_at, created_at
	`, credentialID, principal.TenantID, passwordHash, expiresAt, principal.UserID).Scan(
		&credential.ID, &credential.Username, &credential.ExpiresAt,
		&credential.RevokedAt, &credential.CreatedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return SMTPCredential{}, "", ErrNotFound
	}
	if err != nil {
		return SMTPCredential{}, "", fmt.Errorf("rotate SMTP credential: %w", err)
	}
	if err := repository.recordSMTPCredentialAudit(
		ctx, tx, principal, credential.ID, "rotated",
	); err != nil {
		return SMTPCredential{}, "", err
	}
	if err := tx.Commit(ctx); err != nil {
		return SMTPCredential{}, "", fmt.Errorf("commit SMTP credential rotation: %w", err)
	}
	return credential, password, nil
}

func (repository *PostgreSQLRepository) RevokeSMTPCredential(
	ctx context.Context,
	principal Principal,
	credentialID string,
) error {
	if err := authorizeSMTPCredentialManagement(principal); err != nil {
		return err
	}
	if !validUUID(credentialID) {
		return ErrNotFound
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin SMTP credential revocation: %w", err)
	}
	defer tx.Rollback(ctx)
	var id string
	err = tx.QueryRow(ctx, `
		UPDATE smtp_credentials
		SET revoked_at = COALESCE(revoked_at, NOW()), updated_at = NOW()
		WHERE id = $1 AND tenant_id = $2 AND revoked_at IS NULL
		  AND EXISTS (
		      SELECT 1 FROM users AS account
		      JOIN tenant_memberships AS membership ON membership.user_id = account.id
		      JOIN tenants AS tenant ON tenant.id = membership.tenant_id
		      WHERE account.id = $3 AND account.status = 'active'
		        AND membership.tenant_id = $2 AND tenant.status = 'active'
		  )
		RETURNING id::text
	`, credentialID, principal.TenantID, principal.UserID).Scan(&id)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return ErrNotFound
		}
		return fmt.Errorf("revoke SMTP credential: %w", err)
	}
	if err := repository.recordSMTPCredentialAudit(
		ctx, tx, principal, id, "revoked",
	); err != nil {
		return err
	}
	if err := tx.Commit(ctx); err != nil {
		return fmt.Errorf("commit SMTP credential revocation: %w", err)
	}
	return nil
}

func (repository *PostgreSQLRepository) ListSMTPCredentials(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]SMTPCredential, error) {
	if !principal.durable || !principal.Scopes["smtp:credentials:read"] ||
		!validUUID(principal.UserID) || !validUUID(principal.TenantID) {
		return nil, ErrForbidden
	}
	if limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT id::text, username, expires_at, revoked_at, created_at
		FROM smtp_credentials
		WHERE tenant_id = $1
		  AND EXISTS (
		      SELECT 1 FROM users AS account
		      JOIN tenant_memberships AS membership ON membership.user_id = account.id
		      JOIN tenants AS tenant ON tenant.id = membership.tenant_id
		      WHERE account.id = $3 AND account.status = 'active'
		        AND membership.tenant_id = smtp_credentials.tenant_id
		        AND tenant.status = 'active'
		  )
		ORDER BY created_at DESC, id
		LIMIT $2
	`, principal.TenantID, limit, principal.UserID)
	if err != nil {
		return nil, fmt.Errorf("list SMTP credentials: %w", err)
	}
	defer rows.Close()
	credentials := make([]SMTPCredential, 0)
	for rows.Next() {
		var credential SMTPCredential
		if err := rows.Scan(
			&credential.ID, &credential.Username, &credential.ExpiresAt,
			&credential.RevokedAt, &credential.CreatedAt,
		); err != nil {
			return nil, fmt.Errorf("scan SMTP credential: %w", err)
		}
		credentials = append(credentials, credential)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate SMTP credentials: %w", err)
	}
	return credentials, nil
}

func (repository *PostgreSQLRepository) insertSMTPCredential(
	ctx context.Context,
	principal Principal,
	username string,
	passwordHash string,
	expiresAt *time.Time,
) (SMTPCredential, error) {
	if err := authorizeSMTPCredentialManagement(principal); err != nil {
		return SMTPCredential{}, err
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return SMTPCredential{}, fmt.Errorf("begin SMTP credential creation: %w", err)
	}
	defer tx.Rollback(ctx)
	var credential SMTPCredential
	err = tx.QueryRow(ctx, `
		INSERT INTO smtp_credentials
		  (id, tenant_id, user_id, username, password_hash, password_hash_version, expires_at)
		SELECT gen_random_uuid(), membership.tenant_id, membership.user_id, $3, $4, 1, $5
		FROM tenant_memberships AS membership
		JOIN users AS account ON account.id = membership.user_id AND account.status = 'active'
		JOIN tenants AS tenant ON tenant.id = membership.tenant_id AND tenant.status = 'active'
		WHERE membership.tenant_id = $1 AND membership.user_id = $2
		RETURNING id::text, username, expires_at, revoked_at, created_at
	`, principal.TenantID, principal.UserID, username, passwordHash, expiresAt).Scan(
		&credential.ID, &credential.Username, &credential.ExpiresAt,
		&credential.RevokedAt, &credential.CreatedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return SMTPCredential{}, ErrUnauthorized
	}
	if err != nil {
		var databaseError *pgconn.PgError
		if errors.As(err, &databaseError) && databaseError.Code == "23505" {
			return SMTPCredential{}, ErrConflict
		}
		return SMTPCredential{}, fmt.Errorf("create SMTP credential: %w", err)
	}
	if err := repository.recordSMTPCredentialAudit(
		ctx, tx, principal, credential.ID, "created",
	); err != nil {
		return SMTPCredential{}, err
	}
	if err := tx.Commit(ctx); err != nil {
		return SMTPCredential{}, fmt.Errorf("commit SMTP credential creation: %w", err)
	}
	return credential, nil
}

func authorizeSMTPCredentialManagement(principal Principal) error {
	if !principal.durable || !principal.Scopes["smtp:credentials:write"] ||
		!validUUID(principal.UserID) || !validUUID(principal.TenantID) {
		return ErrForbidden
	}
	return nil
}

func (repository *PostgreSQLRepository) recordSMTPCredentialAudit(
	ctx context.Context,
	tx pgx.Tx,
	principal Principal,
	credentialID string,
	action string,
) error {
	_, err := tx.Exec(ctx, `
		INSERT INTO smtp_credential_audit_events
		  (tenant_id, credential_id, actor_user_id, action)
		VALUES ($1, $2, $3, $4)
	`, principal.TenantID, credentialID, principal.UserID, action)
	if err != nil {
		return fmt.Errorf("audit SMTP credential %s: %w", action, err)
	}
	return nil
}

func normalizeSMTPCredentialUsername(username string) (string, error) {
	username = strings.ToLower(strings.TrimSpace(username))
	if !smtpCredentialUsernamePattern.MatchString(username) {
		return "", ErrInvalid
	}
	return username, nil
}

func validateSMTPCredentialExpiry(expiresAt *time.Time) error {
	if expiresAt == nil {
		return ErrInvalid
	}
	now := time.Now().UTC()
	if !expiresAt.After(now) || expiresAt.After(now.AddDate(1, 0, 0)) {
		return ErrInvalid
	}
	return nil
}

func generateSMTPCredentialPassword() (string, error) {
	raw := make([]byte, smtpCredentialPasswordBytes)
	if _, err := rand.Read(raw); err != nil {
		return "", fmt.Errorf("generate SMTP credential password: %w", err)
	}
	return base64.RawURLEncoding.EncodeToString(raw), nil
}

func hashSMTPCredentialPassword(password string) (string, error) {
	salt := make([]byte, smtpCredentialSaltBytes)
	if _, err := rand.Read(salt); err != nil {
		return "", fmt.Errorf("generate SMTP credential salt: %w", err)
	}

	hash := argon2.IDKey(
		[]byte(password), salt, smtpCredentialHashTime,
		smtpCredentialHashMemory, smtpCredentialHashThreads, smtpCredentialHashBytes,
	)
	encoded := base64.RawStdEncoding
	return fmt.Sprintf(
		"$argon2id$v=19$m=%d,t=%d,p=%d$%s$%s",
		smtpCredentialHashMemory, smtpCredentialHashTime, smtpCredentialHashThreads,
		encoded.EncodeToString(salt), encoded.EncodeToString(hash),
	), nil
}

func hashSMTPCredentialPasswordBounded(password string) (string, error) {
	select {
	case smtpCredentialHashSlots <- struct{}{}:
		defer func() { <-smtpCredentialHashSlots }()
	default:
		return "", ErrRateLimited
	}
	return hashSMTPCredentialPassword(password)
}
