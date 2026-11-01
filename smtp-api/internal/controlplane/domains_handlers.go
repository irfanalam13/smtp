package controlplane

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"net/http"
	"strconv"
	"time"
)

func (h Handler) checksRun(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:read")
	if !ok {
		return
	}
	if !h.allow(r.Context(), "dns:"+p.TenantID, 6, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, 429, "rate_limited", "DNS checks are temporarily rate limited")
		return
	}
	resolver := h.DNS
	if resolver == nil {
		resolver = NetDNSResolver{}
	}
	if h.Postgres != nil && isDatabasePrincipal(p) {
		domain, err := h.Postgres.GetDomain(r.Context(), p, r.PathValue("id"))
		if errors.Is(err, ErrNotFound) {
			Error(w, 404, "not_found", "domain not found")
			return
		}
		if err != nil {
			h.databaseError(w, "load domain for DNS checks", err)
			return
		}
		check := inspectDomain(domain, resolver)
		h.Store.cacheDomainCheck(check)
		JSON(w, 200, check)
		return
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
	if h.Postgres != nil && isDatabasePrincipal(p) {
		if _, err := h.Postgres.GetDomain(r.Context(), p, r.PathValue("id")); errors.Is(err, ErrNotFound) {
			Error(w, 404, "not_found", "domain not found")
			return
		} else if err != nil {
			h.databaseError(w, "load domain check state", err)
			return
		}
		check, found := h.Store.cachedDomainCheck(r.PathValue("id"))
		if !found {
			check = DomainCheck{DomainID: r.PathValue("id"), Stale: true}
		}
		JSON(w, 200, check)
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
	if h.Postgres != nil && isDatabasePrincipal(p) {
		_, err := h.Postgres.GetDomain(r.Context(), p, r.PathValue("id"))
		if errors.Is(err, ErrNotFound) {
			Error(w, 404, "not_found", "domain not found")
			return
		}
		if err != nil {
			h.databaseError(w, "load domain readiness", err)
			return
		}
		check, found := h.Store.cachedDomainCheck(r.PathValue("id"))
		if !found {
			check = DomainCheck{DomainID: r.PathValue("id"), Stale: true}
		}
		JSON(w, 200, readinessResponse(check))
		return
	}
	check, err := h.Store.DomainCheck(p, r.PathValue("id"))
	if err == ErrNotFound {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	JSON(w, 200, readinessResponse(check))
}

func readinessResponse(check DomainCheck) map[string]any {
	if check.Stale || check.CheckedAt.IsZero() {
		return map[string]any{"status": "checking"}
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
	return map[string]any{"status": status, "checked_at": check.CheckedAt, "stale": check.Stale}
}

func (h Handler) domains(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:read")
	if !ok {
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, 400, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	if h.Postgres != nil && isDatabasePrincipal(p) {
		items, err := h.Postgres.ListDomains(r.Context(), p, limit)
		if err != nil {
			h.databaseError(w, "list domains", err)
			return
		}
		JSON(w, 200, map[string]any{"items": items, "next_page": nil})
		return
	}
	items := h.Store.Domains(p.TenantID)
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
	var (
		d   Domain
		err error
	)
	if h.Postgres != nil && isDatabasePrincipal(p) {
		d, err = h.Postgres.CreateDomain(r.Context(), p, req.Domain)
	} else {
		d, err = h.Store.AddDomain(p, req.Domain)
	}
	if errors.Is(err, ErrConflict) {
		Error(w, 409, "conflict", "domain already registered")
		return
	}
	if errors.Is(err, ErrInvalid) {
		Error(w, 400, "invalid_domain", "domain is invalid")
		return
	}
	if err != nil {
		h.databaseError(w, "create domain", err)
		return
	}
	JSON(w, 201, d)
}

func (h Handler) challenge(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:write")
	if !ok {
		return
	}
	if h.Postgres != nil && isDatabasePrincipal(p) {
		challenge, err := h.Postgres.CreateDomainChallenge(r.Context(), p, r.PathValue("id"))
		if errors.Is(err, ErrNotFound) {
			Error(w, 404, "not_found", "domain not found")
			return
		}
		if err != nil {
			h.databaseError(w, "create domain verification challenge", err)
			return
		}
		JSON(w, 201, challenge)
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
	var err error
	if h.Postgres != nil && isDatabasePrincipal(p) {
		err = h.Postgres.CheckDomainChallenge(r.Context(), p, r.PathValue("id"), resolver)
	} else {
		err = h.Store.CheckChallenge(p, r.PathValue("id"), resolver)
	}
	if errors.Is(err, ErrNotFound) {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	if errors.Is(err, ErrVerificationPending) {
		JSON(w, 200, map[string]string{"status": "pending_verification"})
		return
	}
	if err != nil {
		h.databaseError(w, "check domain verification", err)
		return
	}
	if h.Postgres != nil && isDatabasePrincipal(p) {
		h.Store.invalidateDomainCheck(r.PathValue("id"))
	}
	JSON(w, 200, map[string]string{"status": "verified"})
}

func (h Handler) disableDomain(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "domains:write")
	if !ok {
		return
	}
	var err error
	if h.Postgres != nil && isDatabasePrincipal(p) {
		err = h.Postgres.DisableDomain(r.Context(), p, r.PathValue("id"))
	} else {
		err = h.Store.DisableDomain(p, r.PathValue("id"))
	}
	if errors.Is(err, ErrNotFound) {
		Error(w, 404, "not_found", "domain not found")
		return
	}
	if err != nil {
		h.databaseError(w, "disable domain", err)
		return
	}
	if h.Postgres != nil && isDatabasePrincipal(p) {
		h.Store.invalidateDomainCheck(r.PathValue("id"))
	}
	JSON(w, 200, map[string]string{"status": "disabled"})
}
