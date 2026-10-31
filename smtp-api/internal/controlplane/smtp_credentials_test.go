package controlplane

import (
	"encoding/base64"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"golang.org/x/crypto/argon2"
)

func TestSMTPCredentialPasswordHashMatchesRustParameters(t *testing.T) {
	const password = "test-generated-smtp-password"
	encoded, err := hashSMTPCredentialPassword(password)
	if err != nil {
		t.Fatal(err)
	}
	fields := strings.Split(encoded, "$")
	if len(fields) != 6 || fields[1] != "argon2id" || fields[2] != "v=19" ||
		fields[3] != "m=19456,t=2,p=1" {
		t.Fatalf("unexpected Argon2id PHC parameters: %q", encoded)
	}
	salt, err := base64.RawStdEncoding.DecodeString(fields[4])
	if err != nil {
		t.Fatalf("decode salt: %v", err)
	}
	expected, err := base64.RawStdEncoding.DecodeString(fields[5])
	if err != nil {
		t.Fatalf("decode hash: %v", err)
	}
	actual := argon2.IDKey([]byte(password), salt, 2, 19_456, 1, 32)
	if string(actual) != string(expected) {
		t.Fatal("PHC hash does not verify with the Rust SMTP Argon2id parameters")
	}
}

func TestSMTPCredentialHashingIsConcurrencyBounded(t *testing.T) {
	for range smtpCredentialHashParallel {
		smtpCredentialHashSlots <- struct{}{}
	}
	defer func() {
		for range smtpCredentialHashParallel {
			<-smtpCredentialHashSlots
		}
	}()
	if _, err := hashSMTPCredentialPasswordBounded("test-generated-smtp-password"); !errors.Is(err, ErrRateLimited) {
		t.Fatalf("hashing under full load error=%v, want rate limited", err)
	}
}

func TestSMTPCredentialUsernameAndExpiryBounds(t *testing.T) {
	username, err := normalizeSMTPCredentialUsername(" Alice+SMTP@Example ")
	if err != nil || username != "alice+smtp@example" {
		t.Fatalf("normalize username=%q err=%v", username, err)
	}
	for _, invalid := range []string{"", "white space", "bad:name", strings.Repeat("a", 129)} {
		if _, err := normalizeSMTPCredentialUsername(invalid); err == nil {
			t.Errorf("accepted invalid username %q", invalid)
		}
	}
	if err := validateSMTPCredentialExpiry(nil); err == nil {
		t.Fatal("accepted a missing credential expiry")
	}
	past := time.Now().Add(-time.Minute)
	if err := validateSMTPCredentialExpiry(&past); err == nil {
		t.Fatal("accepted an expired credential")
	}
	tooFar := time.Now().AddDate(1, 0, 1)
	if err := validateSMTPCredentialExpiry(&tooFar); err == nil {
		t.Fatal("accepted credential expiry more than one year away")
	}
}

func TestDevelopmentIdentityCannotManageSMTPCredentials(t *testing.T) {
	handler := Handler{
		Store:    NewStore(),
		Postgres: &PostgreSQLRepository{},
		DevMode:  true,
	}.Routes()
	request := httptest.NewRequest(http.MethodGet, "/v1/smtp-credentials", nil)
	request.Header.Set("X-Dev-Subject", "local-dashboard-user")
	response := httptest.NewRecorder()

	handler.ServeHTTP(response, request)

	if response.Code != http.StatusUnauthorized {
		t.Fatalf("development identity status=%d, want %d", response.Code, http.StatusUnauthorized)
	}
	if !strings.Contains(response.Body.String(), "durable authenticated tenant") {
		t.Fatalf("development identity response lacks setup guidance: %s", response.Body.String())
	}
}
