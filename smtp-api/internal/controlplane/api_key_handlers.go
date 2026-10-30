package controlplane

import (
	"errors"
	"net/http"
	"strings"
	"time"
)

func (h Handler) keys(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "keys:write")
	if !ok {
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, 400, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	if h.Postgres != nil && isDatabasePrincipal(p) {
		items, err := h.Postgres.ListAPIKeys(r.Context(), p, limit)
		if err != nil {
			h.databaseError(w, "list API keys", err)
			return
		}
		JSON(w, 200, map[string]any{"items": items})
		return
	}
	JSON(w, 200, map[string]any{"items": h.Store.Keys(p, limit)})
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
	var (
		key    APIKey
		secret string
		err    error
	)
	if h.Postgres != nil && isDatabasePrincipal(p) {
		key, secret, err = h.Postgres.CreateAPIKey(
			r.Context(), p, strings.TrimSpace(req.Name), req.Scopes, req.ExpiresAt,
		)
	} else {
		key, secret, err = h.Store.CreateKey(p, strings.TrimSpace(req.Name), req.Scopes, req.ExpiresAt)
	}
	if err != nil && !errors.Is(err, ErrForbidden) && !errors.Is(err, ErrInvalid) {
		if errors.Is(err, ErrConflict) {
			Error(w, 409, "conflict", "an API key with this name already exists")
			return
		}
		h.databaseError(w, "create API key", err)
		return
	}
	if err == ErrForbidden {
		Error(w, 403, "forbidden", "permission denied")
		return
	}
	if err == ErrInvalid {
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
	var err error
	if h.Postgres != nil && isDatabasePrincipal(p) {
		err = h.Postgres.RevokeAPIKey(r.Context(), p, r.PathValue("id"))
	} else {
		err = h.Store.RevokeKey(p, r.PathValue("id"))
	}
	if err == ErrNotFound {
		Error(w, 404, "not_found", "API key not found")
		return
	}
	if err != nil {
		h.databaseError(w, "revoke API key", err)
		return
	}
	JSON(w, 200, map[string]string{"status": "revoked"})
}
