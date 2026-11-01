package controlplane

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"errors"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
)

type WebhookSubscription struct {
	ID         string    `json:"id"`
	Endpoint   string    `json:"endpoint"`
	EventTypes []string  `json:"event_types"`
	Active     bool      `json:"active"`
	CreatedAt  time.Time `json:"created_at"`
	UpdatedAt  time.Time `json:"updated_at"`
}

func newUUID() (string, error) {
	var raw [16]byte
	if _, err := rand.Read(raw[:]); err != nil {
		return "", err
	}
	raw[6] = (raw[6] & 0x0f) | 0x40
	raw[8] = (raw[8] & 0x3f) | 0x80
	return formatUUID(raw), nil
}

func formatUUID(raw [16]byte) string {
	const hex = "0123456789abcdef"
	var output [36]byte
	j := 0
	for i, value := range raw {
		if i == 4 || i == 6 || i == 8 || i == 10 {
			output[j] = '-'
			j++
		}
		output[j] = hex[value>>4]
		output[j+1] = hex[value&15]
		j += 2
	}
	return string(output[:])
}

func (repository *PostgreSQLRepository) CreateWebhookSubscription(
	ctx context.Context, principal Principal, endpoint string, eventTypes []string,
) (WebhookSubscription, string, error) {
	endpoint = strings.TrimSpace(endpoint)
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) ||
		endpoint == "" || len(endpoint) > 2048 {
		return WebhookSubscription{}, "", ErrInvalid
	}
	parsed, err := url.Parse(endpoint)
	if err != nil || (parsed.Scheme != "http" && parsed.Scheme != "https") || parsed.Host == "" {
		return WebhookSubscription{}, "", ErrInvalid
	}
	normalized := make([]string, 0, len(eventTypes))
	seen := make(map[string]struct{}, len(eventTypes))
	for _, eventType := range eventTypes {
		eventType = strings.TrimSpace(eventType)
		if eventType == "" || len(eventType) > 128 {
			return WebhookSubscription{}, "", ErrInvalid
		}
		if _, exists := seen[eventType]; !exists {
			seen[eventType] = struct{}{}
			normalized = append(normalized, eventType)
		}
	}
	if len(normalized) == 0 {
		return WebhookSubscription{}, "", ErrInvalid
	}
	id, err := newUUID()
	if err != nil {
		return WebhookSubscription{}, "", errors.New("generate webhook id: " + err.Error())
	}
	secretBytes := make([]byte, 32)
	if _, err := rand.Read(secretBytes); err != nil {
		return WebhookSubscription{}, "", errors.New("generate webhook secret: " + err.Error())
	}
	secret := base64.RawURLEncoding.EncodeToString(secretBytes)
	var item WebhookSubscription
	err = repository.pool.QueryRow(ctx, `
		INSERT INTO webhook_subscriptions
			(id, tenant_id, endpoint, secret, event_types)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING id::text, endpoint, event_types, active, created_at, updated_at
	`, id, principal.TenantID, endpoint, secretBytes, normalized).Scan(
		&item.ID, &item.Endpoint, &item.EventTypes, &item.Active,
		&item.CreatedAt, &item.UpdatedAt,
	)
	if err != nil {
		return WebhookSubscription{}, "", errors.New("create webhook subscription: " + err.Error())
	}
	return item, secret, nil
}

func (repository *PostgreSQLRepository) ListWebhookSubscriptions(
	ctx context.Context, principal Principal,
) ([]WebhookSubscription, error) {
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) {
		return nil, ErrInvalid
	}
	rows, err := repository.pool.Query(ctx, `
		SELECT id::text, endpoint, event_types, active, created_at, updated_at
		FROM webhook_subscriptions
		WHERE tenant_id = $1
		ORDER BY created_at DESC
	`, principal.TenantID)
	if err != nil {
		return nil, errors.New("list webhook subscriptions: " + err.Error())
	}
	defer rows.Close()
	var items []WebhookSubscription
	for rows.Next() {
		var item WebhookSubscription
		if err := rows.Scan(&item.ID, &item.Endpoint, &item.EventTypes, &item.Active, &item.CreatedAt, &item.UpdatedAt); err != nil {
			return nil, errors.New("scan webhook subscription: " + err.Error())
		}
		items = append(items, item)
	}
	if err := rows.Err(); err != nil {
		return nil, errors.New("iterate webhook subscriptions: " + err.Error())
	}
	return items, nil
}

func (repository *PostgreSQLRepository) DeleteWebhookSubscription(ctx context.Context, principal Principal, id string) error {
	if repository == nil || !principal.durable || !validUUID(principal.TenantID) || !validUUID(id) {
		return ErrInvalid
	}
	tag, err := repository.pool.Exec(ctx, `
		DELETE FROM webhook_subscriptions WHERE tenant_id = $1 AND id = $2
	`, principal.TenantID, id)
	if err != nil {
		return errors.New("delete webhook subscription: " + err.Error())
	}
	if tag.RowsAffected() == 0 {
		return pgx.ErrNoRows
	}
	return nil
}

func (h Handler) webhooks(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "webhooks require PostgreSQL")
		return
	}
	items, err := h.Postgres.ListWebhookSubscriptions(r.Context(), principal)
	if err != nil {
		h.databaseError(w, "list webhook subscriptions", err)
		return
	}
	JSON(w, http.StatusOK, map[string]any{"items": items})
}

func (h Handler) createWebhook(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "webhooks require PostgreSQL")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 16*1024)
	var request struct {
		Endpoint   string   `json:"endpoint"`
		EventTypes []string `json:"event_types"`
	}
	if err := decodeJSON(r, &request); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "invalid JSON request")
		return
	}
	item, secret, err := h.Postgres.CreateWebhookSubscription(r.Context(), principal, request.Endpoint, request.EventTypes)
	if errors.Is(err, ErrInvalid) {
		Error(w, http.StatusBadRequest, "invalid_request", "endpoint or event types are invalid")
		return
	}
	if err != nil {
		h.databaseError(w, "create webhook subscription", err)
		return
	}
	JSON(w, http.StatusCreated, map[string]any{"subscription": item, "secret": secret})
}

func (h Handler) deleteWebhook(w http.ResponseWriter, r *http.Request) {
	principal, ok := h.principal(w, r, "delivery:write")
	if !ok {
		return
	}
	if h.Postgres == nil || !principal.durable {
		Error(w, http.StatusServiceUnavailable, "control_plane_unavailable", "webhooks require PostgreSQL")
		return
	}
	err := h.Postgres.DeleteWebhookSubscription(r.Context(), principal, r.PathValue("id"))
	if errors.Is(err, pgx.ErrNoRows) {
		Error(w, http.StatusNotFound, "not_found", "webhook subscription not found")
		return
	}
	if errors.Is(err, ErrInvalid) {
		Error(w, http.StatusBadRequest, "invalid_request", "webhook subscription id is invalid")
		return
	}
	if err != nil {
		h.databaseError(w, "delete webhook subscription", err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
