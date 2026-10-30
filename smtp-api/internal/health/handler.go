package health

import (
	"encoding/json"
	"net/http"
	"sync/atomic"
)

type Readiness struct {
	draining atomic.Bool
}

func (r *Readiness) Drain() {
	r.draining.Store(true)
}

func (r *Readiness) Handler(check func() bool) http.HandlerFunc {
	return ReadyHandler(func() bool {
		return !r.draining.Load() && check != nil && check()
	})
}

func Handler(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	_ = json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}

func ReadyHandler(check func() bool) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		status := http.StatusOK
		body := map[string]string{"status": "ready"}
		if check == nil || !check() {
			status = http.StatusServiceUnavailable
			body["status"] = "not_ready"
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(status)
		_ = json.NewEncoder(w).Encode(body)
	}
}
