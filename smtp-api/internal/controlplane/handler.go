package controlplane

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"net/http"
	"strconv"
	"strings"
	"time"
	"unicode/utf8"

	"example.com/smtp-platform/smtp-api/internal/observability"
)

type Handler struct {
	Store   *Store
	DevMode bool
	TXT     TXTResolver
	DNS     DNSResolver
	Metrics *observability.Metrics
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
func (h Handler) checksRun(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:read")
	if !ok {
		return
	}
	if !h.Store.Allow("dns:"+p.TenantID, 6, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, 429, "rate_limited", "DNS checks are temporarily rate limited")
		return
	}
	resolver := h.DNS
	if resolver == nil {
		resolver = NetDNSResolver{}
	}
	check, err := h.Store.CheckDomain(p, r.PathValue("id"), resolver)
	if err == ErrNotFound {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	if err != nil {
		Error(w, 502, "dns_unavailable", "DNS check could not be completed")
		return
	}
	JSON(w, 200, check)
}
func (h Handler) checksGet(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:read")
	if !ok {
		return
	}
	check, err := h.Store.DomainCheck(p, r.PathValue("id"))
	if err == ErrNotFound {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	JSON(w, 200, check)
}
func (h Handler) readiness(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:read")
	if !ok {
		return
	}
	check, err := h.Store.DomainCheck(p, r.PathValue("id"))
	if err == ErrNotFound {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	if check.Stale || check.CheckedAt.IsZero() {
		JSON(w, 200, map[string]string{"status": "checking"})
		return
	}
	status := "ready"
	for _, result := range check.Results {
		if result.Status == "fail" {
			status = "action_required"
			break
		}
		if result.Status == "pending" || result.Status == "warning" {
			status = "degraded"
		}
	}
	JSON(w, 200, map[string]any{"status": status, "checked_at": check.CheckedAt, "stale": check.Stale})
}
func (h Handler) usage(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	JSON(w, 200, h.Store.Usage(p))
}
func (h Handler) mailboxes(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": h.Store.Mailboxes(p.TenantID), "next_page": nil})
}
func (h Handler) messages(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	items, err := h.Store.Messages(p.TenantID, r.PathValue("mailbox_id"))
	if err == ErrNotFound {
		Error(w, http.StatusNotFound, "not_found", "mailbox not found")
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, 400, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	if len(items) > limit {
		items = items[:limit]
	}
	JSON(w, http.StatusOK, map[string]any{"items": items, "next_page": nil})
}
func (h Handler) message(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	item, err := h.Store.Message(p.TenantID, r.PathValue("mailbox_id"), r.PathValue("message_id"))
	if err == ErrNotFound {
		Error(w, http.StatusNotFound, "not_found", "message not found")
		return
	}
	JSON(w, http.StatusOK, item)
}
func (h Handler) principal(w http.ResponseWriter, r *http.Request, scope string) (Principal, bool) {
	p, err := h.Store.Auth(r, h.DevMode)
	if err != nil {
		Error(w, 401, "unauthorized", "authentication is required")
		return Principal{}, false
	}
	if err := Require(p, scope); err != nil {
		if err == ErrForbidden {
			Error(w, 403, "forbidden", "permission denied")
		} else {
			Error(w, 401, "unauthorized", "authentication is required")
		}
		return Principal{}, false
	}
	if !h.Store.Allow("principal:"+p.TenantID+":"+scope, 120, time.Minute) ||
		!h.Store.Allow("ip:"+r.RemoteAddr+":"+scope, 240, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, 429, "rate_limited", "too many requests")
		return Principal{}, false
	}
	return p, true
}
func (h Handler) me(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	t, _ := h.Store.Tenant(p.TenantID)
	JSON(w, 200, map[string]any{"subject": p.Subject, "user_id": p.UserID, "tenant": t, "role": p.Role})
}
func (h Handler) tenant(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	t, found := h.Store.Tenant(p.TenantID)
	if !found {
		Error(w, 404, "not_found", "tenant not found")
		return
	}
	JSON(w, 200, t)
}
func (h Handler) domains(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:read")
	if !ok {
		return
	}
	items := h.Store.Domains(p.TenantID)
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, 400, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	if len(items) > limit {
		items = items[:limit]
	}
	JSON(w, 200, map[string]any{"items": items, "next_page": nil})
}

func queryLimit(r *http.Request) (int, error) {
	raw := r.URL.Query().Get("limit")
	if raw == "" {
		return 50, nil
	}
	n, err := strconv.Atoi(raw)
	if err != nil || n < 1 || n > 100 {
		return 0, ErrInvalid
	}
	return n, nil
}
func (h Handler) createDomain(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:write")
	if !ok {
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var req struct {
		Domain string `json:"domain"`
	}
	if err := decodeJSON(r, &req); err != nil {
		Error(w, 400, "invalid_request", "invalid JSON request")
		return
	}
	d, err := h.Store.AddDomain(p, req.Domain)
	if err == ErrConflict {
		Error(w, 409, "conflict", "domain already registered")
		return
	}
	if err != nil {
		Error(w, 400, "invalid_domain", "domain is invalid")
		return
	}
	JSON(w, 201, d)
}
func (h Handler) challenge(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:write")
	if !ok {
		return
	}
	token, err := h.Store.Challenge(p, r.PathValue("id"))
	if err == ErrNotFound {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	if err != nil {
		Error(w, 500, "internal_error", "could not create challenge")
		return
	}
	JSON(w, 201, map[string]string{"record_name": "_smtp-platform-verification", "record_type": "TXT", "record_value": "smtp-platform-verification=" + sha256Hex(token), "secret": token})
}
func sha256Hex(v string) string { b := sha256.Sum256([]byte(v)); return hex.EncodeToString(b[:]) }
func (h Handler) check(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:write")
	if !ok {
		return
	}
	resolver := h.TXT
	if resolver == nil {
		resolver = NetTXTResolver{}
	}
	err := h.Store.CheckChallenge(p, r.PathValue("id"), resolver)
	if err == ErrNotFound {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	if err != nil {
		JSON(w, 200, map[string]string{"status": "pending_verification"})
		return
	}
	JSON(w, 200, map[string]string{"status": "verified"})
}
func (h Handler) disableDomain(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:write")
	if !ok {
		return
	}
	if err := h.Store.DisableDomain(p, r.PathValue("id")); err == ErrNotFound {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	JSON(w, 200, map[string]string{"status": "disabled"})
}
func (h Handler) keys(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "keys:write")
	if !ok {
		return
	}
	JSON(w, 200, map[string]any{"items": h.Store.Keys(p)})
}
func (h Handler) createKey(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "keys:write")
	if !ok {
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 8192)
	var req struct {
		Name      string     `json:"name"`
		Scopes    []string   `json:"scopes"`
		ExpiresAt *time.Time `json:"expires_at"`
	}
	if err := decodeJSON(r, &req); err != nil {
		Error(w, 400, "invalid_request", "invalid JSON request")
		return
	}
	key, secret, err := h.Store.CreateKey(p, strings.TrimSpace(req.Name), req.Scopes, req.ExpiresAt)
	if err == ErrForbidden {
		Error(w, 403, "forbidden", "permission denied")
		return
	}
	if err != nil {
		Error(w, 400, "invalid_request", "invalid API-key request")
		return
	}
	JSON(w, 201, map[string]any{"id": key.ID, "name": key.Name, "prefix": key.Prefix, "scopes": key.Scopes, "secret": secret, "warning": "This secret is shown once and cannot be retrieved again."})
}
func (h Handler) revokeKey(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "keys:write")
	if !ok {
		return
	}
	if err := h.Store.RevokeKey(p, r.PathValue("id")); err == ErrNotFound {
		Error(w, 404, "not_found", "API key not found")
		return
	}
	JSON(w, 200, map[string]string{"status": "revoked"})
}
