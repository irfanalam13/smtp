package controlplane

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestNormalizeEmailAndPasswordPolicy(t *testing.T) {
	email, ok := normalizeEmail("Example.User@example.test")
	if !ok || email != "example.user@example.test" {
		t.Fatalf("normalizeEmail() = (%q, %t)", email, ok)
	}
	for _, invalid := range []string{
		"", "not-an-email", " user@example.test", "User <user@example.test>",
		"user@example.test\r\nBcc: other@example.test",
	} {
		if _, ok := normalizeEmail(invalid); ok {
			t.Errorf("normalizeEmail accepted %q", invalid)
		}
	}
	if !validPassword("a sufficiently long password") ||
		validPassword("short") || validPassword("a\npassword with newline") {
		t.Fatal("password policy accepted an invalid password or rejected a valid one")
	}
}

func TestPasswordMembershipSelectionIsTenantScoped(t *testing.T) {
	memberships := []PasswordMembership{
		{UserID: "123e4567-e89b-12d3-a456-426614174000", TenantID: "123e4567-e89b-12d3-a456-426614174001", Role: Owner},
		{UserID: "123e4567-e89b-12d3-a456-426614174000", TenantID: "123e4567-e89b-12d3-a456-426614174002", Role: Member},
	}
	if _, err := selectPasswordMembership(memberships, ""); err != ErrTenantSelectionRequired {
		t.Fatalf("missing tenant selection error = %v", err)
	}
	principal, err := selectPasswordMembership(memberships, memberships[1].TenantID)
	if err != nil || principal.TenantID != memberships[1].TenantID || principal.Role != Member {
		t.Fatalf("selected principal = (%#v, %v)", principal, err)
	}
	if _, err := selectPasswordMembership(memberships, "123e4567-e89b-12d3-a456-426614174099"); err != ErrUnauthorized {
		t.Fatalf("cross-tenant selection error = %v", err)
	}
}

func TestAuthProvidersExposeOnlyConfiguredCapabilities(t *testing.T) {
	handler := Handler{Store: NewStore()}
	recorder := httptest.NewRecorder()
	handler.Routes().ServeHTTP(recorder, httptest.NewRequest(http.MethodGet, "/v1/auth/providers", nil))
	if recorder.Code != http.StatusOK {
		t.Fatalf("provider status=%d", recorder.Code)
	}
	for _, field := range []string{
		`"oidc_enabled":false`,
		`"password_login_enabled":false`,
		`"signup_enabled":false`,
		`"password_reset_enabled":false`,
	} {
		if !strings.Contains(recorder.Body.String(), field) {
			t.Errorf("provider response missing %s: %s", field, recorder.Body.String())
		}
	}
}

func TestSignUpFailsClosedWhenEmailDeliveryIsUnconfigured(t *testing.T) {
	handler := Handler{
		Store: NewStore(),
		AuthSessions: BrowserSessionConfig{
			Enabled: true, PublicOrigin: "http://127.0.0.1:3210",
			SessionCookie: "smtp_session", CSRFCookie: "smtp_csrf",
			IdleTTL: time.Minute, AbsoluteTTL: time.Hour,
		},
	}
	recorder := httptest.NewRecorder()
	handler.Routes().ServeHTTP(recorder, httptest.NewRequest(
		http.MethodPost, "/v1/auth/signup", strings.NewReader(`{}`),
	))
	if recorder.Code != http.StatusServiceUnavailable {
		t.Fatalf("signup status=%d, want disabled configuration to fail closed", recorder.Code)
	}
}
