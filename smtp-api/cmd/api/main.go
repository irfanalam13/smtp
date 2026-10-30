package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"example.com/smtp-platform/smtp-api/internal/controlplane"
	"example.com/smtp-platform/smtp-api/internal/health"
	"example.com/smtp-platform/smtp-api/internal/identity"
	"example.com/smtp-platform/smtp-api/internal/observability"
	"github.com/jackc/pgx/v5/pgxpool"
)

func main() {
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stderr, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	})))
	if err := run(); err != nil {
		slog.Error("API stopped", "error", err)
		os.Exit(1)
	}
}

func run() error {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health/live", health.Handler)
	readiness := &health.Readiness{}
	environment := strings.ToLower(strings.TrimSpace(os.Getenv("APP_ENVIRONMENT")))
	devMode := os.Getenv("CONTROL_PLANE_DEV_MODE") == "true"
	if devMode && environment != "development" && environment != "dev" {
		return errors.New("development identity adapter requires a development environment")
	}
	oidcVerifier, err := newOIDCVerifierFromEnv(os.Getenv)
	if err != nil {
		return fmt.Errorf("configure API OIDC verifier: %w", err)
	}

	poolConfig, err := controlplane.PostgreSQLPoolConfigFromEnv(os.Getenv)
	if err != nil {
		return fmt.Errorf("configure API PostgreSQL pool: %w", err)
	}
	connectCtx, cancelConnect := context.WithTimeout(context.Background(), 6*time.Second)
	pool, err := pgxpool.NewWithConfig(connectCtx, poolConfig)
	if err != nil {
		cancelConnect()
		return fmt.Errorf("create API PostgreSQL pool: %w", err)
	}
	if err := pool.Ping(connectCtx); err != nil {
		pool.Close()
		cancelConnect()
		return fmt.Errorf("connect API to PostgreSQL: %w", err)
	}
	cancelConnect()
	repository := controlplane.NewPostgreSQLRepository(pool)
	defer repository.Close()

	mux.HandleFunc("GET /health/ready", readiness.Handler(func() bool {
		readyCtx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		return repository.Ping(readyCtx) == nil
	}))
	metrics := observability.NewMetrics()
	control := controlplane.Handler{
		Store:    controlplane.NewStore(),
		Postgres: repository,
		OIDC:     oidcVerifier,
		DevMode:  devMode,
		Metrics:  metrics,
	}
	mux.Handle("/v1/", control.Routes())
	mux.HandleFunc("GET /metrics", metrics.Handler)

	addr := os.Getenv("API_ADDR")
	if addr == "" {
		addr = "127.0.0.1:8080"
	}

	server := &http.Server{
		Addr:              addr,
		Handler:           securityHeaders(mux),
		ReadHeaderTimeout: 5 * time.Second,
		IdleTimeout:       60 * time.Second,
		MaxHeaderBytes:    64 << 10,
	}
	slog.Info("API listening", "address", addr)
	serverErrors := make(chan error, 1)
	go func() {
		serverErrors <- server.ListenAndServe()
	}()

	shutdownCtx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	select {
	case err := <-serverErrors:
		if !errors.Is(err, http.ErrServerClosed) {
			return fmt.Errorf("serve API: %w", err)
		}
	case <-shutdownCtx.Done():
		readiness.Drain()
		time.Sleep(time.Second)
		gracefulCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := server.Shutdown(gracefulCtx); err != nil {
			return fmt.Errorf("shut down API server: %w", err)
		}
	}
	return nil
}

func newOIDCVerifierFromEnv(getenv func(string) string) (controlplane.OIDCVerifier, error) {
	issuer := strings.TrimSpace(getenv("OIDC_ISSUER_URL"))
	audience := strings.TrimSpace(getenv("OIDC_API_AUDIENCE"))
	if issuer == "" && audience == "" {
		return nil, nil
	}
	if issuer == "" || audience == "" {
		return nil, errors.New("OIDC_ISSUER_URL and OIDC_API_AUDIENCE must both be configured")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	return identity.NewKeycloakVerifier(ctx, issuer, audience)
}

func securityHeaders(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		w.Header().Set("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'")
		w.Header().Set("Referrer-Policy", "no-referrer")
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("X-Frame-Options", "DENY")
		next.ServeHTTP(w, r)
	})
}
