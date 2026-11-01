package controlplane

import (
	"strings"
	"testing"
	"time"
)

func TestJWTIssueAndVerify(t *testing.T) {
	config := JWTConfig{SigningKey: []byte("0123456789abcdef0123456789abcdef"), Issuer: "https://app.example.test", TTL: time.Minute}
	principal := Principal{Subject: "password:123e4567-e89b-12d3-a456-426614174000", UserID: "123e4567-e89b-12d3-a456-426614174000", TenantID: "123e4567-e89b-12d3-a456-426614174001", Role: Owner, Scopes: map[string]bool{"domains:read": true}, durable: true}
	token, _, err := config.Issue(principal)
	if err != nil {
		t.Fatalf("Issue() error = %v", err)
	}
	verified, err := config.Verify(token)
	if err != nil || verified.UserID != principal.UserID || verified.TenantID != principal.TenantID || verified.Role != Owner {
		t.Fatalf("Verify() = %#v, %v", verified, err)
	}
	parts := strings.Split(token, ".")
	parts[1] = "tampered"
	if _, err := config.Verify(strings.Join(parts, ".")); err == nil {
		t.Fatal("Verify accepted a tampered JWT")
	}
}
