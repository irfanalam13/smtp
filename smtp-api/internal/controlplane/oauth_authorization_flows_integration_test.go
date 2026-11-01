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

func TestOAuthAuthorizationFlowIsProviderBoundAndSingleUseIntegration(t *testing.T) {
	databaseURL := strings.TrimSpace(os.Getenv("TEST_DATABASE_URL"))
	if databaseURL == "" {
		t.Skip("set TEST_DATABASE_URL to an isolated database with migrations through 000037 applied")
	}
	config, err := pgxpool.ParseConfig(databaseURL)
	if err != nil || !strings.Contains(strings.ToLower(config.ConnConfig.Database), "test") {
		t.Fatal("TEST_DATABASE_URL must identify an isolated test database")
	}
	pool, err := pgxpool.NewWithConfig(context.Background(), config)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(pool.Close)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := pool.Ping(ctx); err != nil {
		t.Fatal(err)
	}

	repository := NewPostgreSQLRepository(pool)
	state, stateHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	nonce, nonceHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	verifier, verifierHash, err := smtpAuth.NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	if err := repository.CreateOAuthAuthorizationFlow(
		ctx, "google", stateHash, nonceHash, verifierHash, "", time.Now().Add(time.Minute),
	); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		_, _ = pool.Exec(context.Background(), "DELETE FROM oauth_authorization_flows WHERE state_hash = $1", stateHash[:])
	})

	if _, err := repository.ConsumeOAuthAuthorizationFlow(
		ctx, "github", smtpAuth.HashSecret(state), smtpAuth.HashSecret(nonce), smtpAuth.HashSecret(verifier),
	); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("cross-provider flow consumption error=%v, want unauthorized", err)
	}
	if _, err := repository.ConsumeOAuthAuthorizationFlow(
		ctx, "google", smtpAuth.HashSecret(state), smtpAuth.HashSecret(nonce), smtpAuth.HashSecret(verifier),
	); err != nil {
		t.Fatalf("consume valid flow: %v", err)
	}
	if _, err := repository.ConsumeOAuthAuthorizationFlow(
		ctx, "google", smtpAuth.HashSecret(state), smtpAuth.HashSecret(nonce), smtpAuth.HashSecret(verifier),
	); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("replayed flow consumption error=%v, want unauthorized", err)
	}
}
