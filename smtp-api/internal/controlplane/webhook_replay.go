package controlplane

import (
	"context"
	"errors"
	"net/http"
	"time"

	"github.com/jackc/pgx/v5"
)

type WebhookDeadLetter struct {
	ID             string    `json:"id"`
	SubscriptionID string    `json:"subscription_id"`
	EventID        string    `json:"event_id"`
	EventType      string    `json:"event_type"`
	State          string    `json:"state"`
	AttemptCount   int       `json:"attempt_count"`
	LastError      *string   `json:"last_error,omitempty"`
	CreatedAt      time.Time `json:"created_at"`
}

func (repository *PostgreSQLRepository) ListWebhookDeadLetters(
	ctx context.Context, principal Principal, limit int,
) ([]WebhookDeadLetter, error) {
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) || limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT id::text, subscription_id::text, event_id::text, event_type,
		       state, attempt_count, last_error, created_at
		FROM webhook_deliveries
		WHERE tenant_id = $1 AND state = 'dead_letter'
		ORDER BY created_at DESC
		LIMIT $2
	`, principal.TenantID, limit)
	if err != nil {
		return nil, errors.New("list webhook dead letters: " + err.Error())
	}
	defer rows.Close()
	items := make([]WebhookDeadLetter, 0)
	for rows.Next() {
		var item WebhookDeadLetter
		if err := rows.Scan(&item.ID, &item.SubscriptionID, &item.EventID, &item.EventType,
			&item.State, &item.AttemptCount, &item.LastError, &item.CreatedAt); err != nil {
			return nil, errors.New("scan webhook dead letter: " + err.Error())
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return nil, errors.New("iterate webhook dead letters: " + err.Error())
	}
	return items, nil
}

func (repository *PostgreSQLRepository) ReplayWebhookDelivery(
	ctx context.Context, principal Principal, deliveryID string,
) error {
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) || !validUUID(deliveryID) {
		return ErrInvalid
	}
	tag, err := repository.pool.Exec(ctx, `
		UPDATE webhook_deliveries
		SET state = 'retry', next_attempt_at = NOW(), lease_until = NULL,
		    last_error = NULL
		WHERE tenant_id = $1 AND id = $2 AND state = 'dead_letter'
	`, principal.TenantID, deliveryID)
	if err != nil {
		return errors.New("replay webhook delivery: " + err.Error())
	}
	if tag.RowsAffected() == 0 {
		var exists bool
		if err := repository.pool.QueryRow(ctx, `
			SELECT EXISTS (
				SELECT 1 FROM webhook_deliveries
				WHERE tenant_id = $1 AND id = $2
			)
		`, principal.TenantID, deliveryID).Scan(&exists); err != nil {
			return errors.New("check webhook delivery: " + err.Error())
		}
		if !exists {
			return pgx.ErrNoRows
		}
		return ErrInvalid
	}
	return nil
}

func (h Handler) webhookDeadLetters(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "webhook dead letters require PostgreSQL")
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	items, err := h.Postgres.ListWebhookDeadLetters(r.Context(), principal, limit)
	if err != nil {
		h.databaseError(w, "list webhook dead letters", err)
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": items})
}

func (h Handler) replayWebhook(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "webhook replay requires PostgreSQL")
		return
	}
	err := h.Postgres.ReplayWebhookDelivery(r.Context(), principal, r.PathValue("id"))
	if errors.Is(err, pgx.ErrNoRows) {
		Error(w, http.StatusNotFound, "not_found", "webhook delivery not found")
		return
	}
	if errors.Is(err, ErrInvalid) {
		Error(w, http.StatusConflict, "invalid_state", "webhook delivery is not a dead letter")
		return
	}
	if err != nil {
		h.databaseError(w, "replay webhook delivery", err)
		return
	}
	JSON(w, http.StatusAccepted, map[string]string{"status": "queued"})
}
