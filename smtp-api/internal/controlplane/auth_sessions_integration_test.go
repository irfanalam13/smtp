package controlplane

import (
	"context"
	"errors"
	"os"
	"strings"
	"testing"
	"time"

	smtpAuth "example.com/smtp-platform/smtp-auth"
	"github.com/jackc/pgx/v5/pgxpool"
)

func TestAuthSessionLifecycleIntegration(t *testing.T) {
	databaseURL := strings.TrimSpace(os.Getenv("TEST_DATABASE_URL"))
	if databaseURL == "" {
		t.Skip("set TEST_DATABASE_URL to an isolated database with migrations applied")
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
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := pool.Ping(ctx); err != nil {
		t.Fatal("ping isolated PostgreSQL test database")
	}

	tenantID := insertTestTenant(t, ctx, pool, "auth session integration tenant")
	userID := insertTestUser(t, ctx, pool, tenantID, "auth-session-integration-user")
	t.Cleanup(func() {
		cleanupCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if _, err := pool.Exec(cleanupCtx,
			"DELETE FROM audit_events WHERE tenant_id = $1", tenantID,
		); err != nil {
			t.Errorf("remove auth-session audit fixtures: %v", err)
		}
		cleanupTestIdentity(t, cleanupCtx, pool, []string{tenantID}, []string{userID})
	})

	principal := Principal{
		Subject: "oidc:auth-session-integration-user", TenantID: tenantID,
		UserID: userID, Role: Owner, durable: true,
	}
	repository := NewPostgreSQLRepository(pool)
	now := time.Now().UTC()
	token, tokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	csrf, csrfHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	session, err := repository.CreateAuthSession(
		ctx, principal, tokenHash, csrfHash, now.Add(30*time.Minute),
		now.Add(12*time.Hour), "Integration Browser",
	)
	if err != nil {
		t.Fatal(err)
	}
	var persistedTokenHash []byte
	if err := pool.QueryRow(ctx, `
		SELECT token_hash FROM auth_sessions WHERE id = $1
	`, session.ID).Scan(&persistedTokenHash); err != nil {
		t.Fatal(err)
	}
	if len(persistedTokenHash) != len(tokenHash) ||
		string(persistedTokenHash) != string(tokenHash[:]) {
		t.Fatal("database did not retain only the session-token verifier")
	}

	authenticated, resolved, err := repository.AuthenticateAuthSession(
		ctx, token, csrf, csrf, true, 30*time.Minute,
	)
	if err != nil || authenticated.UserID != userID || resolved.ID != session.ID {
		t.Fatalf("valid session authentication = (%#v, %#v, %v)", authenticated, resolved, err)
	}
	if _, _, err := repository.AuthenticateAuthSession(
		ctx, token, csrf, "incorrect-csrf-token", true, 30*time.Minute,
	); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("invalid CSRF token error=%v, want unauthorized", err)
	}

	secondToken, secondTokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	secondCSRF, secondCSRFHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	second, err := repository.CreateAuthSession(
		ctx, principal, secondTokenHash, secondCSRFHash, now.Add(30*time.Minute),
		now.Add(12*time.Hour), "Another Integration Browser",
	)
	if err != nil {
		t.Fatal(err)
	}
	sessions, err := repository.ListAuthSessions(ctx, authenticated)
	if err != nil || len(sessions) != 2 {
		t.Fatalf("session list = (%d, %v), want two active sessions", len(sessions), err)
	}
	if err := repository.RevokeAuthSession(ctx, authenticated, session.ID); err != nil {
		t.Fatalf("revoke selected session: %v", err)
	}
	if _, _, err := repository.AuthenticateAuthSession(
		ctx, token, csrf, csrf, false, 30*time.Minute,
	); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("revoked session error=%v, want unauthorized", err)
	}

	authenticatedSecond, _, err := repository.AuthenticateAuthSession(
		ctx, secondToken, secondCSRF, secondCSRF, true, 30*time.Minute,
	)
	if err != nil {
		t.Fatal(err)
	}
	if err := repository.RevokeAllAuthSessions(ctx, authenticatedSecond); err != nil {
		t.Fatalf("revoke all sessions: %v", err)
	}
	if _, _, err := repository.AuthenticateAuthSession(
		ctx, secondToken, secondCSRF, secondCSRF, false, 30*time.Minute,
	); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("session after logout-all error=%v, want unauthorized", err)
	}
	if second.ID == session.ID {
		t.Fatal("session identifiers unexpectedly repeated")
	}
}
