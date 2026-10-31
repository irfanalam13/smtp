package controlplane

import (
	"context"
	"crypto/sha256"
	"errors"
	"fmt"
	"strings"
	"time"

	smtpAuth "example.com/smtp-platform/smtp-auth"
	"github.com/jackc/pgx/v5"
)

type AuthSession struct {
	ID            string    `json:"id"`
	UserID        string    `json:"user_id"`
	TenantID      string    `json:"tenant_id"`
	UserAgent     string    `json:"user_agent"`
	CreatedAt     time.Time `json:"created_at"`
	LastUsedAt    time.Time `json:"last_used_at"`
	IdleExpiresAt time.Time `json:"idle_expires_at"`
	ExpiresAt     time.Time `json:"expires_at"`
	Current       bool      `json:"is_current,omitempty"`
}

func (repository *PostgreSQLRepository) CreateAuthSession(
	ctx context.Context,
	principal Principal,
	tokenHash [sha256.Size]byte,
	csrfHash [sha256.Size]byte,
	idleExpiresAt time.Time,
	expiresAt time.Time,
	userAgent string,
) (AuthSession, error) {
	if !principal.durable || principal.UserID == "" || !validUUID(principal.UserID) ||
		!validUUID(principal.TenantID) || !idleExpiresAt.Before(expiresAt) {
		return AuthSession{}, ErrUnauthorized
	}
	userAgent = strings.ToValidUTF8(userAgent, "")
	if len(userAgent) > 512 {
		userAgent = userAgent[:512]
	}
	userAgent = strings.Map(func(r rune) rune {
		if r < 0x20 || r > 0x7e {
			return -1
		}
		return r
	}, userAgent)

	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return AuthSession{}, fmt.Errorf("begin browser session creation: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()

	var session AuthSession
	err = tx.QueryRow(ctx, `
		INSERT INTO auth_sessions (
			tenant_id, user_id, token_hash, csrf_hash, user_agent,
			idle_expires_at, expires_at
		)
		SELECT membership.tenant_id, account.id, $3, $4, $5, $6, $7
		FROM users AS account
		JOIN tenant_memberships AS membership
		  ON membership.user_id = account.id
		JOIN tenants AS tenant
		  ON tenant.id = membership.tenant_id
		WHERE account.id = $1 AND account.status = 'active'
		  AND membership.tenant_id = $2 AND tenant.status = 'active'
		RETURNING id::text, user_id::text, tenant_id::text, user_agent,
		          created_at, last_used_at, idle_expires_at, expires_at
	`, principal.UserID, principal.TenantID, tokenHash[:], csrfHash[:],
		userAgent, idleExpiresAt, expiresAt,
	).Scan(
		&session.ID, &session.UserID, &session.TenantID, &session.UserAgent,
		&session.CreatedAt, &session.LastUsedAt, &session.IdleExpiresAt, &session.ExpiresAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return AuthSession{}, ErrUnauthorized
	}
	if err != nil {
		return AuthSession{}, fmt.Errorf("create browser session: %w", err)
	}
	if err := insertAuthAudit(ctx, tx, principal, "session.created", session.ID); err != nil {
		return AuthSession{}, err
	}
	if err := tx.Commit(ctx); err != nil {
		return AuthSession{}, fmt.Errorf("commit browser session creation: %w", err)
	}
	return session, nil
}

func (repository *PostgreSQLRepository) AuthenticateAuthSession(
	ctx context.Context,
	secret string,
	csrfCookie string,
	csrfHeader string,
	requireCSRF bool,
	idleTTL time.Duration,
) (Principal, AuthSession, error) {
	if len(secret) < 40 || len(secret) > 128 ||
		strings.ContainsAny(secret, "\r\n\t ") || idleTTL <= 0 {
		return Principal{}, AuthSession{}, ErrUnauthorized
	}
	tokenHash := smtpAuth.HashSecret(secret)
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return Principal{}, AuthSession{}, fmt.Errorf("begin browser session authentication: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()

	var (
		principal Principal
		session   AuthSession
		csrfHash  []byte
	)
	err = tx.QueryRow(ctx, `
		SELECT auth_session.id::text, auth_session.user_id::text,
		       auth_session.tenant_id::text, auth_session.user_agent,
		       auth_session.created_at, auth_session.last_used_at,
		       auth_session.idle_expires_at, auth_session.expires_at,
		       auth_session.csrf_hash, membership.role
		FROM auth_sessions AS auth_session
		JOIN users AS account
		  ON account.id = auth_session.user_id AND account.status = 'active'
		JOIN tenant_memberships AS membership
		  ON membership.user_id = account.id
		 AND membership.tenant_id = auth_session.tenant_id
		JOIN tenants AS tenant
		  ON tenant.id = auth_session.tenant_id AND tenant.status = 'active'
		WHERE auth_session.token_hash = $1
		  AND auth_session.revoked_at IS NULL
		  AND auth_session.expires_at > NOW()
		  AND auth_session.idle_expires_at > NOW()
		FOR UPDATE OF auth_session
	`, tokenHash[:]).Scan(
		&session.ID, &session.UserID, &session.TenantID, &session.UserAgent,
		&session.CreatedAt, &session.LastUsedAt, &session.IdleExpiresAt, &session.ExpiresAt,
		&csrfHash, &principal.Role,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return Principal{}, AuthSession{}, ErrUnauthorized
	}
	if err != nil {
		return Principal{}, AuthSession{}, fmt.Errorf("look up browser session: %w", err)
	}
	if len(csrfHash) != sha256.Size ||
		(requireCSRF && !smtpAuth.VerifyCSRF(csrfCookie, csrfHeader, [sha256.Size]byte(csrfHash))) {
		return Principal{}, AuthSession{}, ErrUnauthorized
	}

	idleExpiresAt := time.Now().Add(idleTTL)
	if idleExpiresAt.After(session.ExpiresAt) {
		idleExpiresAt = session.ExpiresAt
	}
	err = tx.QueryRow(ctx, `
		UPDATE auth_sessions
		SET last_used_at = NOW(), idle_expires_at = $2
		WHERE id = $1 AND revoked_at IS NULL
		  AND expires_at > NOW() AND idle_expires_at > NOW()
		RETURNING last_used_at, idle_expires_at
	`, session.ID, idleExpiresAt).Scan(&session.LastUsedAt, &session.IdleExpiresAt)
	if errors.Is(err, pgx.ErrNoRows) {
		return Principal{}, AuthSession{}, ErrUnauthorized
	}
	if err != nil {
		return Principal{}, AuthSession{}, fmt.Errorf("refresh browser session activity: %w", err)
	}
	if err := tx.Commit(ctx); err != nil {
		return Principal{}, AuthSession{}, fmt.Errorf("complete browser session authentication: %w", err)
	}

	principal.Subject = "session:" + session.ID
	principal.UserID = session.UserID
	principal.TenantID = session.TenantID
	principal.Scopes = smtpAuth.PermissionsForRole(string(principal.Role))
	principal.durable = true
	return principal, session, nil
}

func (repository *PostgreSQLRepository) ListAuthSessions(
	ctx context.Context,
	principal Principal,
) ([]AuthSession, error) {
	if !principal.durable || principal.UserID == "" || !validUUID(principal.UserID) {
		return nil, ErrUnauthorized
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT id::text, user_id::text, tenant_id::text, user_agent,
		       created_at, last_used_at, idle_expires_at, expires_at
		FROM auth_sessions
		WHERE user_id = $1 AND revoked_at IS NULL
		  AND expires_at > NOW() AND idle_expires_at > NOW()
		ORDER BY last_used_at DESC
		LIMIT 100
	`, principal.UserID)
	if err != nil {
		return nil, fmt.Errorf("list browser sessions: %w", err)
	}
	defer rows.Close()
	sessions := make([]AuthSession, 0)
	for rows.Next() {
		var session AuthSession
		if err := rows.Scan(
			&session.ID, &session.UserID, &session.TenantID, &session.UserAgent,
			&session.CreatedAt, &session.LastUsedAt, &session.IdleExpiresAt, &session.ExpiresAt,
		); err != nil {
			return nil, fmt.Errorf("scan browser session: %w", err)
		}
		sessions = append(sessions, session)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate browser sessions: %w", err)
	}
	return sessions, nil
}

func (repository *PostgreSQLRepository) RevokeAuthSession(
	ctx context.Context,
	principal Principal,
	sessionID string,
) error {
	if !principal.durable || principal.UserID == "" || !validUUID(principal.UserID) ||
		!validUUID(sessionID) {
		return ErrNotFound
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin browser session revocation: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()
	tag, err := tx.Exec(ctx, `
		UPDATE auth_sessions
		SET revoked_at = NOW()
		WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
	`, sessionID, principal.UserID)
	if err != nil {
		return fmt.Errorf("revoke browser session: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotFound
	}
	if err := insertAuthAudit(ctx, tx, principal, "session.revoked", sessionID); err != nil {
		return err
	}
	if err := tx.Commit(ctx); err != nil {
		return fmt.Errorf("commit browser session revocation: %w", err)
	}
	return nil
}

func (repository *PostgreSQLRepository) RevokeAllAuthSessions(
	ctx context.Context,
	principal Principal,
) error {
	if !principal.durable || principal.UserID == "" || !validUUID(principal.UserID) {
		return ErrUnauthorized
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin all-session revocation: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()
	_, err = tx.Exec(ctx, `
		UPDATE auth_sessions
		SET revoked_at = NOW()
		WHERE user_id = $1 AND revoked_at IS NULL
	`, principal.UserID)
	if err != nil {
		return fmt.Errorf("revoke all browser sessions: %w", err)
	}
	if err := insertAuthAudit(ctx, tx, principal, "session.all_revoked", ""); err != nil {
		return err
	}
	if err := tx.Commit(ctx); err != nil {
		return fmt.Errorf("commit all-session revocation: %w", err)
	}
	return nil
}

func (repository *PostgreSQLRepository) RevokeAuthSessionByToken(
	ctx context.Context,
	tokenHash [sha256.Size]byte,
	principal Principal,
) error {
	if !principal.durable || principal.UserID == "" || !validUUID(principal.UserID) {
		return nil
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin current-session revocation: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()
	tag, err := tx.Exec(ctx, `
		UPDATE auth_sessions
		SET revoked_at = NOW()
		WHERE token_hash = $1 AND user_id = $2 AND revoked_at IS NULL
	`, tokenHash[:], principal.UserID)
	if err != nil {
		return fmt.Errorf("revoke current browser session: %w", err)
	}
	if tag.RowsAffected() > 0 {
		if err := insertAuthAudit(ctx, tx, principal, "session.revoked", "current"); err != nil {
			return err
		}
	}
	if err := tx.Commit(ctx); err != nil {
		return fmt.Errorf("commit current-session revocation: %w", err)
	}
	return nil
}

func insertAuthAudit(ctx context.Context, tx pgx.Tx, principal Principal, action, target string) error {
	_, err := tx.Exec(ctx, `
		INSERT INTO audit_events (tenant_id, actor_user_id, action, target_type, target_id)
		VALUES ($1, $2, $3, 'auth_session', NULLIF($4, ''))
	`, principal.TenantID, principal.UserID, action, target)
	if err != nil {
		return fmt.Errorf("record browser session audit event: %w", err)
	}
	return nil
}
