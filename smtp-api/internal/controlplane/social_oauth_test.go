package controlplane

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestSocialOAuthStartDevMode(t *testing.T) {
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

	// Test Google Start in Dev Mode
	req := httptest.NewRequest(http.MethodGet, "http://127.0.0.1:3000/v1/auth/oauth/google/start", nil)
	req.Header.Set("Origin", "http://127.0.0.1:3000")
	recorder := httptest.NewRecorder()

	handler.startGoogleOAuth(recorder, req)

	resp := recorder.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200 OK for dev google oauth start, got %d", resp.StatusCode)
	}
	body := recorder.Body.String()
	if !strings.Contains(body, "Sign in with Google") {
		t.Errorf("expected body to contain 'Sign in with Google', got %s", body)
	}

	// Test GitHub Start in Dev Mode
	reqGH := httptest.NewRequest(http.MethodGet, "http://localhost:3000/v1/auth/oauth/github/start", nil)
	reqGH.Header.Set("Origin", "http://localhost:3000")
	recorderGH := httptest.NewRecorder()

	handler.startGitHubOAuth(recorderGH, reqGH)

	respGH := recorderGH.Result()
	if respGH.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200 OK for dev github oauth start, got %d", respGH.StatusCode)
	}
	bodyGH := recorderGH.Body.String()
	if !strings.Contains(bodyGH, "Sign in with GitHub") {
		t.Errorf("expected body to contain 'Sign in with GitHub', got %s", bodyGH)
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
