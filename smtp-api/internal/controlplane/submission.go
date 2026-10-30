package controlplane

import (
	"bytes"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"mime"
	"net/http"
	"net/mail"
	"strings"
	"time"
	"unicode"
	"unicode/utf8"

	"github.com/jackc/pgx/v5/pgconn"
)

const (
	maxSubmissionBytes      = 10 * 1024 * 1024
	maxSubmissionRecipients = 100
)

type submissionRequest struct {
	From    string   `json:"from"`
	To      []string `json:"to"`
	Subject string   `json:"subject"`
	Text    string   `json:"text"`
}

type normalizedSubmission struct {
	From    string
	To      []string
	Subject string
	Text    string
}

type Submission struct {
	ID        string    `json:"id"`
	From      string    `json:"from"`
	To        []string  `json:"to"`
	Subject   string    `json:"subject"`
	Status    string    `json:"status"`
	CreatedAt time.Time `json:"created_at"`
}

func normalizeSubmission(input submissionRequest) (normalizedSubmission, error) {
	from := strings.TrimSpace(input.From)
	if !validSubmissionAddress(from) {
		return normalizedSubmission{}, ErrInvalid
	}
	if len(input.To) == 0 || len(input.To) > maxSubmissionRecipients {
		return normalizedSubmission{}, ErrInvalid
	}

	recipients := make([]string, 0, len(input.To))
	seen := make(map[string]struct{}, len(input.To))
	for _, raw := range input.To {
		recipient := strings.TrimSpace(raw)
		if !validSubmissionAddress(recipient) {
			return normalizedSubmission{}, ErrInvalid
		}
		key := strings.ToLower(recipient)
		if _, exists := seen[key]; exists {
			return normalizedSubmission{}, ErrInvalid
		}
		seen[key] = struct{}{}
		recipients = append(recipients, recipient)
	}

	subject := strings.TrimSpace(input.Subject)
	if len(subject) > 900 || strings.IndexFunc(subject, unicode.IsControl) >= 0 {
		return normalizedSubmission{}, ErrInvalid
	}
	if !utf8.ValidString(input.Text) || strings.ContainsRune(input.Text, '\x00') {
		return normalizedSubmission{}, ErrInvalid
	}
	text := strings.ReplaceAll(input.Text, "\r\n", "\n")
	text = strings.ReplaceAll(text, "\r", "\n")
	text = strings.ReplaceAll(text, "\n", "\r\n")
	return normalizedSubmission{From: from, To: recipients, Subject: subject, Text: text}, nil
}

func validSubmissionAddress(value string) bool {
	if value == "" || len(value) > 320 || !utf8.ValidString(value) ||
		strings.ContainsAny(value, "\r\n") || strings.Count(value, "@") != 1 {
		return false
	}
	for _, character := range value {
		if character < '!' || character > '~' {
			return false
		}
	}
	parsed, err := mail.ParseAddress(value)
	return err == nil && parsed.Address == value && len(value) <= 320
}

func validIdempotencyKey(value string) bool {
	if len(value) == 0 || len(value) > 256 {
		return false
	}
	for _, r := range value {
		if !(r == '-' || r == '_' || r == '.' || r == ':' ||
			r >= '0' && r <= '9' || r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z') {
			return false
		}
	}
	return true
}

func submissionFingerprint(submission normalizedSubmission) ([]byte, error) {
	canonical, err := json.Marshal(submission)
	if err != nil {
		return nil, fmt.Errorf("encode normalized submission: %w", err)
	}
	fingerprint := sha256.Sum256(canonical)
	return fingerprint[:], nil
}

func submissionMessage(submission normalizedSubmission, messageID string, now time.Time) []byte {
	subject := mime.QEncoding.Encode("UTF-8", submission.Subject)
	var message bytes.Buffer
	fmt.Fprintf(&message, "From: %s\r\n", submission.From)
	fmt.Fprintf(&message, "To: %s\r\n", strings.Join(submission.To, ", "))
	fmt.Fprintf(&message, "Date: %s\r\n", now.UTC().Format(time.RFC1123Z))
	fmt.Fprintf(&message, "Message-ID: <%s@smtp-platform.local>\r\n", messageID)
	fmt.Fprintf(&message, "Subject: %s\r\n", subject)
	message.WriteString("MIME-Version: 1.0\r\n")
	message.WriteString("Content-Type: text/plain; charset=utf-8\r\n")
	message.WriteString("Content-Transfer-Encoding: base64\r\n")
	message.WriteString("\r\n")
	encoded := base64.StdEncoding.EncodeToString([]byte(submission.Text))
	for len(encoded) > 0 {
		lineLength := min(76, len(encoded))
		message.WriteString(encoded[:lineLength])
		message.WriteString("\r\n")
		encoded = encoded[lineLength:]
	}
	return message.Bytes()
}

func submissionSubject(message []byte) (string, error) {
	parsed, err := mail.ReadMessage(bytes.NewReader(message))
	if err != nil {
		return "", fmt.Errorf("parse queued submission headers: %w", err)
	}
	subject, err := new(mime.WordDecoder).DecodeHeader(parsed.Header.Get("Subject"))
	if err != nil {
		return "", fmt.Errorf("decode queued submission subject: %w", err)
	}
	return subject, nil
}

var errIdempotencyConflict = errors.New("idempotency key already used with a different submission")

func (h Handler) submit(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	idempotencyKey := r.Header.Get("Idempotency-Key")
	if !validIdempotencyKey(idempotencyKey) {
		Error(w, http.StatusBadRequest, "invalid_request", "Idempotency-Key must be 1-256 ASCII letters, numbers, or - _ . :")
		return
	}

	r.Body = http.MaxBytesReader(w, r.Body, maxSubmissionBytes+(64<<10))
	var input submissionRequest
	if err := decodeJSON(r, &input); err != nil {
		var tooLarge *http.MaxBytesError
		if errors.As(err, &tooLarge) {
			Error(w, http.StatusRequestEntityTooLarge, "message_too_large", "submission request exceeds the maximum size")
			return
		}
		Error(w, http.StatusBadRequest, "invalid_request", "invalid JSON submission")
		return
	}
	submission, err := normalizeSubmission(input)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_submission", "sender, recipients, subject, or message body is invalid")
		return
	}
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "submission_unavailable", "durable message queue is unavailable")
		return
	}
	item, created, err := h.Postgres.EnqueueSubmission(r.Context(), principal, idempotencyKey, submission)
	switch {
	case errors.Is(err, errIdempotencyConflict):
		Error(w, http.StatusConflict, "idempotency_conflict", "Idempotency-Key was already used for a different submission")
		return
	case errors.Is(err, ErrForbidden):
		Error(w, http.StatusForbidden, "sender_domain_unverified", "sender address must use a verified tenant domain")
		return
	case errors.Is(err, ErrInvalid):
		Error(w, http.StatusBadRequest, "invalid_submission", "submission exceeds queue limits or contains invalid data")
		return
	case err != nil:
		logSubmissionError(w, "message submission failed", err)
		Error(w, http.StatusServiceUnavailable, "submission_unavailable", "message could not be added to the queue")
		return
	}
	JSON(w, http.StatusAccepted, map[string]any{
		"submission": item,
		"created":    created,
		"message":    "Accepted into the durable queue; delivery is not confirmed.",
	})
}

func (h Handler) submissions(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:read")
	if !ok {
		return
	}
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "submission_unavailable", "durable message queue is unavailable")
		return
	}
	limit, err := queryLimit(r)
	if err != nil {
		Error(w, http.StatusBadRequest, "invalid_query", "limit must be an integer from 1 to 100")
		return
	}
	items, err := h.Postgres.ListSubmissions(r.Context(), principal, limit)
	if err != nil {
		logSubmissionError(w, "message submission history failed", err)
		Error(w, http.StatusServiceUnavailable, "submission_unavailable", "submission history is temporarily unavailable")
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": items, "next_page": nil})
}

func logSubmissionError(w http.ResponseWriter, message string, err error) {
	var databaseError *pgconn.PgError
	if errors.As(err, &databaseError) {
		slog.Error(message,
			"request_id", w.Header().Get(requestIDHeader),
			"database_code", databaseError.Code,
		)
		return
	}
	slog.Error(message,
		"request_id", w.Header().Get(requestIDHeader),
		"error_type", fmt.Sprintf("%T", err),
	)
}
