package smtpauth

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
	"net/url"
	"testing"
	"time"

	"golang.org/x/oauth2"
)

func TestOIDCAuthorizationCodeUsesPKCEAndValidatesNonce(t *testing.T) {
	privateKey, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}
	const (
		clientID = "smtp-dashboard-test"
		nonce    = "test-nonce-with-enough-entropy-1234567890"
		verifier = "S256-verifier-with-enough-entropy-12345678901234567890"
	)
	var issuer string
	mux := http.NewServeMux()
	server := httptest.NewServer(mux)
	defer server.Close()
	issuer = server.URL
	idToken := signOIDCTestToken(t, privateKey, issuer, clientID, nonce, time.Now().Add(-time.Second))
	mux.HandleFunc("GET /.well-known/openid-configuration", func(w http.ResponseWriter, _ *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]string{
			"issuer": issuer, "authorization_endpoint": server.URL + "/authorize",
			"token_endpoint": server.URL + "/token", "jwks_uri": server.URL + "/keys",
		})
	})
	mux.HandleFunc("GET /keys", func(w http.ResponseWriter, _ *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]any{
			"keys": []map[string]string{{
				"kty": "RSA", "use": "sig", "alg": "RS256", "kid": "oidc-test-key",
				"n": base64.RawURLEncoding.EncodeToString(privateKey.PublicKey.N.Bytes()),
				"e": "AQAB",
			}},
		})
	})
	mux.HandleFunc("POST /token", func(w http.ResponseWriter, r *http.Request) {
		if err := r.ParseForm(); err != nil {
			http.Error(w, "bad form", http.StatusBadRequest)
			return
		}
		if r.Form.Get("code") != "authorization-code" ||
			r.Form.Get("redirect_uri") != "http://localhost:3210/api/backend/v1/auth/oidc/callback" ||
			r.Form.Get("code_verifier") != verifier {
			http.Error(w, "invalid authorization-code request", http.StatusBadRequest)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"access_token": "provider-access-token",
			"token_type":   "Bearer",
			"expires_in":   300,
			"id_token":     idToken,
		})
	})

	client, err := NewOIDCClient(context.Background(), OIDCConfig{
		Issuer: issuer, ClientID: clientID, ClientSecret: "server-only-test-secret",
		RedirectURL: "http://localhost:3210/api/backend/v1/auth/oidc/callback",
	})
	if err != nil {
		t.Fatal(err)
	}
	authorizationURL, err := client.AuthorizationURL(
		"state-with-sufficient-entropy-1234567890", nonce, verifier,
	)
	if err != nil {
		t.Fatal(err)
	}
	parsedAuthorizationURL, err := url.Parse(authorizationURL)
	if err != nil {
		t.Fatal(err)
	}
	query := parsedAuthorizationURL.Query()
	if query.Get("state") != "state-with-sufficient-entropy-1234567890" ||
		query.Get("nonce") != nonce || query.Get("code_challenge_method") != "S256" ||
		query.Get("code_challenge") != oauth2.S256ChallengeFromVerifier(verifier) {
		t.Fatalf("authorization URL is missing OIDC state, nonce, or PKCE: %s", authorizationURL)
	}

	identity, err := client.AuthenticateCode(context.Background(), "authorization-code", verifier, nonce)
	if err != nil {
		t.Fatal(err)
	}
	if identity.Issuer != issuer || identity.Subject != "oidc-subject-1" {
		t.Fatalf("verified identity = %#v", identity)
	}
	if _, err := client.AuthenticateCode(context.Background(), "authorization-code", verifier, nonce+"-wrong"); err == nil {
		t.Fatal("OIDC callback accepted an ID token with the wrong nonce")
	}
	idToken = signOIDCTestToken(t, privateKey, issuer, clientID, nonce, time.Now().Add(-11*time.Minute))
	if _, err := client.AuthenticateCode(context.Background(), "authorization-code", verifier, nonce); err == nil {
		t.Fatal("OIDC callback accepted a stale ID token")
	}
}

func TestNewOIDCClientRejectsUntrustedURLs(t *testing.T) {
	for _, config := range []OIDCConfig{
		{Issuer: "http://idp.example.test", ClientID: "client", ClientSecret: "secret", RedirectURL: "https://app.example.test/callback"},
		{Issuer: "https://idp.example.test", ClientID: "client", ClientSecret: "secret", RedirectURL: "http://app.example.test/callback"},
	} {
		if _, err := NewOIDCClient(context.Background(), config); err == nil {
			t.Fatalf("NewOIDCClient accepted untrusted URLs: %#v", config)
		}
	}
}

func signOIDCTestToken(
	t *testing.T,
	key *rsa.PrivateKey,
	issuer, audience, nonce string,
	issuedAt time.Time,
) string {
	t.Helper()
	header, err := json.Marshal(map[string]string{
		"alg": "RS256", "kid": "oidc-test-key", "typ": "JWT",
	})
	if err != nil {
		t.Fatal(err)
	}
	claims, err := json.Marshal(map[string]any{
		"iss": issuer, "sub": "oidc-subject-1", "aud": audience,
		"exp": time.Now().Add(time.Minute).Unix(), "iat": issuedAt.Unix(),
		"nonce": nonce,
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
