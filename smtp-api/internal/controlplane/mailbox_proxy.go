package controlplane

import (
	"bytes"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net"
	"net/http"
	"net/url"
	"sort"
	"strings"
	"time"
)

const maxMailboxRequestBytes = 10*1024*1024 + 16*1024

type MailboxProxy struct {
	baseURL *url.URL
	key     []byte
	client  *http.Client
}

type mailboxServicePrincipal struct {
	TenantID       string   `json:"tenant_id"`
	UserID         string   `json:"user_id"`
	Role           string   `json:"role"`
	Scopes         []string `json:"scopes"`
	IssuedAt       int64    `json:"issued_at"`
	Method         string   `json:"method"`
	URI            string   `json:"uri"`
	BodySHA256     string   `json:"body_sha256"`
	IdempotencyKey string   `json:"idempotency_key,omitempty"`
}

func NewMailboxProxyFromEnv(getenv func(string) string) (*MailboxProxy, error) {
	rawURL := strings.TrimSpace(getenv("MAILBOX_SERVICE_URL"))
	if rawURL == "" {
		rawURL = "http://127.0.0.1:8081"
	}
	baseURL, err := url.Parse(rawURL)
	if err != nil || baseURL.Host == "" || baseURL.User != nil ||
		baseURL.RawQuery != "" || baseURL.Fragment != "" || baseURL.Path != "" ||
		(baseURL.Scheme != "https" && baseURL.Scheme != "http") {
		return nil, errors.New("MAILBOX_SERVICE_URL must be an origin URL without credentials or path")
	}
	if baseURL.Scheme != "https" && !isLoopbackMailboxService(baseURL.Hostname()) {
		return nil, errors.New("remote mailbox service URLs require HTTPS")
	}
	rawKey := strings.TrimSpace(getenv("MAILBOX_SERVICE_HMAC_KEY"))
	key, err := hex.DecodeString(rawKey)
	if err != nil || len(key) < 32 {
		return nil, errors.New("MAILBOX_SERVICE_HMAC_KEY must be at least 64 hexadecimal characters")
	}
	return &MailboxProxy{
		baseURL: baseURL,
		key:     key,
		client:  &http.Client{Timeout: 30 * time.Second},
	}, nil
}

func isLoopbackMailboxService(host string) bool {
	if strings.EqualFold(host, "localhost") {
		return true
	}
	ip := net.ParseIP(host)
	return ip != nil && ip.IsLoopback()
}

func (proxy *MailboxProxy) ServeHTTP(w http.ResponseWriter, r *http.Request, principal Principal) {
	if proxy == nil {
		Error(w, http.StatusServiceUnavailable, "mailbox_service_unavailable", "mailbox service is not configured")
		return
	}
	if !strings.HasPrefix(r.URL.Path, "/v1/mailboxes") {
		Error(w, http.StatusNotFound, "not_found", "mailbox route not found")
		return
	}
	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, maxMailboxRequestBytes))
	if err != nil {
		Error(w, http.StatusRequestEntityTooLarge, "request_too_large", "request body exceeds mailbox service limit")
		return
	}
	if !validMailboxPrincipal(principal) {
		Error(w, http.StatusForbidden, "forbidden", "mailbox access requires a user-bound principal")
		return
	}

	scopes := make([]string, 0, len(principal.Scopes))
	for scope, allowed := range principal.Scopes {
		if allowed {
			scopes = append(scopes, scope)
		}
	}
	sort.Strings(scopes)
	identity := mailboxServicePrincipal{
		TenantID:       principal.TenantID,
		UserID:         principal.UserID,
		Role:           string(principal.Role),
		Scopes:         scopes,
		IssuedAt:       time.Now().Unix(),
		Method:         r.Method,
		URI:            r.URL.RequestURI(),
		BodySHA256:     fmt.Sprintf("%x", sha256.Sum256(body)),
		IdempotencyKey: r.Header.Get("Idempotency-Key"),
	}
	payload, err := json.Marshal(identity)
	if err != nil {
		slog.Error("encode mailbox service identity", "error", err)
		Error(w, http.StatusInternalServerError, "internal_error", "request could not be processed")
		return
	}
	mac := hmac.New(sha256.New, proxy.key)
	_, _ = mac.Write(payload)
	token := base64.RawURLEncoding.EncodeToString(payload) + "." +
		base64.RawURLEncoding.EncodeToString(mac.Sum(nil))

	target := *proxy.baseURL
	target.Path = r.URL.Path
	target.RawQuery = r.URL.RawQuery
	request, err := http.NewRequestWithContext(
		r.Context(),
		r.Method,
		target.String(),
		bytes.NewReader(body),
	)
	if err != nil {
		slog.Error("build mailbox service request", "error", err)
		Error(w, http.StatusInternalServerError, "internal_error", "request could not be processed")
		return
	}
	request.Header.Set("X-Mailbox-Principal", token)
	for _, header := range []string{
		"Content-Type",
		"Range",
		"If-Range",
		"If-None-Match",
		"If-Modified-Since",
	} {
		if value := r.Header.Get(header); value != "" {
			request.Header.Set(header, value)
		}
	}
	response, err := proxy.client.Do(request)
	if err != nil {
		slog.ErrorContext(r.Context(), "call mailbox service",
			"request_id", w.Header().Get(requestIDHeader),
			"method", r.Method,
			"path", r.URL.Path,
			"error", err,
		)
		Error(w, http.StatusServiceUnavailable, "mailbox_service_unavailable", "mailbox service is temporarily unavailable")
		return
	}
	defer response.Body.Close()
	for _, header := range []string{
		"Content-Type",
		"Content-Length",
		"Content-Range",
		"Accept-Ranges",
		"ETag",
		"Last-Modified",
		"Content-Disposition",
	} {
		if value := response.Header.Get(header); value != "" {
			w.Header().Set(header, value)
		}
	}
	w.WriteHeader(response.StatusCode)
	if _, err := io.Copy(w, response.Body); err != nil {
		slog.ErrorContext(r.Context(), "stream mailbox service response",
			"request_id", w.Header().Get(requestIDHeader),
			"method", r.Method,
			"path", r.URL.Path,
			"error", err,
		)
	}
}
