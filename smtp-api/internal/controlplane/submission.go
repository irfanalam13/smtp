package controlplane

import (
	"bytes"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"mime"
	"net/http"
	"net/mail"
	"os"
	"strconv"
	"strings"
	"time"
	"unicode"
	"unicode/utf8"

	"github.com/jackc/pgx/v5/pgconn"
)

const (
	maxSubmissionBytes      = 10 * 1024 * 1024
	maxSubmissionRecipients = 100
	maxIndexedBodyTextBytes = 64 * 1024
	maxIndexedMetadataBytes = 768 * 1024
)

type submissionRequest struct {
	From    string   `json:"from"`
	To      []string `json:"to"`
	Cc      []string `json:"cc"`
	Bcc     []string `json:"bcc"`
	Subject string   `json:"subject"`
	Text    string   `json:"text"`
}

type normalizedSubmission struct {
	From    string
	To      []string
	Cc      []string
	Bcc     []string
	Subject string
	Text    string
}

type Submission struct {
	ID        string    `json:"id"`
	From      string    `json:"from"`
	To        []string  `json:"to"`
	Cc        []string  `json:"cc"`
	Bcc       []string  `json:"bcc"`
	Subject   string    `json:"subject"`
	Status    string    `json:"status"`
	CreatedAt time.Time `json:"created_at"`
}

func normalizeSubmission(input submissionRequest) (normalizedSubmission, error) {
	from := strings.TrimSpace(input.From)
	if !validSubmissionAddress(from) {
		return normalizedSubmission{}, ErrInvalid
	}
	if len(input.To)+len(input.Cc)+len(input.Bcc) == 0 ||
		len(input.To)+len(input.Cc)+len(input.Bcc) > maxSubmissionRecipients {
		return normalizedSubmission{}, ErrInvalid
	}

	recipients := make([]string, 0, len(input.To)+len(input.Cc)+len(input.Bcc))
	seen := make(map[string]struct{}, len(input.To))
	normalizeRecipients := func(input []string) ([]string, error) {
		result := make([]string, 0, len(input))
		for _, raw := range input {
			recipient := strings.TrimSpace(raw)
			if !validSubmissionAddress(recipient) {
				return nil, ErrInvalid
			}
			key := strings.ToLower(recipient)
			if _, exists := seen[key]; exists {
				return nil, ErrInvalid
			}
			seen[key] = struct{}{}
			recipients = append(recipients, recipient)
			result = append(result, recipient)
		}
		return result, nil
	}
	to, err := normalizeRecipients(input.To)
	if err != nil {
		return normalizedSubmission{}, err
	}
	cc, err := normalizeRecipients(input.Cc)
	if err != nil {
		return normalizedSubmission{}, err
	}
	bcc, err := normalizeRecipients(input.Bcc)
	if err != nil {
		return normalizedSubmission{}, err
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
	return normalizedSubmission{From: from, To: to, Cc: cc, Bcc: bcc, Subject: subject, Text: text}, nil
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
	writeFoldedHeader(&message, "From", submission.From)
	if len(submission.To) > 0 {
		writeFoldedHeader(&message, "To", strings.Join(submission.To, ", "))
	}
	if len(submission.Cc) > 0 {
		writeFoldedHeader(&message, "Cc", strings.Join(submission.Cc, ", "))
	}
	writeFoldedHeader(&message, "Date", now.UTC().Format(time.RFC1123Z))
	writeFoldedHeader(&message, "Message-ID", "<"+messageID+"@smtp-platform.local>")
	writeFoldedHeader(&message, "Subject", subject)
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

func writeFoldedHeader(message *bytes.Buffer, name, value string) {
	line := name + ":"
	for _, word := range strings.Fields(value) {
		if len(line)+1+len(word) > 78 && line != name+":" {
			message.WriteString(line)
			message.WriteString("\r\n ")
			line = word
		} else if line == name+":" {
			line += " " + word
		} else {
			line += " " + word
		}
	}
	message.WriteString(line)
	message.WriteString("\r\n")
}

func indexedSubmissionMetadata(message []byte) (map[string]any, error) {
	parsed, err := mail.ReadMessage(bytes.NewReader(message))
	if err != nil {
		return nil, fmt.Errorf("parse generated submission headers: %w", err)
	}
	from, err := mail.ParseAddressList(parsed.Header.Get("From"))
	if err != nil || len(from) != 1 {
		return nil, fmt.Errorf("parse generated submission sender: invalid From header")
	}
	to, err := parseOptionalAddressList(parsed.Header.Get("To"))
	if err != nil || len(to) > maxSubmissionRecipients {
		return nil, fmt.Errorf("parse generated submission recipients: invalid To header")
	}
	cc, err := parseOptionalAddressList(parsed.Header.Get("Cc"))
	if err != nil || len(to)+len(cc) > maxSubmissionRecipients {
		return nil, fmt.Errorf("parse generated submission recipients: invalid Cc header")
	}
	subject, err := new(mime.WordDecoder).DecodeHeader(parsed.Header.Get("Subject"))
	if err != nil {
		return nil, fmt.Errorf("decode generated submission subject: %w", err)
	}
	messageID := strings.TrimSpace(parsed.Header.Get("Message-ID"))
	messageID = strings.TrimSuffix(strings.TrimPrefix(messageID, "<"), ">")
	if messageID == "" {
		return nil, fmt.Errorf("parse generated submission message ID: missing Message-ID")
	}
	mediaType, parameters, err := mime.ParseMediaType(parsed.Header.Get("Content-Type"))
	if err != nil || mediaType != "text/plain" || !strings.EqualFold(parameters["charset"], "utf-8") {
		return nil, fmt.Errorf("parse generated submission content type: unsupported Content-Type")
	}
	if !strings.EqualFold(parsed.Header.Get("Content-Transfer-Encoding"), "base64") {
		return nil, fmt.Errorf("parse generated submission body: unsupported transfer encoding")
	}
	text, err := io.ReadAll(io.LimitReader(
		base64.NewDecoder(base64.StdEncoding, parsed.Body),
		maxIndexedBodyTextBytes+1,
	))
	if err != nil {
		return nil, fmt.Errorf("decode generated submission body: %w", err)
	}
	if len(text) > maxIndexedBodyTextBytes {
		text = text[:maxIndexedBodyTextBytes]
		for !utf8.Valid(text) {
			text = text[:len(text)-1]
		}
	}
	if !utf8.Valid(text) {
		return nil, fmt.Errorf("decode generated submission body: invalid UTF-8")
	}
	recipients := make([]string, 0, len(to))
	for _, recipient := range to {
		recipients = append(recipients, recipient.Address)
	}
	ccRecipients := make([]string, 0, len(cc))
	for _, recipient := range cc {
		ccRecipients = append(ccRecipients, recipient.Address)
	}
	return map[string]any{
		"subject":        subject,
		"message_id":     messageID,
		"in_reply_to":    nil,
		"references":     []string{},
		"from":           []string{from[0].Address},
		"to":             recipients,
		"cc":             ccRecipients,
		"body_text":      []string{string(text)},
		"body_html":      []string{},
		"attachments":    []any{},
		"parse_warnings": 0,
	}, nil
}

func parseOptionalAddressList(value string) ([]*mail.Address, error) {
	if strings.TrimSpace(value) == "" {
		return nil, nil
	}
	return mail.ParseAddressList(value)
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

func (h Handler) sendingConfig(w http.ResponseWriter, r *http.Request) {
	if _, ok := h.principal(w, r, "delivery:read"); !ok {
		return
	}
	from := strings.TrimSpace(os.Getenv("SMTP_RELAY_FROM"))
	JSON(w, http.StatusOK, map[string]any{
		"relay_ready":   smtpRelayReady(),
		"delivery_mode": submissionDeliveryMode(),
		"from":          from,
	})
}

// submissionDeliveryMode is deliberately server-only: callers cannot choose a
// transport. This prevents a direct-MX failure from becoming an implicit relay.
func submissionDeliveryMode() string {
	if !strings.EqualFold(strings.TrimSpace(os.Getenv("SMTP_DELIVERY_ENABLED")), "true") {
		return ""
	}
	if smtpRelayReady() {
		if strings.EqualFold(strings.TrimSpace(os.Getenv("APP_ENVIRONMENT")), "development") &&
			strings.TrimSpace(os.Getenv("SMTP_RELAY_HOST")) == "127.0.0.1" &&
			strings.TrimSpace(os.Getenv("SMTP_RELAY_PORT")) == "1025" {
			return "mailpit"
		}
		return "relay"
	}
	// A direct-MX deployment must not carry a partial relay configuration.
	for _, name := range []string{"SMTP_RELAY_HOST", "SMTP_RELAY_PORT", "SMTP_RELAY_USERNAME", "SMTP_RELAY_PASSWORD", "SMTP_RELAY_FROM"} {
		if strings.TrimSpace(os.Getenv(name)) != "" {
			return ""
		}
	}
	if strings.TrimSpace(os.Getenv("SMTP_DELIVERY_ALLOWLIST")) == "" ||
		strings.TrimSpace(os.Getenv("SMTP_DKIM_KEY_DIR")) == "" {
		return ""
	}
	return "direct_mx"
}

func smtpRelayReady() bool {
	port, err := strconv.Atoi(strings.TrimSpace(os.Getenv("SMTP_RELAY_PORT")))
	host := strings.TrimSpace(os.Getenv("SMTP_RELAY_HOST"))
	username := os.Getenv("SMTP_RELAY_USERNAME")
	password := os.Getenv("SMTP_RELAY_PASSWORD")
	from := strings.TrimSpace(os.Getenv("SMTP_RELAY_FROM"))
	if err != nil {
		return false
	}
	localMailpit := strings.EqualFold(strings.TrimSpace(os.Getenv("APP_ENVIRONMENT")), "development") &&
		host == "127.0.0.1" &&
		port == 1025 &&
		username == "" &&
		password == ""
	authenticatedRelay := validRelayHostname(host) &&
		(port == 465 || port == 587) &&
		username != "" && len(username) <= 512 &&
		password != "" && len(password) <= 512 &&
		len(username)+len(password)+2 <= 360 &&
		strings.IndexFunc(username+password, unicode.IsControl) < 0
	return strings.EqualFold(strings.TrimSpace(os.Getenv("SMTP_DELIVERY_ENABLED")), "true") &&
		(localMailpit || authenticatedRelay) &&
		validSubmissionAddress(from)
}

func validRelayHostname(host string) bool {
	if host == "" || len(host) > 253 || !utf8.ValidString(host) {
		return false
	}
	host = strings.TrimSuffix(host, ".")
	if host == "" {
		return false
	}
	for _, label := range strings.Split(host, ".") {
		if len(label) == 0 || len(label) > 63 ||
			!isASCIIAlphaNumeric(label[0]) ||
			!isASCIIAlphaNumeric(label[len(label)-1]) {
			return false
		}
		for _, character := range label {
			if !(isASCIIAlphaNumeric(byte(character)) || character == '-') {
				return false
			}
		}
	}
	return true
}

func isASCIIAlphaNumeric(value byte) bool {
	return value >= 'a' && value <= 'z' ||
		value >= 'A' && value <= 'Z' ||
		value >= '0' && value <= '9'
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
	deliveryMode := submissionDeliveryMode()
	if deliveryMode == "" {
		Error(w, http.StatusServiceUnavailable, "delivery_unavailable", "SMTP delivery is not configured")
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
	item, created, err := h.Postgres.EnqueueSubmissionWithDelivery(
		r.Context(), principal, idempotencyKey, submission, deliveryMode,
	)
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
	case errors.Is(err, ErrSuppressed):
		Error(w, http.StatusUnprocessableEntity, "recipient_suppressed", "one or more recipients are suppressed for this tenant")
		return
	case err != nil:
		logSubmissionError(w, "message submission failed", err)
		Error(w, http.StatusServiceUnavailable, "submission_unavailable", "message could not be added to the queue")
		return
	}
	slog.Info("message submission enqueued",
		"request_id", w.Header().Get(requestIDHeader),
		"message_id", item.ID,
		"created", created,
		"recipient_count", len(item.To)+len(item.Cc)+len(item.Bcc),
	)
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
