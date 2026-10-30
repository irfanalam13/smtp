package main

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestRunRequiresDatabaseConfiguration(t *testing.T) {
	t.Setenv("DATABASE_URL", "")
	t.Setenv("API_DATABASE_MAX_CONNECTIONS", "")
	t.Setenv("APP_ENVIRONMENT", "development")
	t.Setenv("CONTROL_PLANE_DEV_MODE", "false")
	t.Setenv("OIDC_ISSUER_URL", "")
	t.Setenv("OIDC_API_AUDIENCE", "")

	err := run()
	if err == nil || !strings.Contains(err.Error(), "DATABASE_URL is required") {
		t.Fatalf("run error=%v, want required DATABASE_URL", err)
	}
}

func TestRunFailsWhenDatabaseIsUnavailable(t *testing.T) {
	t.Setenv("DATABASE_URL", "postgres://127.0.0.1:1/smtp_test?sslmode=disable")
	t.Setenv("API_DATABASE_MAX_CONNECTIONS", "1")
	t.Setenv("APP_ENVIRONMENT", "development")
	t.Setenv("CONTROL_PLANE_DEV_MODE", "false")
	t.Setenv("OIDC_ISSUER_URL", "")
	t.Setenv("OIDC_API_AUDIENCE", "")

	err := run()
	if err == nil || !strings.Contains(err.Error(), "connect API to PostgreSQL") {
		t.Fatalf("run error=%v, want bounded PostgreSQL connection failure", err)
	}
}

func TestRunRejectsDevelopmentIdentityOutsideDevelopmentEnvironment(t *testing.T) {
	for _, environment := range []string{"", "production", "prod", "staging", "test", "prodution"} {
		t.Run(environment, func(t *testing.T) {
			t.Setenv("APP_ENVIRONMENT", environment)
			t.Setenv("CONTROL_PLANE_DEV_MODE", "true")
			t.Setenv("DATABASE_URL", "")
			t.Setenv("OIDC_ISSUER_URL", "")
			t.Setenv("OIDC_API_AUDIENCE", "")

			err := run()
			if err == nil || !strings.Contains(err.Error(), "requires a development environment") {
				t.Fatalf("run error=%v, want development environment rejection", err)
			}
		})
	}
}

func TestRunAllowsDevelopmentIdentityOnlyInDevelopmentEnvironment(t *testing.T) {
	for _, environment := range []string{"development", "dev"} {
		t.Run(environment, func(t *testing.T) {
			t.Setenv("APP_ENVIRONMENT", environment)
			t.Setenv("CONTROL_PLANE_DEV_MODE", "true")
			t.Setenv("DATABASE_URL", "postgres://127.0.0.1:1/smtp_test?sslmode=disable")
			t.Setenv("API_DATABASE_MAX_CONNECTIONS", "1")
			t.Setenv("OIDC_ISSUER_URL", "")
			t.Setenv("OIDC_API_AUDIENCE", "")

			err := run()
			if err == nil || !strings.Contains(err.Error(), "connect API to PostgreSQL") {
				t.Fatalf("run error=%v, want to pass identity guard and fail on database connection", err)
			}
		})
	}
}

func TestNewOIDCVerifierFromEnvRequiresCompleteConfiguration(t *testing.T) {
	tests := []struct {
		name    string
		env     map[string]string
		wantErr bool
	}{
		{name: "disabled", env: map[string]string{}},
		{
			name:    "issuer without audience",
			env:     map[string]string{"OIDC_ISSUER_URL": "https://idp.example.test/realms/smtp"},
			wantErr: true,
		},
		{
			name:    "audience without issuer",
			env:     map[string]string{"OIDC_API_AUDIENCE": "smtp-control-plane"},
			wantErr: true,
		},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			verifier, err := newOIDCVerifierFromEnv(func(key string) string {
				return test.env[key]
			})
			if (err != nil) != test.wantErr {
				t.Fatalf("newOIDCVerifierFromEnv() = (%v, %v), want error=%t", verifier, err, test.wantErr)
			}
		})
	}
}

func TestSecurityHeaders(t *testing.T) {
	handler := securityHeaders(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusNoContent)
	}))
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, httptest.NewRequest(http.MethodGet, "/", nil))

	for header, want := range map[string]string{
		"Cache-Control":           "no-store",
		"Content-Security-Policy": "default-src 'none'; frame-ancestors 'none'",
		"Referrer-Policy":         "no-referrer",
		"X-Content-Type-Options":  "nosniff",
		"X-Frame-Options":         "DENY",
	} {
		if got := recorder.Header().Get(header); got != want {
			t.Errorf("%s = %q, want %q", header, got, want)
		}
	}
}
