package controlplane

import (
	"context"
	"crypto/sha256"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
)

type PasswordMembership struct {
	UserID   string
	TenantID string
	Role     Role
}

type PasswordAccount struct {
	PasswordHash string
	Verified     bool
	Memberships  []PasswordMembership
}

type RegisteredAccount struct {
	UserID   string `json:"user_id"`
	TenantID string `json:"tenant_id"`
}

func (repository *PostgreSQLRepository) PasswordHashForUser(ctx context.Context, userID string) (string, error) {
	if !validUUID(userID) {
		return "", ErrUnauthorized
	}
	var passwordHash string
	err := repository.pool.QueryRow(ctx, `
		SELECT password_hash FROM users
		WHERE id = $1 AND status = 'active' AND password_hash IS NOT NULL
	`, userID).Scan(&passwordHash)
	if errors.Is(err, pgx.ErrNoRows) {
		return "", ErrUnauthorized
	}
	if err != nil {
		return "", fmt.Errorf("load account password: %w", err)
	}
	return passwordHash, nil
}

func (repository *PostgreSQLRepository) UpdatePasswordHash(ctx context.Context, userID, passwordHash string) error {
	if !validUUID(userID) || passwordHash == "" || len(passwordHash) > 512 {
		return ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return fmt.Errorf("begin password update: %w", err)
	}
	defer func() { _ = tx.Rollback(ctx) }()
	tag, err := tx.Exec(ctx, `
		UPDATE users SET password_hash = $2, updated_at = NOW()
		WHERE id = $1 AND status = 'active' AND password_hash IS NOT NULL
	`, userID, passwordHash)
	if err != nil {
		return fmt.Errorf("update account password: %w", err)
	}
	if tag.RowsAffected() != 1 {
		return ErrUnauthorized
	}
	if _, err := tx.Exec(ctx, `
		UPDATE auth_sessions SET revoked_at = NOW()
		WHERE user_id = $1 AND revoked_at IS NULL
	`, userID); err != nil {
		return fmt.Errorf("revoke sessions after password update: %w", err)
	}
	if err := tx.Commit(ctx); err != nil {
		return fmt.Errorf("commit password update: %w", err)
	}
	return nil
}

func (repository *PostgreSQLRepository) RegisterPasswordAccount(
	ctx context.Context,
	email string,
	passwordHash string,
	organizationName string,
	tokenHash [sha256.Size]byte,
	tokenExpiresAt time.Time,
) (RegisteredAccount, error) {
	return repository.RegisterPasswordAccountWithProfile(
		ctx, "User", "Account", "", "", email, passwordHash,
		organizationName, tokenHash, tokenExpiresAt,
	)
}

func (repository *PostgreSQLRepository) RegisterPasswordAccountWithProfile(
	ctx context.Context,
	firstName string,
	lastName string,
	phoneNumber string,
	country string,
	email string,
	passwordHash string,
	organizationName string,
	tokenHash [sha256.Size]byte,
	tokenExpiresAt time.Time,
) (RegisteredAccount, error) {
	if !validEmail(email) || passwordHash == "" || len(passwordHash) > 512 ||
		!validName(firstName, 2, 50) || !validName(lastName, 2, 50) ||
		!validOptionalField(phoneNumber, 7, 32) || !validOptionalField(country, 2, 100) ||
		strings.TrimSpace(organizationName) == "" || len(organizationName) > 160 ||
		!tokenExpiresAt.After(time.Now()) {
		return RegisteredAccount{}, ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return RegisteredAccount{}, fmt.Errorf("begin password account registration: %w", err)
	}
	defer func() { _ = tx.Rollback(ctx) }()

	var account RegisteredAccount
	if err := tx.QueryRow(ctx, `
		INSERT INTO tenants (id, name)
		VALUES (gen_random_uuid(), $1)
		RETURNING id::text
	`, organizationName).Scan(&account.TenantID); err != nil {
		return RegisteredAccount{}, fmt.Errorf("create account organization: %w", err)
	}
	err = tx.QueryRow(ctx, `
		INSERT INTO users (
			id, external_subject, identity_issuer, first_name, last_name,
			phone_number, country, organization_name, email, password_hash
		)
		SELECT generated.id, 'password:' || generated.id::text, 'smtp-password',
		       $1, $2, NULLIF($3, ''), NULLIF($4, ''), $5, $6, $7
		FROM (SELECT gen_random_uuid() AS id) AS generated
		RETURNING id::text
	`, firstName, lastName, strings.TrimSpace(phoneNumber), strings.TrimSpace(country),
		organizationName, email, passwordHash).Scan(&account.UserID)
	if err != nil {
		var postgresError *pgconn.PgError
		if errors.As(err, &postgresError) && postgresError.Code == "23505" {
			return RegisteredAccount{}, ErrConflict
		}
		return RegisteredAccount{}, fmt.Errorf("create password account: %w", err)
	}
	if _, err := tx.Exec(ctx, `
		INSERT INTO tenant_memberships (tenant_id, user_id, role)
		VALUES ($1, $2, 'owner')
	`, account.TenantID, account.UserID); err != nil {
		return RegisteredAccount{}, fmt.Errorf("create organization owner membership: %w", err)
	}
	if err := insertEmailToken(ctx, tx, account.UserID, "verify_email", tokenHash, tokenExpiresAt); err != nil {
		return RegisteredAccount{}, err
	}
	if _, err := tx.Exec(ctx, `
		INSERT INTO audit_events (tenant_id, actor_user_id, action, target_type, target_id)
		VALUES ($1, $2, 'account.signup', 'user', $3)
	`, account.TenantID, account.UserID, account.UserID); err != nil {
		return RegisteredAccount{}, fmt.Errorf("record account signup: %w", err)
	}
	if err := tx.Commit(ctx); err != nil {
		return RegisteredAccount{}, fmt.Errorf("commit password account registration: %w", err)
	}
	return account, nil
}

func (repository *PostgreSQLRepository) CreatePasswordEmailToken(
	ctx context.Context,
	email string,
	purpose string,
	tokenHash [sha256.Size]byte,
	expiresAt time.Time,
) (bool, error) {
	if !validEmail(email) || (purpose != "verify_email" && purpose != "reset_password") ||
		!expiresAt.After(time.Now()) {
		return false, ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return false, fmt.Errorf("begin account email token creation: %w", err)
	}
	defer func() { _ = tx.Rollback(ctx) }()

	var userID string
	err = tx.QueryRow(ctx, `
		SELECT id::text
		FROM users
		WHERE email = $1 AND status = 'active'
		  AND password_hash IS NOT NULL
		  AND (($2 = 'verify_email' AND email_verified_at IS NULL)
		    OR ($2 = 'reset_password' AND email_verified_at IS NOT NULL))
		FOR UPDATE
	`, email, purpose).Scan(&userID)
	if errors.Is(err, pgx.ErrNoRows) {
		return false, nil
	}
	if err != nil {
		return false, fmt.Errorf("find account for email token: %w", err)
	}
	if _, err := tx.Exec(ctx, `
		UPDATE auth_email_tokens
		SET used_at = NOW()
		WHERE user_id = $1 AND purpose = $2 AND used_at IS NULL
	`, userID, purpose); err != nil {
		return false, fmt.Errorf("supersede account email tokens: %w", err)
	}
	if err := insertEmailToken(ctx, tx, userID, purpose, tokenHash, expiresAt); err != nil {
		return false, err
	}
	if err := tx.Commit(ctx); err != nil {
		return false, fmt.Errorf("commit account email token: %w", err)
	}
	return true, nil
}

func insertEmailToken(
	ctx context.Context,
	tx pgx.Tx,
	userID string,
	purpose string,
	tokenHash [sha256.Size]byte,
	expiresAt time.Time,
) error {
	if _, err := tx.Exec(ctx, `
		INSERT INTO auth_email_tokens (user_id, token_hash, purpose, expires_at)
		VALUES ($1, $2, $3, $4)
	`, userID, tokenHash[:], purpose, expiresAt); err != nil {
		return fmt.Errorf("store one-time account token: %w", err)
	}
	return nil
}

func (repository *PostgreSQLRepository) PasswordAccount(
	ctx context.Context,
	email string,
) (PasswordAccount, error) {
	if !validEmail(email) {
		return PasswordAccount{}, ErrUnauthorized
	}
	var account PasswordAccount
	err := repository.pool.QueryRow(ctx, `
		SELECT password_hash, email_verified_at IS NOT NULL
		FROM users
		WHERE email = $1 AND status = 'active' AND password_hash IS NOT NULL
	`, email).Scan(&account.PasswordHash, &account.Verified)
	if errors.Is(err, pgx.ErrNoRows) {
		return PasswordAccount{}, ErrUnauthorized
	}
	if err != nil {
		return PasswordAccount{}, fmt.Errorf("load password account: %w", err)
	}

	rows, err := repository.pool.Query(ctx, `
		SELECT membership.user_id::text, membership.tenant_id::text, membership.role
		FROM tenant_memberships AS membership
		JOIN users AS account
		  ON account.id = membership.user_id AND account.status = 'active'
		JOIN tenants AS tenant
		  ON tenant.id = membership.tenant_id AND tenant.status = 'active'
		WHERE account.email = $1
		ORDER BY membership.created_at, membership.tenant_id
		LIMIT 101
	`, email)
	if err != nil {
		return PasswordAccount{}, fmt.Errorf("load password account memberships: %w", err)
	}
	defer rows.Close()
	account.Memberships = make([]PasswordMembership, 0)
	for rows.Next() {
		var membership PasswordMembership
		if err := rows.Scan(&membership.UserID, &membership.TenantID, &membership.Role); err != nil {
			return PasswordAccount{}, fmt.Errorf("scan password account membership: %w", err)
		}
		account.Memberships = append(account.Memberships, membership)
	}
	if err := rows.Err(); err != nil {
		return PasswordAccount{}, fmt.Errorf("iterate password account memberships: %w", err)
	}
	return account, nil
}

func (repository *PostgreSQLRepository) ConsumePasswordEmailToken(
	ctx context.Context,
	tokenHash [sha256.Size]byte,
	purpose string,
	passwordHash string,
) (bool, error) {
	if purpose != "verify_email" && purpose != "reset_password" {
		return false, ErrInvalid
	}
	if purpose == "reset_password" && (passwordHash == "" || len(passwordHash) > 512) {
		return false, ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return false, fmt.Errorf("begin account token consumption: %w", err)
	}
	defer func() { _ = tx.Rollback(ctx) }()

	var userID string
	err = tx.QueryRow(ctx, `
		SELECT user_id::text
		FROM auth_email_tokens
		WHERE token_hash = $1 AND purpose = $2
		  AND used_at IS NULL AND expires_at > NOW()
		FOR UPDATE
	`, tokenHash[:], purpose).Scan(&userID)
	if errors.Is(err, pgx.ErrNoRows) {
		return false, nil
	}
	if err != nil {
		return false, fmt.Errorf("load account email token: %w", err)
	}
	if purpose == "verify_email" {
		_, err = tx.Exec(ctx, `
			UPDATE users SET email_verified_at = COALESCE(email_verified_at, NOW()), updated_at = NOW()
			WHERE id = $1 AND status = 'active' AND password_hash IS NOT NULL
		`, userID)
	} else {
		_, err = tx.Exec(ctx, `
			UPDATE users
			SET password_hash = $2, updated_at = NOW()
			WHERE id = $1 AND status = 'active' AND password_hash IS NOT NULL
		`, userID, passwordHash)
		if err == nil {
			_, err = tx.Exec(ctx, `
				UPDATE auth_sessions SET revoked_at = NOW()
				WHERE user_id = $1 AND revoked_at IS NULL
			`, userID)
		}
	}
	if err != nil {
		return false, fmt.Errorf("update password account: %w", err)
	}
	if _, err := tx.Exec(ctx, `
		UPDATE auth_email_tokens
		SET used_at = NOW()
		WHERE user_id = $1 AND used_at IS NULL
	`, userID); err != nil {
		return false, fmt.Errorf("consume account email tokens: %w", err)
	}
	action := "account.email_verified"
	if purpose == "reset_password" {
		action = "account.password_reset"
	}
	if _, err := tx.Exec(ctx, `
		INSERT INTO audit_events (tenant_id, actor_user_id, action, target_type, target_id)
		SELECT membership.tenant_id, $1, $2, 'user', $3
		FROM tenant_memberships AS membership
		WHERE membership.user_id = $1
		ORDER BY membership.tenant_id
		LIMIT 1
	`, userID, action, userID); err != nil {
		return false, fmt.Errorf("record account security event: %w", err)
	}
	if err := tx.Commit(ctx); err != nil {
		return false, fmt.Errorf("commit account email token consumption: %w", err)
	}
	return true, nil
}

func validEmail(email string) bool {
	return len(email) >= 3 && len(email) <= 320 &&
		strings.TrimSpace(email) == email &&
		!strings.ContainsAny(email, "\r\n\t ")
}
