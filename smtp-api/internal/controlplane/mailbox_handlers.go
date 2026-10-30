package controlplane

import (
	"errors"
	"net/http"
)

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
	if !h.requireDurableMailboxPrincipal(w, p) {
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	if h.Postgres != nil {
		items, err := h.Postgres.ListMailboxes(r.Context(), p, limit)
		if errors.Is(err, ErrForbidden) {
			Error(w, http.StatusForbidden, "forbidden", "mailbox access requires a user-bound API key")
			return
		}
		if err != nil {
			h.databaseError(w, "list mailboxes", err)
			return
		}
		JSON(w, http.StatusOK, map[string]any{"items": items, "next_page": nil})
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": h.Store.Mailboxes(p.TenantID, limit), "next_page": nil})
}

func (h Handler) messages(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	if !h.requireDurableMailboxPrincipal(w, p) {
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, 400, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	if h.Postgres != nil {
		items, err := h.Postgres.ListMessages(
			r.Context(), p, r.PathValue("mailbox_id"), limit,
		)
		if errors.Is(err, ErrForbidden) {
			Error(w, http.StatusForbidden, "forbidden", "mailbox access requires a user-bound API key")
			return
		}
		if errors.Is(err, ErrInvalid) || errors.Is(err, ErrNotFound) {
			Error(w, http.StatusNotFound, "not_found", "mailbox not found")
			return
		}
		if err != nil {
			h.databaseError(w, "list mailbox messages", err)
			return
		}
		JSON(w, http.StatusOK, map[string]any{"items": items, "next_page": nil})
		return
	}
	items, err := h.Store.Messages(p.TenantID, r.PathValue("mailbox_id"))
	if err == ErrNotFound {
		Error(w, http.StatusNotFound, "not_found", "mailbox not found")
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
	if !h.requireDurableMailboxPrincipal(w, p) {
		return
	}
	if h.Postgres != nil {
		item, err := h.Postgres.GetMessage(
			r.Context(), p, r.PathValue("mailbox_id"), r.PathValue("message_id"),
		)
		if errors.Is(err, ErrForbidden) {
			Error(w, http.StatusForbidden, "forbidden", "mailbox access requires a user-bound API key")
			return
		}
		if errors.Is(err, ErrNotFound) {
			Error(w, http.StatusNotFound, "not_found", "message not found")
			return
		}
		if err != nil {
			h.databaseError(w, "get mailbox message", err)
			return
		}
		JSON(w, http.StatusOK, item)
		return
	}
	item, err := h.Store.Message(p.TenantID, r.PathValue("mailbox_id"), r.PathValue("message_id"))
	if err == ErrNotFound {
		Error(w, http.StatusNotFound, "not_found", "message not found")
		return
	}
	JSON(w, http.StatusOK, item)
}
