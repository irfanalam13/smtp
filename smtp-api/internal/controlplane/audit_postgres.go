package controlplane

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
)

func (repository *PostgreSQLRepository) ListAuditEvents(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]AuditEvent, error) {
	if !validUUID(principal.TenantID) {
		return nil, ErrInvalid
	}
	if limit <= 0 || limit > 100 {
		limit = 50
	}

	rows, err := repository.pool.Query(ctx, `
		SELECT id, tenant_id::text, actor_user_id::text, action, target_type, target_id, metadata, created_at
		FROM audit_events
		WHERE tenant_id = $1
		ORDER BY created_at DESC, id DESC
		LIMIT $2
	`, principal.TenantID, limit)
	if err != nil {
		return nil, fmt.Errorf("list audit events: %w", err)
	}
	defer rows.Close()

	var events []AuditEvent
	for rows.Next() {
		var (
			e           AuditEvent
			actorUserID *string
			targetID    *string
			metaBytes   []byte
		)
		if err := rows.Scan(
			&e.ID, &e.TenantID, &actorUserID, &e.Action, &e.TargetType, &targetID, &metaBytes, &e.CreatedAt,
		); err != nil {
			return nil, fmt.Errorf("scan audit event: %w", err)
		}
		e.ActorUserID = actorUserID
		e.TargetID = targetID
		if len(metaBytes) > 0 {
			_ = json.Unmarshal(metaBytes, &e.Metadata)
		}
		if e.Metadata == nil {
			e.Metadata = map[string]any{}
		}
		events = append(events, e)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate audit events: %w", err)
	}
	if events == nil {
		events = []AuditEvent{}
	}
	return events, nil
}

func (h Handler) listAuditEvents(w http.ResponseWriter, r *http.Request) {
	p, ok := h.principal(w, r, "audit:read")
	if !ok {
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	if h.Postgres != nil && p.durable {
		events, err := h.Postgres.ListAuditEvents(r.Context(), p, limit)
		if err != nil {
			h.databaseError(w, "list audit events", err)
			return
		}
		JSON(w, http.StatusOK, map[string]any{"items": events, "next_page": nil})
		return
	}
	if !h.DevMode {
		Error(w, http.StatusServiceUnavailable, "database_unavailable", "database is required")
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": []AuditEvent{}, "next_page": nil})
}
