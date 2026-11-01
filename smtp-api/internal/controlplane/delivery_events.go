package controlplane

import (
	"context"
	"errors"
	"net/http"
	"time"
)

type DeliveryEvent struct {
	MessageID     string     `json:"message_id"`
	Recipient     string     `json:"recipient"`
	State         string     `json:"state"`
	AttemptNumber int        `json:"attempt_number"`
	SMTPCode      *int       `json:"smtp_code,omitempty"`
	Diagnostic    *string    `json:"diagnostic,omitempty"`
	RemoteHost    *string    `json:"remote_host,omitempty"`
	EventAt       time.Time  `json:"event_at"`
	FinishedAt    *time.Time `json:"finished_at,omitempty"`
}

func (repository *PostgreSQLRepository) ListDeliveryEvents(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]DeliveryEvent, error) {
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) ||
		limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT delivery.message_id::text, recipient.recipient,
		       delivery.state, attempt.attempt_number, attempt.smtp_code,
		       attempt.diagnostic, attempt.remote_host, attempt.started_at,
		       attempt.finished_at
		FROM mail_recipient_attempts AS attempt
		JOIN mail_recipient_deliveries AS delivery
		  ON delivery.id = attempt.delivery_id
		JOIN mail_recipients AS recipient
		  ON recipient.id = delivery.recipient_id
		JOIN mail_messages AS message
		  ON message.id = delivery.message_id
		WHERE message.tenant_id = $1
		ORDER BY attempt.started_at DESC, attempt.id DESC
		LIMIT $2
	`, principal.TenantID, limit)
	if err != nil {
		return nil, errors.New("list delivery events: " + err.Error())
	}
	defer rows.Close()

	items := make([]DeliveryEvent, 0)
	for rows.Next() {
		var item DeliveryEvent
		if err := rows.Scan(
			&item.MessageID, &item.Recipient, &item.State,
			&item.AttemptNumber, &item.SMTPCode, &item.Diagnostic,
			&item.RemoteHost, &item.EventAt, &item.FinishedAt,
		); err != nil {
			return nil, errors.New("scan delivery event: " + err.Error())
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return nil, errors.New("iterate delivery events: " + err.Error())
	}
	return items, nil
}

func (h Handler) deliveryEvents(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "activity_unavailable", "delivery events require PostgreSQL")
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	items, err := h.Postgres.ListDeliveryEvents(r.Context(), principal, limit)
	if errors.Is(err, ErrInvalid) {
		Error(w, http.StatusBadRequest, "invalid_request", "delivery event query is invalid")
		return
	}
	if err != nil {
		h.databaseError(w, "list delivery events", err)
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": items})
}
