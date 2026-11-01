package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"net/url"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"example.com/smtp-platform/smtp-api/internal/authmail"
	"example.com/smtp-platform/smtp-api/internal/controlplane"
	"example.com/smtp-platform/smtp-api/internal/health"
	"example.com/smtp-platform/smtp-api/internal/identity"
	"example.com/smtp-platform/smtp-api/internal/observability"
	smtpAuth "example.com/smtp-platform/smtp-auth"
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
	authSessions, err := browserSessionConfigFromEnv(os.Getenv)
	if err != nil {
		return fmt.Errorf("configure browser sessions: %w", err)
	}
	jwtConfig, err := jwtConfigFromEnv(os.Getenv, environment, authSessions.PublicOrigin)
	if err != nil {
		return fmt.Errorf("configure JWT authentication: %w", err)
	}
	oidcLogin, err := newOIDCLoginFromEnv(os.Getenv, authSessions.PublicOrigin)
	if err != nil {
		return fmt.Errorf("configure OIDC login: %w", err)
	}
	configuredMailer, err := authmail.NewSMTPFromEnv(os.Getenv)
	if err != nil {
		return fmt.Errorf("configure authentication email: %w", err)
	}
	var authMailer authmail.Sender
	if configuredMailer != nil {
		authMailer = configuredMailer
	} else if devMode || !authSessions.SecureCookies {
		authMailer = &authmail.DevLogger{}
	}
	dummyPasswordHash, err := smtpAuth.HashPassword("dummy-password-verifier-not-for-login")
	if err != nil {
		return fmt.Errorf("initialize password verifier: %w", err)
	}
	mailboxProxy, err := controlplane.NewMailboxProxyFromEnv(os.Getenv)
	if err != nil {
		return fmt.Errorf("configure mailbox service proxy: %w", err)
	}

	mux.HandleFunc("GET /health/ready", readiness.Handler(func() bool {
		readyCtx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		return repository.Ping(readyCtx) == nil
	}))
	metrics := observability.NewMetrics()
	socialOAuth := socialOAuthConfigFromEnv(os.Getenv, authSessions.PublicOrigin)
	control := controlplane.Handler{
		Store:             controlplane.NewStore(),
		Postgres:          repository,
		MailboxProxy:      mailboxProxy,
		OIDC:              oidcVerifier,
		OIDCLogin:         oidcLogin,
		AuthMailer:        authMailer,
		DummyPasswordHash: dummyPasswordHash,
		AuthSessions:      authSessions,
		SocialOAuth:       socialOAuth,
		JWT:               jwtConfig,
		DevMode:           devMode,
		Metrics:           metrics,
	}
	mux.Handle("/v1/", control.Routes())
	// Keep the versioned public API contract available when the API is deployed
	// behind a gateway that does not rewrite the /api prefix.
	mux.Handle("/api/v1/", http.StripPrefix("/api", control.Routes()))
	mux.Handle("/api/backend/v1/", http.StripPrefix("/api/backend", control.Routes()))
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
	shutdownCtx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	go func() {
		serverErrors <- server.ListenAndServe()
	}()
	go func() {
		ticker := time.NewTicker(24 * time.Hour)
		defer ticker.Stop()
		for {
			ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
			if err := repository.EnforceAPIKeyLifecycle(ctx, authMailer); err != nil {
				slog.Error("API-key lifecycle enforcement failed", "error", err)
			}
			cancel()
			select {
			case <-ticker.C:
			case <-shutdownCtx.Done():
				return
			}
		}
	}()

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

func jwtConfigFromEnv(getenv func(string) string, environment, issuer string) (controlplane.JWTConfig, error) {
	key := strings.TrimSpace(getenv("AUTH_JWT_SECRET"))
	development := environment == "development" || environment == "dev"
	if key == "" && development {
		// This key is deliberately limited to local development. Deployments must
		// supply a unique 32+ byte secret through their secret manager.
		key = "local-development-jwt-secret-change-before-deploy"
	}
	if len(key) < 32 {
		return controlplane.JWTConfig{}, errors.New("AUTH_JWT_SECRET must be at least 32 bytes")
	}
	ttl, err := configuredDuration(getenv("AUTH_JWT_TTL"), 15*time.Minute)
	if err != nil || ttl < time.Minute || ttl > 24*time.Hour {
		return controlplane.JWTConfig{}, errors.New("AUTH_JWT_TTL must be between 1m and 24h")
	}
	return controlplane.JWTConfig{SigningKey: []byte(key), Issuer: issuer, TTL: ttl}, nil
}

func newOIDCLoginFromEnv(
	getenv func(string) string,
	publicOrigin string,
) (*smtpAuth.OIDCClient, error) {
	issuer := strings.TrimSpace(getenv("OIDC_ISSUER_URL"))
	clientID := strings.TrimSpace(getenv("OIDC_CLIENT_ID"))
	clientSecret := strings.TrimSpace(getenv("OIDC_CLIENT_SECRET"))
	redirectURL := strings.TrimSpace(getenv("OIDC_REDIRECT_URI"))
	if clientID == "" && clientSecret == "" && redirectURL == "" {
		return nil, nil
	}
	if issuer == "" || clientID == "" || clientSecret == "" {
		return nil, errors.New("OIDC_ISSUER_URL, OIDC_CLIENT_ID, and OIDC_CLIENT_SECRET must be configured together")
	}
	expectedRedirect := publicOrigin + "/api/backend/v1/auth/oidc/callback"
	if redirectURL == "" {
		redirectURL = expectedRedirect
	}
	if redirectURL != expectedRedirect {
		return nil, errors.New("OIDC_REDIRECT_URI must exactly match the same-origin dashboard callback URL")
	}
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	return smtpAuth.NewOIDCClient(ctx, smtpAuth.OIDCConfig{
		Issuer: issuer, ClientID: clientID, ClientSecret: clientSecret, RedirectURL: redirectURL,
	})
}

func socialOAuthConfigFromEnv(getenv func(string) string, publicOrigin string) controlplane.SocialOAuthConfig {
	googleRedirect := strings.TrimSpace(getenv("GOOGLE_REDIRECT_URI"))
	if googleRedirect == "" {
		googleRedirect = publicOrigin + "/api/backend/v1/auth/oauth/google/callback"
	}
	githubRedirect := strings.TrimSpace(getenv("GITHUB_REDIRECT_URI"))
	if githubRedirect == "" {
		githubRedirect = publicOrigin + "/api/backend/v1/auth/oauth/github/callback"
	}
	return controlplane.SocialOAuthConfig{
		GoogleClientID:     strings.TrimSpace(getenv("GOOGLE_CLIENT_ID")),
		GoogleClientSecret: strings.TrimSpace(getenv("GOOGLE_CLIENT_SECRET")),
		GoogleRedirectURI:  googleRedirect,
		GitHubClientID:     strings.TrimSpace(getenv("GITHUB_CLIENT_ID")),
		GitHubClientSecret: strings.TrimSpace(getenv("GITHUB_CLIENT_SECRET")),
		GitHubRedirectURI:  githubRedirect,
	}
}

func browserSessionConfigFromEnv(getenv func(string) string) (controlplane.BrowserSessionConfig, error) {
	environment := strings.ToLower(strings.TrimSpace(getenv("APP_ENVIRONMENT")))
	development := environment == "development" || environment == "dev"
	production := environment == "production" || environment == "prod"
	origin := strings.TrimSpace(getenv("APP_PUBLIC_ORIGIN"))
	if origin == "" {
		switch {
		case development:
			origin = "http://127.0.0.1:3210"
		case production:
			return controlplane.BrowserSessionConfig{}, errors.New("APP_PUBLIC_ORIGIN is required in production")
		default:
			return controlplane.BrowserSessionConfig{}, errors.New("APP_PUBLIC_ORIGIN is required outside development")
		}
	}
	parsed, err := url.Parse(origin)
	if err != nil || parsed.Host == "" || parsed.User != nil ||
		parsed.RawQuery != "" || parsed.Fragment != "" || parsed.Path != "" ||
		(parsed.Scheme != "https" && parsed.Scheme != "http") {
		return controlplane.BrowserSessionConfig{}, errors.New("APP_PUBLIC_ORIGIN must be an HTTP(S) origin without a path")
	}
	secureCookies := !development || parsed.Scheme == "https"
	if secureCookies && parsed.Scheme != "https" {
		return controlplane.BrowserSessionConfig{}, errors.New("APP_PUBLIC_ORIGIN must use HTTPS outside development")
	}
	if !secureCookies && parsed.Scheme == "http" {
		host := strings.ToLower(parsed.Hostname())
		if host != "localhost" && host != "127.0.0.1" && host != "::1" {
			return controlplane.BrowserSessionConfig{}, errors.New("HTTP APP_PUBLIC_ORIGIN is only allowed on a development loopback host")
		}
	}
	canonicalOrigin := parsed.Scheme + "://" + parsed.Host
	if canonicalOrigin != origin {
		return controlplane.BrowserSessionConfig{}, errors.New("APP_PUBLIC_ORIGIN must be a canonical origin")
	}

	sessionCookie := strings.TrimSpace(getenv("AUTH_SESSION_COOKIE_NAME"))
	csrfCookie := strings.TrimSpace(getenv("AUTH_CSRF_COOKIE_NAME"))
	if secureCookies {
		if sessionCookie == "" {
			sessionCookie = "__Host-smtp_session"
		}
		if csrfCookie == "" {
			csrfCookie = "__Host-smtp_csrf"
		}
		if !strings.HasPrefix(sessionCookie, "__Host-") || !strings.HasPrefix(csrfCookie, "__Host-") {
			return controlplane.BrowserSessionConfig{}, errors.New("secure auth cookies must use the __Host- prefix")
		}
	} else {
		if sessionCookie == "" {
			sessionCookie = "smtp_session"
		}
		if csrfCookie == "" {
			csrfCookie = "smtp_csrf"
		}
	}
	idleTTL, err := configuredDuration(getenv("AUTH_SESSION_IDLE_TTL"), 30*time.Minute)
	if err != nil {
		return controlplane.BrowserSessionConfig{}, fmt.Errorf("AUTH_SESSION_IDLE_TTL: %w", err)
	}
	absoluteTTL, err := configuredDuration(getenv("AUTH_SESSION_ABSOLUTE_TTL"), 12*time.Hour)
	if err != nil {
		return controlplane.BrowserSessionConfig{}, fmt.Errorf("AUTH_SESSION_ABSOLUTE_TTL: %w", err)
	}
	if idleTTL < time.Minute || absoluteTTL < idleTTL || absoluteTTL > 30*24*time.Hour {
		return controlplane.BrowserSessionConfig{}, errors.New("session idle/absolute TTLs are outside the allowed range")
	}
	return controlplane.BrowserSessionConfig{
		Enabled: true, PublicOrigin: canonicalOrigin,
		SessionCookie: sessionCookie, CSRFCookie: csrfCookie,
		SecureCookies: secureCookies, IdleTTL: idleTTL, AbsoluteTTL: absoluteTTL,
	}, nil
}

func configuredDuration(value string, fallback time.Duration) (time.Duration, error) {
	value = strings.TrimSpace(value)
	if value == "" {
		return fallback, nil
	}
	duration, err := time.ParseDuration(value)
	if err != nil || duration <= 0 {
		return 0, errors.New("must be a positive Go duration such as 30m or 12h")
	}
	return duration, nil
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
