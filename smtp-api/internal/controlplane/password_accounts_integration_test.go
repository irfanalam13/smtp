package controlplane

import (
	"context"
	"errors"
	"fmt"
	"os"
	"strings"
	"testing"
	"time"

	smtpAuth "example.com/smtp-platform/smtp-auth"
	"github.com/jackc/pgx/v5/pgxpool"
)

func TestPasswordAccountLifecycleIntegration(t *testing.T) {
	databaseURL := strings.TrimSpace(os.Getenv("TEST_DATABASE_URL"))
	if databaseURL == "" {
		t.Skip("set TEST_DATABASE_URL to an isolated database with migrations 000028 and 000029 applied")
	}
	config, err := pgxpool.ParseConfig(databaseURL)
	if err != nil {
		t.Fatal("invalid TEST_DATABASE_URL")
	}
	if !strings.Contains(strings.ToLower(config.ConnConfig.Database), "test") {
		t.Fatal("TEST_DATABASE_URL must identify an isolated test database")
	}
	config.MaxConns = 3
	pool, err := pgxpool.NewWithConfig(context.Background(), config)
	if err != nil {
		t.Fatal("connect to isolated PostgreSQL test database")
	}
	t.Cleanup(pool.Close)
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()
	if err := pool.Ping(ctx); err != nil {
		t.Fatal("ping isolated PostgreSQL test database")
	}

	repository := NewPostgreSQLRepository(pool)
	verificationToken, verificationHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	_, resetHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	email := fmt.Sprintf(
		"password-account-%s@example.test",
		strings.ReplaceAll(verificationToken[:16], "-", ""),
	)
	initialPassword, err := smtpAuth.HashPassword("initial integration password")
	if err != nil {
		t.Fatal(err)
	}
	registered, err := repository.RegisterPasswordAccount(
		ctx, email, initialPassword, "Password account integration",
		verificationHash, time.Now().Add(24*time.Hour),
	)
	if err != nil {
		t.Fatalf("register password account: %v", err)
	}
	t.Cleanup(func() {
		cleanupCtx, cleanupCancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cleanupCancel()
		if _, err := pool.Exec(cleanupCtx,
			"DELETE FROM audit_events WHERE tenant_id = $1", registered.TenantID,
		); err != nil {
			t.Errorf("remove password-account audit fixtures: %v", err)
		}
		cleanupTestIdentity(
			t, cleanupCtx, pool,
			[]string{registered.TenantID}, []string{registered.UserID},
		)
	})

	account, err := repository.PasswordAccount(ctx, email)
	if err != nil || account.Verified || len(account.Memberships) != 1 ||
		account.Memberships[0].Role != Owner ||
		account.Memberships[0].TenantID != registered.TenantID {
		t.Fatalf("new account = (%#v, %v), want an unverified workspace owner", account, err)
	}
	consumed, err := repository.ConsumePasswordEmailToken(
		ctx, verificationHash, "verify_email", "",
	)
	if err != nil || !consumed {
		t.Fatalf("consume verification token = (%t, %v), want true", consumed, err)
	}
	consumed, err = repository.ConsumePasswordEmailToken(
		ctx, verificationHash, "verify_email", "",
	)
	if err != nil || consumed {
		t.Fatalf("reuse verification token = (%t, %v), want false", consumed, err)
	}
	account, err = repository.PasswordAccount(ctx, email)
	if err != nil || !account.Verified {
		t.Fatalf("verified account = (%#v, %v), want verified", account, err)
	}

	sessionToken, sessionHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	csrfToken, csrfHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	principal := Principal{
		Subject: "password:" + registered.UserID,
		UserID:  registered.UserID, TenantID: registered.TenantID,
		Role: Owner, durable: true,
	}
	if _, err := repository.CreateAuthSession(
		ctx, principal, sessionHash, csrfHash,
		time.Now().Add(30*time.Minute), time.Now().Add(12*time.Hour),
		"Password Account Integration",
	); err != nil {
		t.Fatalf("create session before reset: %v", err)
	}
	created, err := repository.CreatePasswordEmailToken(
		ctx, email, "reset_password", resetHash, time.Now().Add(30*time.Minute),
	)
	if err != nil || !created {
		t.Fatalf("create password-reset token = (%t, %v), want true", created, err)
	}
	nextPassword, err := smtpAuth.HashPassword("replacement integration password")
	if err != nil {
		t.Fatal(err)
	}
	consumed, err = repository.ConsumePasswordEmailToken(
		ctx, resetHash, "reset_password", nextPassword,
	)
	if err != nil || !consumed {
		t.Fatalf("consume password-reset token = (%t, %v), want true", consumed, err)
	}
	consumed, err = repository.ConsumePasswordEmailToken(
		ctx, resetHash, "reset_password", nextPassword,
	)
	if err != nil || consumed {
		t.Fatalf("reuse password-reset token = (%t, %v), want false", consumed, err)
	}
	account, err = repository.PasswordAccount(ctx, email)
	if err != nil {
		t.Fatal(err)
	}
	passwordMatches, err := smtpAuth.VerifyPassword(account.PasswordHash, "replacement integration password")
	if err != nil || !passwordMatches {
		t.Fatalf("replacement password verification = (%t, %v), want true", passwordMatches, err)
	}
	if _, _, err := repository.AuthenticateAuthSession(
		ctx, sessionToken, csrfToken, csrfToken, false, 30*time.Minute,
	); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("session after password reset error=%v, want unauthorized", err)
	}
}
