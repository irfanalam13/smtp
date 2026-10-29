package observability

import (
	"fmt"
	"log/slog"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

const (
	requestIDHeader = "X-Request-ID"
	maxPathLength   = 128
)

type Metrics struct {
	requests        atomic.Uint64
	requestFailures atomic.Uint64
	requestDuration atomic.Uint64
	statuses        sync.Map
}

func NewMetrics() *Metrics {
	return &Metrics{}
}

func (m *Metrics) Observe(status int, duration time.Duration) {
	m.requests.Add(1)
	m.requestDuration.Add(uint64(duration / time.Microsecond))
	if status >= http.StatusInternalServerError {
		m.requestFailures.Add(1)
	}
	value := newCounter(&m.statuses, statusClass(status))
	value.value.Add(1)
}

type counter struct {
	value atomic.Uint64
}

func newCounter(store *sync.Map, key string) *counter {
	if existing, ok := store.Load(key); ok {
		return existing.(*counter)
	}
	created := &counter{}
	actual, _ := store.LoadOrStore(key, created)
	return actual.(*counter)
}

func statusClass(status int) string {
	switch {
	case status >= 500:
		return "5xx"
	case status >= 400:
		return "4xx"
	case status >= 300:
		return "3xx"
	case status >= 200:
		return "2xx"
	default:
		return "1xx"
	}
}

type responseWriter struct {
	http.ResponseWriter
	status int
}

func (w *responseWriter) WriteHeader(status int) {
	w.status = status
	w.ResponseWriter.WriteHeader(status)
}

func (w *responseWriter) Write(body []byte) (int, error) {
	if w.status == 0 {
		w.WriteHeader(http.StatusOK)
	}
	return w.ResponseWriter.Write(body)
}

func (m *Metrics) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		writer := &responseWriter{ResponseWriter: w}
		next.ServeHTTP(writer, r)
		status := writer.status
		if status == 0 {
			status = http.StatusOK
		}
		duration := time.Since(start)
		m.Observe(status, duration)
		slog.Info("http request completed",
			"method", r.Method,
			"path", boundedPath(r),
			"status", status,
			"duration_ms", duration.Milliseconds(),
			"request_id", w.Header().Get(requestIDHeader),
		)
	})
}

func boundedPath(r *http.Request) string {
	path := r.Pattern
	if path == "" {
		path = r.URL.Path
	}
	if len(path) > maxPathLength {
		return path[:maxPathLength]
	}
	return path
}

func (m *Metrics) Handler(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "text/plain; version=0.0.4")
	w.WriteHeader(http.StatusOK)
	_, _ = fmt.Fprintf(w, "# HELP smtp_api_http_requests_total Total HTTP requests.\n")
	_, _ = fmt.Fprintf(w, "# TYPE smtp_api_http_requests_total counter\nsmtp_api_http_requests_total %d\n", m.requests.Load())
	_, _ = fmt.Fprintf(w, "# HELP smtp_api_http_failures_total Total HTTP 5xx responses.\n")
	_, _ = fmt.Fprintf(w, "# TYPE smtp_api_http_failures_total counter\nsmtp_api_http_failures_total %d\n", m.requestFailures.Load())
	_, _ = fmt.Fprintf(w, "# HELP smtp_api_http_request_duration_microseconds_total Sum of request durations.\n")
	_, _ = fmt.Fprintf(w, "# TYPE smtp_api_http_request_duration_microseconds_total counter\nsmtp_api_http_request_duration_microseconds_total %d\n", m.requestDuration.Load())
	m.statuses.Range(func(key, value any) bool {
		c := key.(string)
		counter := value.(*counter)
		_, _ = fmt.Fprintf(w, "smtp_api_http_responses_total{status_class=%q} %d\n", c, counter.value.Load())
		return true
	})
}

func (m *Metrics) RecordStatus(status int) {
	key := statusClass(status)
	value := newCounter(&m.statuses, key)
	value.value.Add(1)
}

func ParseEnabled(value string) bool {
	return strings.EqualFold(strings.TrimSpace(value), "true") ||
		strings.EqualFold(strings.TrimSpace(value), "1") ||
		strings.EqualFold(strings.TrimSpace(value), strconv.FormatBool(true))
}
