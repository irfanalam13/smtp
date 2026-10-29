package observability

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestMetricsMiddlewareAndHandler(t *testing.T) {
	metrics := NewMetrics()
	handler := metrics.Middleware(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusCreated)
	}))
	request := httptest.NewRequest(http.MethodPost, "/v1/domains/secret-id", nil)
	request.Header.Set("X-Request-ID", "request-1")
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusCreated {
		t.Fatalf("status = %d", recorder.Code)
	}
	output := httptest.NewRecorder()
	metrics.Handler(output, request)
	body := output.Body.String()
	if !strings.Contains(body, "smtp_api_http_requests_total 1") ||
		!strings.Contains(body, `status_class="2xx"`) {
		t.Fatalf("unexpected metrics output: %s", body)
	}
	if strings.Contains(body, "secret-id") {
		t.Fatal("metrics must not expose resource identifiers")
	}
}

func TestParseEnabled(t *testing.T) {
	for _, value := range []string{"true", "TRUE", "1"} {
		if !ParseEnabled(value) {
			t.Fatalf("expected enabled for %q", value)
		}
	}
	if ParseEnabled("false") {
		t.Fatal("false must remain disabled")
	}
}
