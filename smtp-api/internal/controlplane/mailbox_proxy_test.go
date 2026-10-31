package controlplane

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestMailboxProxySignsRequestIdentityAndPayload(t *testing.T) {
	key := strings.Repeat("a", 64)
	signingKey, err := hex.DecodeString(key)
	if err != nil {
		t.Fatal(err)
	}
	proxyURL := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.RequestURI() != "/v1/mailboxes/box/messages?limit=5" {
			t.Errorf("forwarded URI=%q", r.URL.RequestURI())
		}
		body, err := io.ReadAll(r.Body)
		if err != nil {
			t.Fatal(err)
		}
		token := strings.Split(r.Header.Get("X-Mailbox-Principal"), ".")
		if len(token) != 2 {
			t.Fatalf("malformed internal token %q", r.Header.Get("X-Mailbox-Principal"))
		}
		payload, err := base64.RawURLEncoding.DecodeString(token[0])
		if err != nil {
			t.Fatal(err)
		}
		signature, err := base64.RawURLEncoding.DecodeString(token[1])
		if err != nil {
			t.Fatal(err)
		}
		mac := hmac.New(sha256.New, signingKey)
		_, _ = mac.Write(payload)
		if !hmac.Equal(signature, mac.Sum(nil)) {
			t.Fatal("internal identity signature is invalid")
		}
		var identity mailboxServicePrincipal
		if err := json.Unmarshal(payload, &identity); err != nil {
			t.Fatal(err)
		}
		expectedHash := sha256.Sum256(body)
		if identity.TenantID != "a1234567-1234-1234-1234-123456789abc" ||
			identity.UserID != "b1234567-1234-1234-1234-123456789abc" ||
			identity.Method != http.MethodPost ||
			identity.URI != r.URL.RequestURI() ||
			identity.BodySHA256 != hex.EncodeToString(expectedHash[:]) ||
			identity.IdempotencyKey != "append-001" {
			t.Errorf("unexpected signed identity: %#v", identity)
		}
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusCreated)
		_, _ = w.Write([]byte(`{"created":true}`))
	}))
	defer proxyURL.Close()

	proxy, err := NewMailboxProxyFromEnv(func(name string) string {
		switch name {
		case "MAILBOX_SERVICE_URL":
			return proxyURL.URL
		case "MAILBOX_SERVICE_HMAC_KEY":
			return key
		default:
			return ""
		}
	})
	if err != nil {
		t.Fatal(err)
	}
	request := httptest.NewRequest(
		http.MethodPost,
		"/v1/mailboxes/box/messages?limit=5",
		strings.NewReader("raw message bytes"),
	)
	request.Header.Set("Idempotency-Key", "append-001")
	request.Header.Set("Authorization", "Bearer sp_"+strings.Repeat("a", 43))
	p := Principal{
		Subject:  "api-key:fixture",
		TenantID: "a1234567-1234-1234-1234-123456789abc",
		UserID:   "b1234567-1234-1234-1234-123456789abc",
		Role:     Owner,
		Scopes:   map[string]bool{"mailbox:write": true},
		durable:  true,
	}
	recorder := httptest.NewRecorder()
	proxy.ServeHTTP(recorder, request, p)
	if recorder.Code != http.StatusCreated || recorder.Body.String() != `{"created":true}` {
		t.Fatalf("proxy response=%d %s", recorder.Code, recorder.Body.String())
	}
}

func TestMailboxProxyRejectsRemoteCleartextAndWeakKeys(t *testing.T) {
	for _, test := range []struct {
		name string
		env  map[string]string
	}{
		{
			name: "remote cleartext",
			env: map[string]string{
				"MAILBOX_SERVICE_URL":      "http://mailbox.internal:8081",
				"MAILBOX_SERVICE_HMAC_KEY": strings.Repeat("a", 64),
			},
		},
		{
			name: "short key",
			env: map[string]string{
				"MAILBOX_SERVICE_URL":      "http://127.0.0.1:8081",
				"MAILBOX_SERVICE_HMAC_KEY": strings.Repeat("a", 62),
			},
		},
	} {
		t.Run(test.name, func(t *testing.T) {
			_, err := NewMailboxProxyFromEnv(func(name string) string {
				return test.env[name]
			})
			if err == nil {
				t.Fatal("expected invalid mailbox service configuration to fail")
			}
		})
	}
}
