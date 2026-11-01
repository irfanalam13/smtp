package controlplane

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"strings"
)

func (repository *PostgreSQLRepository) RecordFeedback(
	ctx context.Context,
	principal Principal,
	recipient, eventType, status, diagnostic string,
) error {
	recipient = strings.ToLower(strings.TrimSpace(recipient))
	eventType = strings.ToLower(strings.TrimSpace(eventType))
	status = strings.TrimSpace(status)
	diagnostic = strings.TrimSpace(diagnostic)
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) ||
		!validEmail(recipient) || (eventType != "bounce" && eventType != "complaint") ||
		len(status) > 32 || len(diagnostic) > 1000 {
		return ErrInvalid
	}
	if eventType == "bounce" && !strings.HasPrefix(status, "5.") {
		return ErrInvalid
	}
	id, err := newUUID()
	if err != nil {
		return errors.New("generate feedback event id: " + err.Error())
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return errors.New("begin feedback event: " + err.Error())
	}
	defer func() { _ = tx.Rollback(ctx) }()
	_, err = tx.Exec(ctx, `
		INSERT INTO feedback_events
			(id, tenant_id, recipient, event_type, status, diagnostic)
		VALUES ($1, $2, $3, $4, $5, $6)
		ON CONFLICT (tenant_id, event_type, recipient, status, diagnostic) DO NOTHING
	`, id, principal.TenantID, recipient, eventType, status, diagnostic)
	if err != nil {
		return errors.New("store feedback event: " + err.Error())
	}
	reason := "complaint"
	if eventType == "bounce" {
		reason = "hard_bounce"
	}
	_, err = tx.Exec(ctx, `
		INSERT INTO recipient_suppressions (tenant_id, recipient, reason, source_event_id)
		VALUES ($1, $2, $3, $4)
		ON CONFLICT (tenant_id, recipient) DO UPDATE
		SET reason = EXCLUDED.reason, source_event_id = EXCLUDED.source_event_id,
		    active = TRUE, updated_at = NOW()
	`, principal.TenantID, recipient, reason, id)
	if err != nil {
		return errors.New("update feedback suppression: " + err.Error())
	}
	if err := insertAuthAudit(ctx, tx, principal, "feedback."+eventType+".recorded", recipient); err != nil {
		return err
	}
	payload, err := json.Marshal(map[string]string{
		"id":         id,
		"type":       eventType,
		"recipient":  recipient,
		"status":     status,
		"diagnostic": diagnostic,
	})
	if err != nil {
		return errors.New("marshal feedback webhook: " + err.Error())
	}
	if _, err := tx.Exec(ctx, `
		INSERT INTO webhook_deliveries
			(id, subscription_id, tenant_id, event_id, event_type, payload)
		SELECT gen_random_uuid(), subscription.id, subscription.tenant_id, $2, $3, $4
		FROM webhook_subscriptions AS subscription
		WHERE subscription.tenant_id = $1 AND subscription.active = TRUE
		  AND $3 = ANY(subscription.event_types)
		ON CONFLICT (subscription_id, event_id) DO NOTHING
	`, principal.TenantID, id, eventType, payload); err != nil {
		return errors.New("enqueue feedback webhooks: " + err.Error())
	}
	if err := tx.Commit(ctx); err != nil {
		return errors.New("commit feedback event: " + err.Error())
	}
	return nil
}

func (h Handler) recordBounce(w http.ResponseWriter, r *http.Request) {
	h.recordFeedback(w, r, "bounce")
}

func (h Handler) recordComplaint(w http.ResponseWriter, r *http.Request) {
	h.recordFeedback(w, r, "complaint")
}

func (h Handler) recordFeedback(w http.ResponseWriter, r *http.Request, eventType string) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "feedback ingestion requires PostgreSQL")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 8192)
	var request struct {
		Recipient  string `json:"recipient"`
		Status     string `json:"status"`
		Diagnostic string `json:"diagnostic"`
	}
	if err := decodeJSON(r, &request); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "invalid JSON request")
		return
	}
	if err := h.Postgres.RecordFeedback(r.Context(), principal, request.Recipient, eventType, request.Status, request.Diagnostic); errors.Is(err, ErrInvalid) {
		Error(w, http.StatusBadRequest, "invalid_request", "feedback fields are invalid")
		return
	} else if err != nil {
		h.databaseError(w, "record feedback event", err)
		return
	}
	JSON(w, http.StatusAccepted, map[string]string{"status": "recorded"})
}
