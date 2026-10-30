package controlplane

import (
	"context"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"fmt"
	"strings"

	"github.com/jackc/pgx/v5"
)

func (repository *PostgreSQLRepository) AuthenticateAPIKey(
	ctx context.Context,
	secret string,
) (Principal, error) {
	if !validAPIKey(secret) {
		return Principal{}, ErrUnauthorized
	}
	prefix := secret[:apiKeyPrefixLength]
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return Principal{}, fmt.Errorf("begin API key authentication: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()
	var (
		keyID        string
		tenantID     string
		principalTyp string
		userID       string
		role         string
		scopes       []string
		storedHash   []byte
	)
	err = tx.QueryRow(ctx, `
		SELECT key.id::text, key.tenant_id::text, key.principal_type,
		       COALESCE(key.created_by_user_id::text, ''), key.scopes, key.secret_hash
		FROM api_keys AS key
		JOIN tenants AS tenant ON tenant.id = key.tenant_id AND tenant.status = 'active'
		WHERE key.prefix = $1
		  AND key.revoked_at IS NULL
		  AND (key.expires_at IS NULL OR key.expires_at > NOW())
		FOR SHARE OF key, tenant
	`, prefix).Scan(
		&keyID, &tenantID, &principalTyp, &userID, &scopes, &storedHash,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return Principal{}, ErrUnauthorized
	}
	if err != nil {
		return Principal{}, fmt.Errorf("look up API key: %w", err)
	}

	expectedHash := sha256.Sum256([]byte(secret))
	if len(storedHash) != len(expectedHash) ||
		subtle.ConstantTimeCompare(storedHash, expectedHash[:]) != 1 {
		return Principal{}, ErrUnauthorized
	}

	switch principalTyp {
	case "tenant":
		if userID != "" {
			return Principal{}, ErrUnauthorized
		}
	case "user":
		if !validUUID(userID) {
			return Principal{}, ErrUnauthorized
		}
		err = tx.QueryRow(ctx, `
			SELECT membership.role
			FROM users AS account
			JOIN tenant_memberships AS membership
			  ON membership.user_id = account.id
			WHERE account.id = $1 AND account.status = 'active'
			  AND membership.tenant_id = $2
			FOR SHARE OF account, membership
		`, userID, tenantID).Scan(&role)
		if errors.Is(err, pgx.ErrNoRows) {
			return Principal{}, ErrUnauthorized
		}
		if err != nil {
			return Principal{}, fmt.Errorf("resolve API key owner: %w", err)
		}
	default:
		return Principal{}, ErrUnauthorized
	}

	tag, err := tx.Exec(ctx, `
		UPDATE api_keys
		SET last_used_at = NOW()
		WHERE id = $1 AND revoked_at IS NULL
		  AND (expires_at IS NULL OR expires_at > NOW())
	`, keyID)
	if err != nil {
		return Principal{}, fmt.Errorf("update API key usage: %w", err)
	}
	if tag.RowsAffected() != 1 {
		return Principal{}, ErrUnauthorized
	}
	if err := tx.Commit(ctx); err != nil {
		return Principal{}, fmt.Errorf("complete API key authentication: %w", err)
	}

	return Principal{
		Subject:  "api-key:" + keyID,
		TenantID: tenantID,
		UserID:   userID,
		Role:     Role(role),
		Scopes:   scopeSet(scopes),
		durable:  true,
	}, nil
}

func (repository *PostgreSQLRepository) AuthenticateOIDCSubject(
	ctx context.Context,
	issuer string,
	subject string,
	tenantID string,
) (Principal, error) {
	if strings.TrimSpace(issuer) == "" || len(issuer) > 2048 ||
		strings.TrimSpace(subject) == "" || len(subject) > 255 ||
		(tenantID != "" && !validUUID(tenantID)) {
		return Principal{}, ErrUnauthorized
	}

	rows, err := repository.pool.Query(ctx, `
		SELECT account.id::text, membership.tenant_id::text, membership.role
		FROM users AS account
		JOIN tenant_memberships AS membership
		  ON membership.user_id = account.id
		JOIN tenants AS tenant
		  ON tenant.id = membership.tenant_id AND tenant.status = 'active'
		WHERE account.external_subject = $1
		  AND account.identity_issuer = $2
		  AND account.status = 'active'
		  AND ($3 = '' OR membership.tenant_id::text = $3)
		ORDER BY membership.created_at, membership.tenant_id
		LIMIT 2
	`, subject, issuer, tenantID)
	if err != nil {
		return Principal{}, fmt.Errorf("resolve OIDC user membership: %w", err)
	}
	defer rows.Close()

	type membership struct {
		userID   string
		tenantID string
		role     string
	}
	var memberships []membership
	for rows.Next() {
		var item membership
		if err := rows.Scan(&item.userID, &item.tenantID, &item.role); err != nil {
			return Principal{}, fmt.Errorf("scan OIDC user membership: %w", err)
		}
		memberships = append(memberships, item)
	}
	if err := rows.Err(); err != nil {
		return Principal{}, fmt.Errorf("iterate OIDC user memberships: %w", err)
	}
	if len(memberships) == 0 {
		return Principal{}, ErrUnauthorized
	}
	if len(memberships) > 1 {
		return Principal{}, ErrTenantSelectionRequired
	}

	member := memberships[0]
	role := Role(member.role)
	if role != Owner && role != Admin && role != Member {
		return Principal{}, ErrUnauthorized
	}
	scopes := map[string]bool{"domains:read": true, "delivery:read": true}
	if role == Owner || role == Admin {
		scopes["domains:write"] = true
		scopes["keys:write"] = true
		scopes["delivery:write"] = true
	}
	return Principal{
		Subject:  "oidc:" + subject,
		TenantID: member.tenantID,
		UserID:   member.userID,
		Role:     role,
		Scopes:   scopes,
		durable:  true,
	}, nil
}

func validAPIKey(secret string) bool {
	if len(secret) < apiKeyPrefixLength+1 ||
		!strings.HasPrefix(secret, "sp_") ||
		strings.ContainsAny(secret, "\r\n\t ") {
		return false
	}
	decoded, err := base64.RawURLEncoding.DecodeString(strings.TrimPrefix(secret, "sp_"))
	return err == nil && len(decoded) == 32
}
