package controlplane

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"net/http"
	"time"
)

type SubmissionAttempt struct {
	ID            int64      `json:"id"`
	AttemptNumber int        `json:"attempt_number"`
	Recipient     string     `json:"recipient"`
	StartedAt     time.Time  `json:"started_at"`
	FinishedAt    *time.Time `json:"finished_at,omitempty"`
	Outcome       string     `json:"outcome"`
	ErrorCategory *string    `json:"error_category,omitempty"`
	Diagnostic    *string    `json:"diagnostic,omitempty"`
	SMTPCode      *int       `json:"smtp_code,omitempty"`
	RemoteHost    *string    `json:"remote_host,omitempty"`
}

func (repository *PostgreSQLRepository) ListSubmissionAttempts(
	ctx context.Context,
	principal Principal,
	submissionID string,
) ([]SubmissionAttempt, error) {
	scope := "api:" + principal.TenantID
	if scope == "api:" || !validUUID(submissionID) {
		return nil, ErrInvalid
	}

	var exists bool
	if err := repository.pool.QueryRow(ctx, `
		SELECT EXISTS (
			SELECT 1 FROM mail_idempotency
			WHERE scope = $1 AND message_id = $2
		)
	`, scope, submissionID).Scan(&exists); err != nil {
		return nil, fmt.Errorf("check submission activity access: %w", err)
	}
	if !exists {
		return nil, ErrNotFound
	}

	rows, err := repository.pool.Query(ctx, `
		SELECT attempt.id, attempt.attempt_number, recipient.recipient,
		       attempt.started_at, attempt.finished_at, attempt.outcome,
		       attempt.diagnostic, attempt.smtp_code, attempt.remote_host
		FROM mail_recipient_attempts AS attempt
		JOIN mail_recipient_deliveries AS delivery ON delivery.id = attempt.delivery_id
		JOIN mail_recipients AS recipient ON recipient.id = delivery.recipient_id
		WHERE delivery.message_id = $1
		ORDER BY attempt.started_at ASC, recipient.recipient ASC
		LIMIT 100
	`, submissionID)
	if err != nil {
		return nil, fmt.Errorf("list submission activity: %w", err)
	}
	defer rows.Close()

	items := make([]SubmissionAttempt, 0)
	for rows.Next() {
		var item SubmissionAttempt
		var finishedAt sql.NullTime
		var diagnostic sql.NullString
		var smtpCode sql.NullInt64
		var remoteHost sql.NullString
		if err := rows.Scan(
			&item.ID,
			&item.AttemptNumber,
			&item.Recipient,
			&item.StartedAt,
			&finishedAt,
			&item.Outcome,
			&diagnostic,
			&smtpCode,
			&remoteHost,
		); err != nil {
			return nil, fmt.Errorf("scan submission activity: %w", err)
		}
		if finishedAt.Valid {
			item.FinishedAt = &finishedAt.Time
		}
		if diagnostic.Valid {
			item.Diagnostic = &diagnostic.String
		}
		if smtpCode.Valid {
			code := int(smtpCode.Int64)
			item.SMTPCode = &code
		}
		if remoteHost.Valid {
			item.RemoteHost = &remoteHost.String
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("iterate submission activity: %w", err)
	}
	return items, nil
}

func (h Handler) submissionAttempts(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	submissionID := r.PathValue("id")
	if !validUUID(submissionID) {
		Error(w, http.StatusBadRequest, "invalid_request", "submission ID must be a UUID")
		return
	}
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "activity_unavailable", "submission activity is unavailable")
		return
	}

	items, err := h.Postgres.ListSubmissionAttempts(r.Context(), principal, submissionID)
	switch {
	case errors.Is(err, ErrInvalid):
		Error(w, http.StatusBadRequest, "invalid_request", "submission ID is invalid")
	case errors.Is(err, ErrNotFound):
		Error(w, http.StatusNotFound, "not_found", "submission was not found")
	case err != nil:
		logSubmissionError(w, "submission activity lookup failed", err)
		Error(w, http.StatusServiceUnavailable, "activity_unavailable", "submission activity is temporarily unavailable")
	default:
		JSON(w, http.StatusOK, map[string]any{"items": items})
	}
}
