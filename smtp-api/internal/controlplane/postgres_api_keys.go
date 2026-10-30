package controlplane

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
)

func (repository *PostgreSQLRepository) CreateAPIKey(
	ctx context.Context,
	principal Principal,
	name string,
	scopes []string,
	expiresAt *time.Time,
) (APIKey, string, error) {
	if !principal.Scopes["keys:write"] || !validUUID(principal.UserID) {
		return APIKey{}, "", ErrForbidden
	}
	name = strings.TrimSpace(name)
	if name == "" || len(name) > 120 || len(scopes) == 0 ||
		!validUUID(principal.TenantID) {
		return APIKey{}, "", ErrInvalid
	}
	allowedScopes := map[string]bool{
		"domains:read": true, "domains:write": true,
		"delivery:read": true, "delivery:write": true, "keys:write": true,
	}
	for _, scope := range scopes {
		if !allowedScopes[scope] || !principal.Scopes[scope] {
			return APIKey{}, "", ErrInvalid
		}
	}
	if expiresAt != nil && !expiresAt.After(time.Now()) {
		return APIKey{}, "", ErrInvalid
	}

	for attempt := 0; attempt < 3; attempt++ {
		raw := make([]byte, 32)
		if _, err := rand.Read(raw); err != nil {
			return APIKey{}, "", fmt.Errorf("generate API key: %w", err)
		}
		secret := "sp_" + base64.RawURLEncoding.EncodeToString(raw)
		prefix := secret[:apiKeyPrefixLength]
		hash := sha256.Sum256([]byte(secret))
		var key APIKey
		err := repository.pool.QueryRow(ctx, `
		INSERT INTO api_keys
		  (id, tenant_id, created_by_user_id, principal_type, name, prefix,
		   secret_hash, scopes, expires_at)
		SELECT gen_random_uuid(), membership.tenant_id, membership.user_id,
		       'user', $3, $4, $5, $6, $7
		FROM tenant_memberships AS membership
		JOIN users AS account ON account.id = membership.user_id AND account.status = 'active'
		JOIN tenants AS tenant ON tenant.id = membership.tenant_id AND tenant.status = 'active'
		WHERE membership.tenant_id = $1 AND membership.user_id = $2
		RETURNING id::text, name, prefix, scopes, expires_at, created_at, tenant_id::text
	`, principal.TenantID, principal.UserID, name, prefix, hash[:], scopes, expiresAt).Scan(
			&key.ID,
			&key.Name,
			&key.Prefix,
			&key.Scopes,
			&key.ExpiresAt,
			&key.CreatedAt,
			&key.TenantID,
		)
		if errors.Is(err, pgx.ErrNoRows) {
			return APIKey{}, "", ErrUnauthorized
		}
		if err == nil {
			return key, secret, nil
		}
		var databaseError *pgconn.PgError
		if errors.As(err, &databaseError) && databaseError.Code == "23505" {
			switch databaseError.ConstraintName {
			case "api_keys_tenant_name_unique_idx":
				return APIKey{}, "", ErrConflict
			case "api_keys_prefix_key":
				continue
			}
		}
		return APIKey{}, "", fmt.Errorf("persist API key: %w", err)
	}
	return APIKey{}, "", fmt.Errorf("generate unique API key: prefix collision limit exceeded")
}

func (repository *PostgreSQLRepository) ListAPIKeys(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]APIKey, error) {
	if !principal.Scopes["keys:write"] {
		return nil, ErrForbidden
	}
	if limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT id::text, name, prefix, scopes, expires_at, revoked_at,
		       created_at, tenant_id::text
		FROM api_keys
		WHERE tenant_id = $1
		ORDER BY created_at DESC, id
		LIMIT $2
	`, principal.TenantID, limit)
	if err != nil {
		return nil, fmt.Errorf("list API keys: %w", err)
	}
	defer rows.Close()
	keys := make([]APIKey, 0)
	for rows.Next() {
		var key APIKey
		if err := rows.Scan(
			&key.ID,
			&key.Name,
			&key.Prefix,
			&key.Scopes,
			&key.ExpiresAt,
			&key.RevokedAt,
			&key.CreatedAt,
			&key.TenantID,
		); err != nil {
			return nil, fmt.Errorf("scan API key: %w", err)
		}
		keys = append(keys, key)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate API key rows: %w", err)
	}
	return keys, nil
}

func (repository *PostgreSQLRepository) RevokeAPIKey(
	ctx context.Context,
	principal Principal,
	keyID string,
) error {
	if !principal.Scopes["keys:write"] {
		return ErrForbidden
	}
	if !validUUID(keyID) {
		return ErrNotFound
	}
	tag, err := repository.pool.Exec(ctx, `
		UPDATE api_keys
		SET revoked_at = COALESCE(revoked_at, NOW())
		WHERE id = $1 AND tenant_id = $2
	`, keyID, principal.TenantID)
	if err != nil {
		return fmt.Errorf("revoke API key: %w", err)
	}
	if tag.RowsAffected() != 1 {
		return ErrNotFound
	}
	return nil
}
