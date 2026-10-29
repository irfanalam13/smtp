package health

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestHandler(t *testing.T) {
	recorder := httptest.NewRecorder()
	Handler(recorder, httptest.NewRequest(http.MethodGet, "/health/live", nil))

	if recorder.Code != http.StatusOK {
		t.Fatalf("status = %d, want %d", recorder.Code, http.StatusOK)
	}

	if got := recorder.Header().Get("Content-Type"); got != "application/json" {
		t.Fatalf("content type = %q, want application/json", got)
	}
	if got := recorder.Body.String(); got != "{\"status\":\"ok\"}\n" {
		t.Fatalf("body = %q, want JSON health response", got)
	}
}

func TestReadyHandler(t *testing.T) {
	t.Run("ready", func(t *testing.T) {
		recorder := httptest.NewRecorder()
		ReadyHandler(func() bool { return true })(recorder, httptest.NewRequest(http.MethodGet, "/health/ready", nil))
		if recorder.Code != http.StatusOK {
			t.Fatalf("status = %d", recorder.Code)
		}
	})
	t.Run("not ready", func(t *testing.T) {
		recorder := httptest.NewRecorder()
		ReadyHandler(func() bool { return false })(recorder, httptest.NewRequest(http.MethodGet, "/health/ready", nil))
		if recorder.Code != http.StatusServiceUnavailable {
			t.Fatalf("status = %d", recorder.Code)
		}
	})
}
