package controlplane

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestTenantIsolationAndOneTimeKeyMetadata(t *testing.T) {
	s := NewStore()
	a := s.Principal("alice")
	b := s.Principal("bob")
	if a.TenantID == b.TenantID {
		t.Fatal("development principals must be isolated")
	}
	d, err := s.AddDomain(a, "Example.COM.")
	if err != nil || d.Domain != "example.com" {
		t.Fatalf("domain: %#v %v", d, err)
	}
	if _, err := s.AddDomain(Principal{TenantID: b.TenantID, Scopes: map[string]bool{"domains:write": true}}, "example.com"); err != nil {
		t.Fatalf("independent tenant domain should be allowed: %v", err)
	}
	k, secret, err := s.CreateKey(a, "send", []string{"domains:read"}, nil)
	if err != nil || secret == "" {
		t.Fatal(err)
	}
	if len(s.Keys(a)) != 1 || s.Keys(a)[0].Prefix != k.Prefix {
		t.Fatal("key metadata missing")
	}
	if _, err := s.AuthenticateKey(secret); err != nil {
		t.Fatal("key should authenticate")
	}
	exp := time.Now().Add(-time.Minute)
	expired, expiredSecret, err := s.CreateKey(a, "expired", []string{"domains:read"}, &exp)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := s.AuthenticateKey(expiredSecret); err == nil {
		t.Fatal("expired key should not authenticate")
	}
	if err := s.RevokeKey(a, expired.ID); err != nil {
		t.Fatal(err)
	}
	active, activeSecret, err := s.CreateKey(a, "active", []string{"domains:read"}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := s.AuthenticateKey(activeSecret); err != nil {
		t.Fatal("active key should authenticate")
	}
	if err := s.RevokeKey(a, active.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := s.AuthenticateKey(activeSecret); err == nil {
		t.Fatal("revoked key should not authenticate")
	}
}
func TestUnauthenticatedAPI(t *testing.T) {
	h := Handler{Store: NewStore()}.Routes()
	r := httptest.NewRequest(http.MethodGet, "/v1/domains", nil)
	w := httptest.NewRecorder()
	h.ServeHTTP(w, r)
	if w.Code != 401 {
		t.Fatalf("got %d", w.Code)
	}
	if w.Header().Get("X-Request-ID") == "" {
		t.Fatal("request ID missing")
	}
	var response struct {
		Error struct {
			Code      string `json:"code"`
			RequestID string `json:"request_id"`
		} `json:"error"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &response); err != nil {
		t.Fatal(err)
	}
	if response.Error.Code != "UNAUTHORIZED" || response.Error.RequestID == "" {
		t.Fatalf("unexpected error response: %s", w.Body.String())
	}
}

func TestAPIRequestIDValidationAndStrictJSON(t *testing.T) {
	h := Handler{Store: NewStore(), DevMode: true}.Routes()
	request := httptest.NewRequest(http.MethodPost, "/v1/domains", bytes.NewBufferString(`{"domain":"example.com","unexpected":true}`))
	request.Header.Set("X-Dev-Subject", "strict-user")
	request.Header.Set("X-Request-ID", "client-request-1")
	request.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	h.ServeHTTP(w, request)
	if w.Code != http.StatusBadRequest {
		t.Fatalf("got %d", w.Code)
	}
	if got := w.Header().Get("X-Request-ID"); got != "client-request-1" {
		t.Fatalf("request ID was not preserved: %q", got)
	}
}

func TestInvalidPaginationIsRejected(t *testing.T) {
	h := Handler{Store: NewStore(), DevMode: true}.Routes()
	request := httptest.NewRequest(http.MethodGet, "/v1/domains?limit=101", nil)
	request.Header.Set("X-Dev-Subject", "pagination-user")
	w := httptest.NewRecorder()
	h.ServeHTTP(w, request)
	if w.Code != http.StatusBadRequest {
		t.Fatalf("got %d", w.Code)
	}
}

type fakeDNS struct{}

func (fakeDNS) TXT(domain string) ([]string, error) {
	if domain == "_dmarc.example.com" {
		return []string{"v=DMARC1; p=none"}, nil
	}
	return []string{"v=spf1 include:example.test ~all"}, nil
}
func (fakeDNS) MX(string) ([]string, error) { return []string{"mx.example.test"}, nil }
func TestDomainChecksAndQuota(t *testing.T) {
	s := NewStore()
	p := s.Principal("dns-user")
	d, err := s.AddDomain(p, "example.com")
	if err != nil {
		t.Fatal(err)
	}
	check, err := s.CheckDomain(p, d.ID, fakeDNS{})
	if err != nil || len(check.Results) != 4 {
		t.Fatalf("check=%#v err=%v", check, err)
	}
	if err := s.ConsumeMessage(p); err != nil {
		t.Fatal(err)
	}
	if s.Usage(p).MessagesUsed != 1 {
		t.Fatal("usage not counted")
	}
}
