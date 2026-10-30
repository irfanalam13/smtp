package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log/slog"
	"os"
	"strings"
	"time"

	"example.com/smtp-platform/smtp-api/internal/controlplane"
	"github.com/jackc/pgx/v5/pgxpool"
)

func main() {
	var (
		tenantID = flag.String("tenant-id", "", "existing tenant UUID")
		userID   = flag.String("user-id", "", "existing tenant member UUID")
		name     = flag.String("name", "", "API key name")
		rawScope = flag.String("scopes", "", "comma-separated initial key scopes")
	)
	flag.Parse()
	if *tenantID == "" || *userID == "" || *name == "" || *rawScope == "" {
		slog.Error("tenant-id, user-id, name, and scopes are required")
		os.Exit(2)
	}

	scopes := strings.Split(*rawScope, ",")
	scopeSet := map[string]bool{"keys:write": true}
	for i := range scopes {
		scopes[i] = strings.TrimSpace(scopes[i])
		if scopes[i] == "" {
			slog.Error("scopes must be non-empty comma-separated values")
			os.Exit(2)
		}
		scopeSet[scopes[i]] = true
	}

	if err := run(*tenantID, *userID, *name, scopes, scopeSet); err != nil {
		slog.Error("API key bootstrap failed", "error", err)
		os.Exit(1)
	}
}

func run(tenantID, userID, name string, scopes []string, scopeSet map[string]bool) error {
	poolConfig, err := controlplane.PostgreSQLPoolConfigFromEnv(os.Getenv)
	if err != nil {
		return fmt.Errorf("configure PostgreSQL: %w", err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	pool, err := pgxpool.NewWithConfig(ctx, poolConfig)
	if err != nil {
		return errors.New("create PostgreSQL connection pool")
	}
	defer pool.Close()
	if err := pool.Ping(ctx); err != nil {
		return errors.New("connect to PostgreSQL")
	}

	principal := controlplane.Principal{
		TenantID: tenantID,
		UserID:   userID,
		Scopes:   scopeSet,
	}
	_, secret, err := controlplane.NewPostgreSQLRepository(pool).CreateAPIKey(
		ctx, principal, name, scopes, nil,
	)
	if err != nil {
		return fmt.Errorf("create key for the specified active tenant member: %w", err)
	}
	fmt.Println(secret)
	return nil
}
