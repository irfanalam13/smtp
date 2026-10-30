package observability

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
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
		!strings.Contains(body, `status_class="2xx"`) ||
		!strings.Contains(body, "smtp_api_http_active_requests 0") ||
		!strings.Contains(body, `smtp_api_http_request_duration_seconds_bucket{le="+Inf"} 1`) ||
		!strings.Contains(body, "smtp_api_http_request_duration_seconds_count 1") {
		t.Fatalf("unexpected metrics output: %s", body)
	}
	if strings.Contains(body, "secret-id") {
		t.Fatal("metrics must not expose resource identifiers")
	}
}

func TestMetricsTrackActiveRequests(t *testing.T) {
	metrics := NewMetrics()
	handler := metrics.Middleware(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		output := httptest.NewRecorder()
		metrics.Handler(output, httptest.NewRequest(http.MethodGet, "/metrics", nil))
		if !strings.Contains(output.Body.String(), "smtp_api_http_active_requests 1") {
			t.Fatalf("active request gauge missing while handler is running: %s", output.Body.String())
		}
		w.WriteHeader(http.StatusNoContent)
	}))
	handler.ServeHTTP(httptest.NewRecorder(), httptest.NewRequest(http.MethodGet, "/v1/usage", nil))

	output := httptest.NewRecorder()
	metrics.Handler(output, httptest.NewRequest(http.MethodGet, "/metrics", nil))
	if !strings.Contains(output.Body.String(), "smtp_api_http_active_requests 0") {
		t.Fatalf("active request gauge did not return to zero: %s", output.Body.String())
	}
}

func TestMetricsObserveDurationHistogramCumulatively(t *testing.T) {
	metrics := NewMetrics()
	metrics.Observe(http.StatusOK, 20*time.Millisecond)
	output := httptest.NewRecorder()
	metrics.Handler(output, httptest.NewRequest(http.MethodGet, "/metrics", nil))
	body := output.Body.String()
	for _, sample := range []string{
		`smtp_api_http_request_duration_seconds_bucket{le="0.005"} 0`,
		`smtp_api_http_request_duration_seconds_bucket{le="0.025"} 1`,
		`smtp_api_http_request_duration_seconds_bucket{le="10"} 1`,
		`smtp_api_http_request_duration_seconds_sum 0.02`,
		"smtp_api_http_request_duration_seconds_count 1",
	} {
		if !strings.Contains(body, sample) {
			t.Errorf("missing histogram sample %q in:\n%s", sample, body)
		}
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
