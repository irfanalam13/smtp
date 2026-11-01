package controlplane

import (
	"context"
	"errors"
	"net/http"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
)

type RecipientSuppression struct {
	Recipient   string    `json:"recipient"`
	Reason      string    `json:"reason"`
	SourceEvent *string   `json:"source_event_id,omitempty"`
	Active      bool      `json:"active"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
}

func (repository *PostgreSQLRepository) ListRecipientSuppressions(
	ctx context.Context,
	principal Principal,
	limit int,
) ([]RecipientSuppression, error) {
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) ||
		limit < 1 || limit > 100 {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT recipient, reason, source_event_id::text, active, created_at, updated_at
		FROM recipient_suppressions
		WHERE tenant_id = $1 AND active = TRUE
		ORDER BY updated_at DESC, recipient
		LIMIT $2
	`, principal.TenantID, limit)
	if err != nil {
		return nil, errors.New("list recipient suppressions: " + err.Error())
	}
	defer rows.Close()

	var items []RecipientSuppression
	for rows.Next() {
		var item RecipientSuppression
		if err := rows.Scan(
			&item.Recipient, &item.Reason, &item.SourceEvent,
			&item.Active, &item.CreatedAt, &item.UpdatedAt,
		); err != nil {
			return nil, errors.New("scan recipient suppression: " + err.Error())
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return nil, errors.New("iterate recipient suppressions: " + err.Error())
	}
	return items, nil
}

func (repository *PostgreSQLRepository) UpsertRecipientSuppression(
	ctx context.Context,
	principal Principal,
	recipient string,
	reason string,
) (RecipientSuppression, error) {
	recipient = strings.ToLower(strings.TrimSpace(recipient))
	reason = strings.ToLower(strings.TrimSpace(reason))
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) ||
		!validEmail(recipient) ||
		(reason != "hard_bounce" && reason != "complaint" && reason != "manual") {
		return RecipientSuppression{}, ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return RecipientSuppression{}, errors.New("begin recipient suppression update: " + err.Error())
	}
	defer func() { _ = tx.Rollback(ctx) }()

	var item RecipientSuppression
	err = tx.QueryRow(ctx, `
		INSERT INTO recipient_suppressions (tenant_id, recipient, reason, active)
		VALUES ($1, $2, $3, TRUE)
		ON CONFLICT (tenant_id, recipient) DO UPDATE
		SET reason = EXCLUDED.reason, active = TRUE, updated_at = NOW()
		RETURNING recipient, reason, source_event_id::text, active, created_at, updated_at
	`, principal.TenantID, recipient, reason).Scan(
		&item.Recipient, &item.Reason, &item.SourceEvent,
		&item.Active, &item.CreatedAt, &item.UpdatedAt,
	)
	if err != nil {
		return RecipientSuppression{}, errors.New("upsert recipient suppression: " + err.Error())
	}
	if err := insertAuthAudit(ctx, tx, principal, "recipient_suppression.upserted", recipient); err != nil {
		return RecipientSuppression{}, err
	}
	if err := tx.Commit(ctx); err != nil {
		return RecipientSuppression{}, errors.New("commit recipient suppression update: " + err.Error())
	}
	return item, nil
}

func (repository *PostgreSQLRepository) DeactivateRecipientSuppression(
	ctx context.Context,
	principal Principal,
	recipient string,
) error {
	recipient = strings.ToLower(strings.TrimSpace(recipient))
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) || !validEmail(recipient) {
		return ErrInvalid
	}
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return errors.New("begin recipient suppression removal: " + err.Error())
	}
	defer func() { _ = tx.Rollback(ctx) }()

	tag, err := tx.Exec(ctx, `
		UPDATE recipient_suppressions
		SET active = FALSE, updated_at = NOW()
		WHERE tenant_id = $1 AND lower(recipient) = $2 AND active = TRUE
	`, principal.TenantID, recipient)
	if err != nil {
		return errors.New("deactivate recipient suppression: " + err.Error())
	}
	if tag.RowsAffected() == 0 {
		return pgx.ErrNoRows
	}
	if err := insertAuthAudit(ctx, tx, principal, "recipient_suppression.deactivated", recipient); err != nil {
		return err
	}
	if err := tx.Commit(ctx); err != nil {
		return errors.New("commit recipient suppression removal: " + err.Error())
	}
	return nil
}

func (h Handler) suppressions(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "recipient suppressions require PostgreSQL")
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	items, err := h.Postgres.ListRecipientSuppressions(r.Context(), principal, limit)
	if err != nil {
		h.databaseError(w, "list recipient suppressions", err)
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": items})
}

func (h Handler) createSuppression(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "recipient suppressions require PostgreSQL")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var request struct {
		Recipient string `json:"recipient"`
		Reason    string `json:"reason"`
	}
	if err := decodeJSON(r, &request); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "invalid JSON request")
		return
	}
	item, err := h.Postgres.UpsertRecipientSuppression(
		r.Context(), principal, request.Recipient, request.Reason,
	)
	if errors.Is(err, ErrInvalid) {
		Error(w, http.StatusBadRequest, "invalid_request", "recipient and reason are invalid")
		return
	}
	if err != nil {
		h.databaseError(w, "upsert recipient suppression", err)
		return
	}
	JSON(w, http.StatusCreated, item)
}

func (h Handler) deleteSuppression(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "recipient suppressions require PostgreSQL")
		return
	}
	err := h.Postgres.DeactivateRecipientSuppression(
		r.Context(), principal, r.PathValue("recipient"),
	)
	if errors.Is(err, pgx.ErrNoRows) {
		Error(w, http.StatusNotFound, "not_found", "recipient suppression not found")
		return
	}
	if errors.Is(err, ErrInvalid) {
		Error(w, http.StatusBadRequest, "invalid_request", "recipient is invalid")
		return
	}
	if err != nil {
		h.databaseError(w, "deactivate recipient suppression", err)
		return
	}
	JSON(w, http.StatusOK, map[string]string{"status": "deactivated"})
}
