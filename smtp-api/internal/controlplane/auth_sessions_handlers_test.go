package controlplane

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestBrowserSessionCookieAttributes(t *testing.T) {
	config := BrowserSessionConfig{
		Enabled: true, PublicOrigin: "https://app.example.test",
		SessionCookie: "__Host-smtp_session", CSRFCookie: "__Host-smtp_csrf",
		SecureCookies: true, IdleTTL: 30 * time.Minute, AbsoluteTTL: 12 * time.Hour,
	}
	if !config.valid() {
		t.Fatal("valid production session configuration was rejected")
	}
	handler := Handler{AuthSessions: config}
	recorder := httptest.NewRecorder()
	handler.setSessionCookies(recorder, "opaque-session-secret", "csrf-verifier", time.Now().Add(time.Hour))
	cookies := recorder.Result().Cookies()
	if len(cookies) != 2 {
		t.Fatalf("cookie count=%d, want two", len(cookies))
	}
	sessionCookie, csrfCookie := cookies[0], cookies[1]
	if sessionCookie.Name != config.SessionCookie || !sessionCookie.HttpOnly ||
		!sessionCookie.Secure || sessionCookie.SameSite != http.SameSiteLaxMode ||
		sessionCookie.Path != "/" || sessionCookie.Domain != "" {
		t.Errorf("unsafe session-cookie attributes: %#v", sessionCookie)
	}
	if csrfCookie.Name != config.CSRFCookie || csrfCookie.HttpOnly ||
		!csrfCookie.Secure || csrfCookie.SameSite != http.SameSiteLaxMode ||
		csrfCookie.Path != "/" || csrfCookie.Domain != "" {
		t.Errorf("unsafe CSRF-cookie attributes: %#v", csrfCookie)
	}
}

func TestBrowserSessionConfigRejectsUnsafeCookieAndOriginSettings(t *testing.T) {
	base := BrowserSessionConfig{
		Enabled: true, PublicOrigin: "https://app.example.test",
		SessionCookie: "smtp_session", CSRFCookie: "__Host-smtp_csrf",
		SecureCookies: true, IdleTTL: time.Minute, AbsoluteTTL: time.Hour,
	}
	if base.valid() {
		t.Fatal("secure session configuration accepted a non-__Host session cookie")
	}
	base.SessionCookie = "__Host-smtp_session"
	base.PublicOrigin = "https://app.example.test/path"
	if base.valid() {
		t.Fatal("session configuration accepted an origin containing a path")
	}
}

func TestSessionMutationRequiresSameOriginAndCSRF(t *testing.T) {
	config := BrowserSessionConfig{
		Enabled: true, PublicOrigin: "https://app.example.test",
		SessionCookie: "__Host-smtp_session", CSRFCookie: "__Host-smtp_csrf",
		SecureCookies: true, IdleTTL: time.Minute, AbsoluteTTL: time.Hour,
	}
	handler := Handler{AuthSessions: config}
	sameOrigin := httptest.NewRequest(http.MethodPost, "/v1/auth/logout-all", nil)
	sameOrigin.Header.Set("Origin", "https://app.example.test")
	if !handler.sameOriginRequest(sameOrigin) {
		t.Fatal("same-origin browser request rejected")
	}
	crossOrigin := httptest.NewRequest(http.MethodPost, "/v1/auth/logout-all", nil)
	crossOrigin.Header.Set("Origin", "https://attacker.example.test")
	crossOrigin.Header.Set("Referer", "https://app.example.test/")
	if handler.sameOriginRequest(crossOrigin) {
		t.Fatal("cross-origin Origin header was overridden by Referer")
	}
}
