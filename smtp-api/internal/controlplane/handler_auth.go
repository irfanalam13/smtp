package controlplane

import (
	"errors"
	"example.com/smtp-platform/smtp-api/internal/identity"
	"log/slog"
	"net/http"
	"strings"
	"time"
)

func (h Handler) principal(w http.ResponseWriter, r *http.Request, scope string) (Principal, bool) {
	var (
		p   Principal
		err error
	)
	authorization := r.Header.Get("Authorization")
	if strings.HasPrefix(authorization, "Bearer ") {
		token := strings.TrimSpace(strings.TrimPrefix(authorization, "Bearer "))
		if strings.HasPrefix(token, "sp_") {
			if h.Postgres != nil {
				p, err = h.Postgres.AuthenticateAPIKey(r.Context(), token)
			} else {
				p, err = h.Store.Auth(r, h.DevMode)
			}
		} else if h.OIDC != nil && h.Postgres != nil {
			var claims identity.Claims
			claims, err = h.OIDC.Verify(r.Context(), token)
			if err == nil {
				p, err = h.Postgres.AuthenticateOIDCSubject(
					r.Context(), claims.Issuer, claims.Subject, r.Header.Get("X-Tenant-ID"),
				)
			} else {
				err = ErrUnauthorized
			}
		} else {
			err = ErrUnauthorized
		}
	} else {
		p, err = h.Store.Auth(r, h.DevMode)
	}
	if err != nil {
		if errors.Is(err, ErrTenantSelectionRequired) {
			Error(w, http.StatusBadRequest, "tenant_selection_required", "specify a tenant with the X-Tenant-ID header")
			return Principal{}, false
		}
		if errors.Is(err, ErrUnauthorized) {
			Error(w, 401, "unauthorized", "authentication is required")
		} else {
			h.databaseError(w, "authenticate request", err)
		}
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

func isDatabasePrincipal(principal Principal) bool {
	return principal.durable
}

func (h Handler) requireDurableMailboxPrincipal(w http.ResponseWriter, principal Principal) bool {
	if h.Postgres == nil {
		return true
	}
	if !principal.durable {
		Error(
			w,
			http.StatusUnauthorized,
			"unauthorized",
			"mailbox reads require a user-bound API key or configured OIDC access token; the local development identity cannot access PostgreSQL mailboxes",
		)
		return false
	}
	if !validMailboxPrincipal(principal) {
		Error(w, http.StatusForbidden, "forbidden", "mailbox access requires a user-bound API key")
		return false
	}
	return true
}

func (h Handler) databaseError(w http.ResponseWriter, operation string, err error) {
	slog.Error("control-plane database operation failed",
		"operation", operation,
		"request_id", w.Header().Get(requestIDHeader),
		"error", err,
	)
	Error(w, http.StatusServiceUnavailable, "service_unavailable", "requested data is temporarily unavailable")
}
