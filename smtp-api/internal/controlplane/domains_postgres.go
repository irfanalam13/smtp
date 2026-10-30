package controlplane

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
)

type DomainChallenge struct {
	RecordName  string `json:"record_name"`
	RecordType  string `json:"record_type"`
	RecordValue string `json:"record_value"`
	Secret      string `json:"secret"`
}

func (repository *PostgreSQLRepository) ListDomains(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]Domain, error) {
	if !principal.durable || !validUUID(principal.TenantID) {
		return nil, ErrUnauthorized
	}
	if limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT domain.id::text, domain.tenant_id::text, domain.domain,
		       domain.status, domain.created_at
		FROM domains AS domain
		JOIN tenants AS tenant
		  ON tenant.id = domain.tenant_id AND tenant.status = 'active'
		WHERE domain.tenant_id = $1
		ORDER BY domain.created_at DESC, domain.id DESC
		LIMIT $2
	`, principal.TenantID, limit)
	if err != nil {
		return nil, fmt.Errorf("list tenant domains: %w", err)
	}
	defer rows.Close()

	items := make([]Domain, 0)
	for rows.Next() {
		var item Domain
		if err := rows.Scan(
			&item.ID, &item.TenantID, &item.Domain, &item.Status, &item.CreatedAt,
		); err != nil {
			return nil, fmt.Errorf("scan tenant domain: %w", err)
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate tenant domains: %w", err)
	}
	return items, nil
}

func (repository *PostgreSQLRepository) CreateDomain(
	ctx context.Context,
	principal Principal,
	rawDomain string,
) (Domain, error) {
	if !principal.durable || !validUUID(principal.TenantID) {
		return Domain{}, ErrUnauthorized
	}
	domain, err := NormalizeDomain(rawDomain)
	if err != nil {
		return Domain{}, ErrInvalid
	}
	var item Domain
	err = repository.pool.QueryRow(ctx, `
		INSERT INTO domains (id, tenant_id, domain, status)
		SELECT gen_random_uuid(), tenant.id, $2, 'pending_verification'
		FROM tenants AS tenant
		WHERE tenant.id = $1 AND tenant.status = 'active'
		RETURNING id::text, tenant_id::text, domain, status, created_at
	`, principal.TenantID, domain).Scan(
		&item.ID, &item.TenantID, &item.Domain, &item.Status, &item.CreatedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return Domain{}, ErrUnauthorized
	}
	if err != nil {
		var databaseError *pgconn.PgError
		if errors.As(err, &databaseError) && databaseError.Code == "23505" {
			return Domain{}, ErrConflict
		}
		return Domain{}, fmt.Errorf("create tenant domain: %w", err)
	}
	return item, nil
}

func (repository *PostgreSQLRepository) GetDomain(
	ctx context.Context,
	principal Principal,
	domainID string,
) (Domain, error) {
	if !principal.durable || !validUUID(principal.TenantID) || !validUUID(domainID) {
		return Domain{}, ErrNotFound
	}
	var item Domain
	err := repository.pool.QueryRow(ctx, `
		SELECT domain.id::text, domain.tenant_id::text, domain.domain,
		       domain.status, domain.created_at
		FROM domains AS domain
		JOIN tenants AS tenant
		  ON tenant.id = domain.tenant_id AND tenant.status = 'active'
		WHERE domain.id = $1 AND domain.tenant_id = $2
	`, domainID, principal.TenantID).Scan(
		&item.ID, &item.TenantID, &item.Domain, &item.Status, &item.CreatedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return Domain{}, ErrNotFound
	}
	if err != nil {
		return Domain{}, fmt.Errorf("get tenant domain: %w", err)
	}
	return item, nil
}

func (repository *PostgreSQLRepository) CreateDomainChallenge(
	ctx context.Context,
	principal Principal,
	domainID string,
) (DomainChallenge, error) {
	if !principal.durable || !validUUID(principal.TenantID) || !validUUID(domainID) {
		return DomainChallenge{}, ErrNotFound
	}
	raw := make([]byte, 32)
	if _, err := rand.Read(raw); err != nil {
		return DomainChallenge{}, fmt.Errorf("generate domain verification token: %w", err)
	}
	secret := base64.RawURLEncoding.EncodeToString(raw)
	hash := sha256.Sum256([]byte(secret))
	var insertedID string
	err := repository.pool.QueryRow(ctx, `
		INSERT INTO domain_verification_challenges
		  (domain_id, token_hash, expires_at, used_at, created_at)
		SELECT domain.id, $3, NOW() + INTERVAL '24 hours', NULL, NOW()
		FROM domains AS domain
		WHERE domain.id = $1 AND domain.tenant_id = $2
		  AND domain.status != 'disabled'
		ON CONFLICT (domain_id) DO UPDATE
		SET token_hash = EXCLUDED.token_hash,
		    expires_at = EXCLUDED.expires_at,
		    used_at = NULL,
		    created_at = NOW()
		RETURNING domain_id::text
	`, domainID, principal.TenantID, hash[:]).Scan(&insertedID)
	if errors.Is(err, pgx.ErrNoRows) {
		return DomainChallenge{}, ErrNotFound
	}
	if err != nil {
		return DomainChallenge{}, fmt.Errorf("create domain verification challenge: %w", err)
	}
	return DomainChallenge{
		RecordName:  "_smtp-platform-verification",
		RecordType:  "TXT",
		RecordValue: "smtp-platform-verification=" + hex.EncodeToString(hash[:]),
		Secret:      secret,
	}, nil
}

func (repository *PostgreSQLRepository) CheckDomainChallenge(
	ctx context.Context,
	principal Principal,
	domainID string,
	resolver TXTResolver,
) error {
	if !principal.durable || !validUUID(principal.TenantID) || !validUUID(domainID) {
		return ErrNotFound
	}
	var (
		domain    string
		status    string
		tokenHash []byte
		expiresAt *time.Time
		usedAt    *time.Time
	)
	err := repository.pool.QueryRow(ctx, `
		SELECT domain.domain, domain.status, challenge.token_hash,
		       challenge.expires_at, challenge.used_at
		FROM domains AS domain
		JOIN tenants AS tenant
		  ON tenant.id = domain.tenant_id AND tenant.status = 'active'
		LEFT JOIN domain_verification_challenges AS challenge
		  ON challenge.domain_id = domain.id
		WHERE domain.id = $1 AND domain.tenant_id = $2
	`, domainID, principal.TenantID).Scan(
		&domain, &status, &tokenHash, &expiresAt, &usedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return ErrNotFound
	}
	if err != nil {
		return fmt.Errorf("load domain verification challenge: %w", err)
	}
	if status == "disabled" || len(tokenHash) != sha256.Size ||
		expiresAt == nil || usedAt != nil || !expiresAt.After(time.Now()) {
		return ErrVerificationPending
	}

	values, err := resolver.LookupTXT("_smtp-platform-verification." + domain)
	if err != nil {
		return ErrVerificationPending
	}
	expected := "smtp-platform-verification=" + hex.EncodeToString(tokenHash)
	found := false
	for _, value := range values {
		if subtle.ConstantTimeCompare([]byte(value), []byte(expected)) == 1 {
			found = true
			break
		}
	}
	if !found {
		return ErrVerificationPending
	}

	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin domain verification transaction: %w", err)
	}
	defer func() {
		_ = tx.Rollback(ctx)
	}()
	result, err := tx.Exec(ctx, `
		UPDATE domain_verification_challenges
		SET used_at = NOW()
		WHERE domain_id = $1 AND token_hash = $2
		  AND used_at IS NULL AND expires_at > NOW()
	`, domainID, tokenHash)
	if err != nil {
		return fmt.Errorf("consume domain verification challenge: %w", err)
	}
	if result.RowsAffected() != 1 {
		return ErrVerificationPending
	}
	result, err = tx.Exec(ctx, `
		UPDATE domains SET status = 'verified', updated_at = NOW()
		WHERE id = $1 AND tenant_id = $2 AND status != 'disabled'
	`, domainID, principal.TenantID)
	if err != nil {
		return fmt.Errorf("mark tenant domain verified: %w", err)
	}
	if result.RowsAffected() != 1 {
		return ErrNotFound
	}
	if err := tx.Commit(ctx); err != nil {
		return fmt.Errorf("commit domain verification: %w", err)
	}
	return nil
}

func (repository *PostgreSQLRepository) DisableDomain(
	ctx context.Context,
	principal Principal,
	domainID string,
) error {
	if !principal.durable || !validUUID(principal.TenantID) || !validUUID(domainID) {
		return ErrNotFound
	}
	result, err := repository.pool.Exec(ctx, `
		UPDATE domains
		SET status = 'disabled', updated_at = NOW()
		WHERE id = $1 AND tenant_id = $2
	`, domainID, principal.TenantID)
	if err != nil {
		return fmt.Errorf("disable tenant domain: %w", err)
	}
	if result.RowsAffected() != 1 {
		return ErrNotFound
	}
	return nil
}

func requireVerifiedSenderDomain(
	ctx context.Context,
	tx pgx.Tx,
	principal Principal,
	address string,
) error {
	if !principal.durable || !validUUID(principal.TenantID) {
		return ErrUnauthorized
	}
	separator := strings.LastIndexByte(address, '@')
	if separator < 0 || separator == len(address)-1 {
		return ErrInvalid
	}
	domain := strings.ToLower(address[separator+1:])
	var domainID string
	err := tx.QueryRow(ctx, `
		SELECT domain.id::text
		FROM domains AS domain
		JOIN tenants AS tenant
		  ON tenant.id = domain.tenant_id AND tenant.status = 'active'
		WHERE domain.tenant_id = $1 AND domain.domain = $2
		  AND domain.status = 'verified'
		FOR SHARE OF domain, tenant
	`, principal.TenantID, domain).Scan(&domainID)
	if errors.Is(err, pgx.ErrNoRows) {
		return ErrForbidden
	}
	if err != nil {
		return fmt.Errorf("authorize verified SMTP sender domain: %w", err)
	}
	return nil
}
