package controlplane

import (
	"strings"
	"testing"
	"time"
)

func TestJWTIssueAndVerify(t *testing.T) {
	config := JWTConfig{
		SigningKey: []byte("0123456789abcdef0123456789abcdef"),
		KeyID:      "key-1",
		Issuer:     "https://app.example.test",
		Audience:   "https://api.example.test",
		TTL:        time.Minute,
	}
	principal := Principal{
		Subject:  "password:123e4567-e89b-12d3-a456-426614174000",
		UserID:   "123e4567-e89b-12d3-a456-426614174000",
		TenantID: "123e4567-e89b-12d3-a456-426614174001",
		Role:     Owner,
		Scopes:   map[string]bool{"domains:read": true},
		durable:  true,
	}
	token, _, err := config.Issue(principal)
	if err != nil {
		t.Fatalf("Issue() error = %v", err)
	}
	verified, err := config.Verify(token)
	if err != nil || verified.UserID != principal.UserID || verified.TenantID != principal.TenantID || verified.Role != Owner {
		t.Fatalf("Verify() = %#v, %v", verified, err)
	}

	// Audience mismatch
	wrongAudienceConfig := config
	wrongAudienceConfig.Audience = "https://wrong.example.test"
	if _, err := wrongAudienceConfig.Verify(token); err == nil {
		t.Fatal("Verify accepted token with wrong audience")
	}

	// Key rotation: rotate signing key to key-2, put key-1 in RotationKeys
	rotatedConfig := JWTConfig{
		SigningKey: []byte("new-active-signing-key-32-bytes!!"),
		KeyID:      "key-2",
		RotationKeys: map[string][]byte{
			"key-1": []byte("0123456789abcdef0123456789abcdef"),
		},
		Issuer:   "https://app.example.test",
		Audience: "https://api.example.test",
		TTL:      time.Minute,
	}
	verifiedRotated, err := rotatedConfig.Verify(token)
	if err != nil || verifiedRotated.UserID != principal.UserID {
		t.Fatalf("Verify() failed after key rotation: %v", err)
	}

	// Unknown key ID rejected
	rejectedConfig := JWTConfig{
		SigningKey: []byte("new-active-signing-key-32-bytes!!"),
		KeyID:      "key-2",
		Issuer:     "https://app.example.test",
		Audience:   "https://api.example.test",
		TTL:        time.Minute,
	}
	if _, err := rejectedConfig.Verify(token); err == nil {
		t.Fatal("Verify accepted token with unknown key ID")
	}

	// Tampered token
	parts := strings.Split(token, ".")
	parts[1] = "tampered"
	if _, err := config.Verify(strings.Join(parts, ".")); err == nil {
		t.Fatal("Verify accepted a tampered JWT")
	}
}
