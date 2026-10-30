package controlplane

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"example.com/smtp-platform/smtp-api/internal/identity"
	"example.com/smtp-platform/smtp-api/internal/observability"
	"fmt"
	"net/http"
	"strings"
	"time"
	"unicode/utf8"
)

type OIDCVerifier interface {
	Verify(context.Context, string) (identity.Claims, error)
}

type Handler struct {
	Store    *Store
	Postgres *PostgreSQLRepository
	OIDC     OIDCVerifier
	DevMode  bool
	TXT      TXTResolver
	DNS      DNSResolver
	Metrics  *observability.Metrics
}

func (h Handler) Routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /v1/me", h.me)
	mux.HandleFunc("GET /v1/tenants/current", h.tenant)
	mux.HandleFunc("GET /v1/domains", h.domains)
	mux.HandleFunc("POST /v1/domains", h.createDomain)
	mux.HandleFunc("POST /v1/domains/{id}/verification", h.challenge)
	mux.HandleFunc("POST /v1/domains/{id}/verification/check", h.check)
	mux.HandleFunc("POST /v1/domains/{id}/checks", h.checksRun)
	mux.HandleFunc("GET /v1/domains/{id}/checks", h.checksGet)
	mux.HandleFunc("GET /v1/domains/{id}/readiness", h.readiness)
	mux.HandleFunc("GET /v1/usage", h.usage)
	mux.HandleFunc("POST /v1/submissions", h.submit)
	mux.HandleFunc("GET /v1/submissions", h.submissions)
	mux.HandleFunc("GET /v1/submissions/{id}/attempts", h.submissionAttempts)
	mux.HandleFunc("DELETE /v1/domains/{id}", h.disableDomain)
	mux.HandleFunc("GET /v1/api-keys", h.keys)
	mux.HandleFunc("POST /v1/api-keys", h.createKey)
	mux.HandleFunc("POST /v1/api-keys/{id}/revoke", h.revokeKey)
	mux.HandleFunc("GET /v1/mailboxes", h.mailboxes)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/messages", h.messages)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/messages/{message_id}", h.message)
	if h.Metrics == nil {
		h.Metrics = observability.NewMetrics()
	}
	return requestContext(h.Metrics.Middleware(mux))
}

const requestIDHeader = "X-Request-ID"

func requestContext(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestID := strings.TrimSpace(r.Header.Get(requestIDHeader))
		if !validRequestID(requestID) {
			requestID = newRequestID()
		}
		w.Header().Set(requestIDHeader, requestID)
		next.ServeHTTP(w, r)
	})
}

func validRequestID(value string) bool {
	if value == "" || len(value) > 64 || !utf8.ValidString(value) {
		return false
	}
	for _, r := range value {
		if !(r == '-' || r == '_' || r == '.' ||
			r >= '0' && r <= '9' || r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z') {
			return false
		}
	}
	return true
}

func newRequestID() string {
	var raw [16]byte
	if _, err := rand.Read(raw[:]); err != nil {
		return fmt.Sprintf("request-%d", time.Now().UnixNano())
	}
	return hex.EncodeToString(raw[:])
}
