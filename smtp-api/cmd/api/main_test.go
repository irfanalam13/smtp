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

func TestBrowserSessionConfigRequiresSecureProductionOrigin(t *testing.T) {
	tests := []struct {
		name string
		env  map[string]string
		want bool
	}{
		{
			name: "development defaults",
			env:  map[string]string{"APP_ENVIRONMENT": "development"},
			want: true,
		},
		{
			name: "development HTTPS uses secure cookies",
			env: map[string]string{
				"APP_ENVIRONMENT":   "development",
				"APP_PUBLIC_ORIGIN": "https://localhost:3210",
			},
			want: true,
		},
		{
			name: "production secure defaults",
			env: map[string]string{
				"APP_ENVIRONMENT":   "production",
				"APP_PUBLIC_ORIGIN": "https://app.example.test",
			},
			want: true,
		},
		{
			name: "production rejects insecure origin",
			env: map[string]string{
				"APP_ENVIRONMENT":   "production",
				"APP_PUBLIC_ORIGIN": "http://app.example.test",
			},
		},
		{
			name: "staging rejects insecure cookie prefix",
			env: map[string]string{
				"APP_ENVIRONMENT":          "staging",
				"APP_PUBLIC_ORIGIN":        "https://stage.example.test",
				"AUTH_SESSION_COOKIE_NAME": "smtp_session",
			},
		},
		{
			name: "non-development requires canonical origin",
			env:  map[string]string{"APP_ENVIRONMENT": "staging"},
		},
		{
			name: "development only permits loopback http",
			env: map[string]string{
				"APP_ENVIRONMENT":   "development",
				"APP_PUBLIC_ORIGIN": "http://app.example.test",
			},
		},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			config, err := browserSessionConfigFromEnv(func(key string) string {
				return test.env[key]
			})
			if (err == nil) != test.want {
				t.Fatalf("browserSessionConfigFromEnv() = (%#v, %v), want success=%t", config, err, test.want)
			}
			if err == nil {
				wantSecure := test.env["APP_ENVIRONMENT"] != "development" &&
					test.env["APP_ENVIRONMENT"] != "dev" ||
					strings.HasPrefix(test.env["APP_PUBLIC_ORIGIN"], "https://")
				if config.SecureCookies != wantSecure {
					t.Fatalf("SecureCookies=%t, want %t", config.SecureCookies, wantSecure)
				}
			}
		})
	}
}

func TestOIDCLoginConfigUsesOnlyTheImplementedSameOriginCallback(t *testing.T) {
	disabled, err := newOIDCLoginFromEnv(func(string) string { return "" }, "https://app.example.test")
	if err != nil || disabled != nil {
		t.Fatalf("unset OIDC login config = (%v, %v), want disabled", disabled, err)
	}
	_, err = newOIDCLoginFromEnv(func(key string) string {
		switch key {
		case "OIDC_ISSUER_URL":
			return "https://idp.example.test"
		case "OIDC_CLIENT_ID":
			return "smtp-dashboard"
		case "OIDC_CLIENT_SECRET":
			return "test-secret"
		case "OIDC_REDIRECT_URI":
			return "https://app.example.test/unimplemented-callback"
		default:
			return ""
		}
	}, "https://app.example.test")
	if err == nil {
		t.Fatal("OIDC login accepted a callback URL that does not match the implemented route")
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
