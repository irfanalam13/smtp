package identity

import (
	"context"
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

const testAudience = "smtp-control-plane"

func TestKeycloakVerifier(t *testing.T) {
	privateKey, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}

	var server *httptest.Server
	mux := http.NewServeMux()
	server = httptest.NewServer(mux)
	defer server.Close()

	mux.HandleFunc("GET /.well-known/openid-configuration", func(w http.ResponseWriter, _ *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]string{
			"issuer":   server.URL,
			"jwks_uri": server.URL + "/keys",
		})
	})
	mux.HandleFunc("GET /keys", func(w http.ResponseWriter, _ *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]any{
			"keys": []map[string]string{{
				"kty": "RSA",
				"use": "sig",
				"alg": "RS256",
				"kid": "test-key",
				"n":   base64.RawURLEncoding.EncodeToString(privateKey.PublicKey.N.Bytes()),
				"e":   "AQAB",
			}},
		})
	})

	verifier, err := NewKeycloakVerifier(context.Background(), server.URL, testAudience)
	if err != nil {
		t.Fatal(err)
	}

	tests := []struct {
		name        string
		issuer      string
		audience    string
		tokenType   string
		expiresAt   time.Time
		wantSubject bool
	}{
		{
			name:        "valid token",
			issuer:      server.URL,
			audience:    testAudience,
			tokenType:   "Bearer",
			expiresAt:   time.Now().Add(time.Minute),
			wantSubject: true,
		},
		{
			name:      "wrong issuer",
			issuer:    server.URL + "/other",
			audience:  testAudience,
			tokenType: "Bearer",
			expiresAt: time.Now().Add(time.Minute),
		},
		{
			name:      "wrong audience",
			issuer:    server.URL,
			audience:  "another-api",
			tokenType: "Bearer",
			expiresAt: time.Now().Add(time.Minute),
		},
		{
			name:      "expired token",
			issuer:    server.URL,
			audience:  testAudience,
			tokenType: "Bearer",
			expiresAt: time.Now().Add(-time.Minute),
		},
		{
			name:      "ID token type",
			issuer:    server.URL,
			audience:  testAudience,
			tokenType: "ID",
			expiresAt: time.Now().Add(time.Minute),
		},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			rawToken := signTestToken(t, privateKey, test.issuer, test.audience, test.tokenType, test.expiresAt)
			subject, err := verifier.Verify(context.Background(), rawToken)
			if test.wantSubject {
				if err != nil || subject.Subject != "keycloak-user-123" || subject.Issuer != server.URL {
					t.Fatalf("Verify() = (%#v, %v), want verified issuer and subject", subject, err)
				}
				return
			}
			if err == nil || subject != (Claims{}) {
				t.Fatalf("Verify() = (%#v, %v), want rejection", subject, err)
			}
		})
	}
}

func TestNewKeycloakVerifierRequiresHTTPSOutsideLoopback(t *testing.T) {
	for _, issuer := range []string{
		"http://idp.example.test/realms/smtp",
		"https://user:pass@idp.example.test/realms/smtp",
		"https://idp.example.test/realms/smtp?query=1",
	} {
		t.Run(issuer, func(t *testing.T) {
			if _, err := NewKeycloakVerifier(context.Background(), issuer, testAudience); err == nil {
				t.Fatal("NewKeycloakVerifier() succeeded for invalid issuer URL")
			}
		})
	}
}

func TestKeycloakVerifierRejectsOversizedToken(t *testing.T) {
	verifier := &KeycloakVerifier{verifier: nil}
	if _, err := verifier.Verify(context.Background(), string(make([]byte, maxAccessTokenBytes+1))); err == nil {
		t.Fatal("Verify() accepted oversized token")
	}
}

func signTestToken(
	t *testing.T,
	key *rsa.PrivateKey,
	issuer string,
	audience string,
	tokenType string,
	expiresAt time.Time,
) string {
	t.Helper()
	header, err := json.Marshal(map[string]string{
		"alg": "RS256",
		"kid": "test-key",
		"typ": "JWT",
	})
	if err != nil {
		t.Fatal(err)
	}
	claims, err := json.Marshal(map[string]any{
		"iss": issuer,
		"sub": "keycloak-user-123",
		"aud": audience,
		"typ": tokenType,
		"exp": expiresAt.Unix(),
		"iat": time.Now().Add(-time.Second).Unix(),
	})
	if err != nil {
		t.Fatal(err)
	}
	unsigned := base64.RawURLEncoding.EncodeToString(header) + "." +
		base64.RawURLEncoding.EncodeToString(claims)
	digest := sha256.Sum256([]byte(unsigned))
	signature, err := rsa.SignPKCS1v15(rand.Reader, key, crypto.SHA256, digest[:])
	if err != nil {
		t.Fatal(err)
	}
	return fmt.Sprintf("%s.%s", unsigned, base64.RawURLEncoding.EncodeToString(signature))
}
