package controlplane

import (
	"errors"
	"net/http"
)

func (h Handler) me(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	var t Tenant
	if h.Postgres != nil && p.durable {
		var err error
		t, err = h.Postgres.Tenant(r.Context(), p)
		if err != nil {
			if errors.Is(err, ErrNotFound) {
				Error(w, http.StatusUnauthorized, "unauthorized", "authentication is required")
			} else {
				h.databaseError(w, "resolve current tenant", err)
			}
			return
		}
	} else {
		t, _ = h.Store.Tenant(p.TenantID)
	}
	JSON(w, 200, map[string]any{"subject": p.Subject, "user_id": p.UserID, "tenant": t, "role": p.Role})
}

func (h Handler) tenant(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	var (
		t     Tenant
		found bool
		err   error
	)
	if h.Postgres != nil && p.durable {
		t, err = h.Postgres.Tenant(r.Context(), p)
		found = err == nil
	} else {
		t, found = h.Store.Tenant(p.TenantID)
	}
	if errors.Is(err, ErrNotFound) || !found {
		Error(w, 404, "not_found", "tenant not found")
		return
	}
	if err != nil {
		h.databaseError(w, "resolve current tenant", err)
		return
	}
	JSON(w, 200, t)
}
