package controlplane

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestSocialOAuthStartFailsClosedWithoutDurableFlowStore(t *testing.T) {
	config := BrowserSessionConfig{
		Enabled:       true,
		PublicOrigin:  "http://127.0.0.1:3000",
		SessionCookie: "smtp_session",
		CSRFCookie:    "smtp_csrf",
		SecureCookies: false,
		IdleTTL:       30 * time.Minute,
		AbsoluteTTL:   12 * time.Hour,
	}
	handler := Handler{
		AuthSessions: config,
		DevMode:      true,
	}

	// A development UI must not create an OAuth transaction that cannot be
	// consumed safely. The callback also needs PostgreSQL, so accepting the
	// start request without it used to create a misleading fake sign-in path.
	req := httptest.NewRequest(http.MethodGet, "http://127.0.0.1:3000/v1/auth/oauth/google/start", nil)
	req.Header.Set("Origin", "http://127.0.0.1:3000")
	recorder := httptest.NewRecorder()

	handler.startGoogleOAuth(recorder, req)

	resp := recorder.Result()
	if resp.StatusCode != http.StatusServiceUnavailable {
		t.Fatalf("expected status 503 without durable flow store, got %d", resp.StatusCode)
	}
	if strings.Contains(recorder.Body.String(), "Sign in with Google") {
		t.Fatal("Google development OAuth page was exposed without durable flow storage")
	}

	// Test GitHub Start in Dev Mode
	reqGH := httptest.NewRequest(http.MethodGet, "http://localhost:3000/v1/auth/oauth/github/start", nil)
	reqGH.Header.Set("Origin", "http://localhost:3000")
	recorderGH := httptest.NewRecorder()

	handler.startGitHubOAuth(recorderGH, reqGH)

	respGH := recorderGH.Result()
	if respGH.StatusCode != http.StatusServiceUnavailable {
		t.Fatalf("expected status 503 without durable flow store, got %d", respGH.StatusCode)
	}
	if strings.Contains(recorderGH.Body.String(), "Sign in with GitHub") {
		t.Fatal("GitHub development OAuth page was exposed without durable flow storage")
	}
}

func TestSameOriginRequestAllowsLoopbackInDev(t *testing.T) {
	config := BrowserSessionConfig{
		Enabled:       true,
		PublicOrigin:  "http://127.0.0.1:3000",
		SessionCookie: "smtp_session",
		CSRFCookie:    "smtp_csrf",
		SecureCookies: false,
		IdleTTL:       30 * time.Minute,
		AbsoluteTTL:   12 * time.Hour,
	}
	handler := Handler{
		AuthSessions: config,
		DevMode:      true,
	}

	// localhost with same port
	reqLocalhost := httptest.NewRequest(http.MethodPost, "http://127.0.0.1:8080/v1/auth/login", nil)
	reqLocalhost.Header.Set("Origin", "http://localhost:3000")
	if !handler.sameOriginRequest(reqLocalhost) {
		t.Errorf("sameOriginRequest rejected localhost:3000 in dev mode")
	}

	// 127.0.0.1 with 3210
	req3210 := httptest.NewRequest(http.MethodPost, "http://127.0.0.1:8080/v1/auth/login", nil)
	req3210.Header.Set("Origin", "http://127.0.0.1:3210")
	if !handler.sameOriginRequest(req3210) {
		t.Errorf("sameOriginRequest rejected 127.0.0.1:3210 in dev mode")
	}

	// Disallowed external domain
	reqExternal := httptest.NewRequest(http.MethodPost, "http://127.0.0.1:8080/v1/auth/login", nil)
	reqExternal.Header.Set("Origin", "https://malicious-site.com")
	if handler.sameOriginRequest(reqExternal) {
		t.Errorf("sameOriginRequest accepted external origin in dev mode")
	}
}
