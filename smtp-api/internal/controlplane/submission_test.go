package controlplane

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

type fixedVerificationTXTResolver struct {
	name  string
	value string
}

func (resolver fixedVerificationTXTResolver) LookupTXT(name string) ([]string, error) {
	if name != resolver.name {
		return nil, fmt.Errorf("unexpected TXT lookup name %q", name)
	}
	return []string{resolver.value}, nil
}

type fixedStoreTXTResolver struct {
	name   string
	values []string
}

func (resolver fixedStoreTXTResolver) LookupTXT(name string) ([]string, error) {
	if name != resolver.name {
		return nil, fmt.Errorf("unexpected TXT lookup name %q", name)
	}
	return resolver.values, nil
}

func TestNormalizeSubmissionAndBuildMessage(t *testing.T) {
	submission, err := normalizeSubmission(submissionRequest{
		From:    "sender@example.test",
		To:      []string{"person@example.test", "other@example.test"},
		Subject: "Hello, 世界",
		Text:    "first line\nsecond line",
	})
	if err != nil {
		t.Fatal(err)
	}
	if submission.Text != "first line\r\nsecond line" {
		t.Fatalf("newlines were not normalized: %q", submission.Text)
	}
	message := submissionMessage(submission, "00000000-0000-0000-0000-000000000001", time.Date(2026, 1, 2, 3, 4, 5, 0, time.UTC))
	if !bytes.Contains(message, []byte("Content-Transfer-Encoding: base64\r\n")) ||
		bytes.Contains(message, []byte("Hello, 世界")) ||
		!bytes.Contains(message, []byte("Message-ID: <00000000-0000-0000-0000-000000000001@smtp-platform.local>\r\n")) {
		t.Fatalf("message headers or encoding are incorrect: %q", message)
	}
	subject, err := submissionSubject(message)
	if err != nil || subject != submission.Subject {
		t.Fatalf("decoded subject=%q err=%v", subject, err)
	}
}

func TestStoreDomainChallengeLooksUpVerificationRecord(t *testing.T) {
	store := NewStore()
	principal := store.Principal("domain-challenge-test")
	domain, err := store.AddDomain(principal, "example.test")
	if err != nil {
		t.Fatal(err)
	}
	token, err := store.Challenge(principal, domain.ID)
	if err != nil {
		t.Fatal(err)
	}
	resolver := fixedStoreTXTResolver{
		name:   "_smtp-platform-verification.example.test",
		values: []string{"smtp-platform-verification=" + sha256Hex(token)},
	}
	if err := store.CheckChallenge(principal, domain.ID, resolver); err != nil {
		t.Fatalf("check verification challenge: %v", err)
	}
}

func TestNormalizeSubmissionRejectsUnsafeInput(t *testing.T) {
	tests := []submissionRequest{
		{From: "sender@example.test\r\nBcc: other@example.test", To: []string{"person@example.test"}},
		{From: "sender@example.test", To: []string{"person@example.test\r\nBcc: other@example.test"}},
		{From: "not-an-address", To: []string{"person@example.test"}},
		{From: "sender@example.test", To: []string{"person@example.test", "PERSON@example.test"}},
		{From: "sender@example.test", To: []string{"person@example.test"}, Subject: "safe\r\nBcc: other@example.test"},
		{From: "sender@example.test", To: []string{"person@example.test"}, Text: "body\x00"},
	}
	for _, input := range tests {
		if _, err := normalizeSubmission(input); err == nil {
			t.Errorf("accepted unsafe submission: %#v", input)
		}
	}

	tooMany := make([]string, maxSubmissionRecipients+1)
	for index := range tooMany {
		tooMany[index] = "person" + strings.Repeat("a", index%2) + "@example.test"
	}
	if _, err := normalizeSubmission(submissionRequest{
		From: "sender@example.test",
		To:   tooMany,
	}); err == nil {
		t.Fatal("accepted too many recipients")
	}
}

func TestIdempotencyKeyValidation(t *testing.T) {
	for _, value := range []string{"submit-1", "request:alpha_2.3"} {
		if !validIdempotencyKey(value) {
			t.Errorf("rejected valid idempotency key %q", value)
		}
	}
	for _, value := range []string{"", "white space", "bad/key", "line\nbreak", strings.Repeat("a", 257)} {
		if validIdempotencyKey(value) {
			t.Errorf("accepted invalid idempotency key %q", value)
		}
	}
}

func TestSubmissionHandlerRequiresWriteScopeAndIdempotencyKey(t *testing.T) {
	store := NewStore()
	handler := Handler{Store: store, DevMode: true}.Routes()
	payload, err := json.Marshal(submissionRequest{
		From: "sender@example.test",
		To:   []string{"person@example.test"},
		Text: "hello",
	})
	if err != nil {
		t.Fatal(err)
	}

	request := httptest.NewRequest(http.MethodPost, "/v1/submissions", bytes.NewReader(payload))
	request.Header.Set("X-Dev-Subject", "submission-test")
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusBadRequest {
		t.Fatalf("missing idempotency key returned %d: %s", response.Code, response.Body)
	}

	principal := store.Principal("read-only-submission-test")
	_, key, err := store.CreateKey(principal, "read-only", []string{"delivery:read"}, nil)
	if err != nil {
		t.Fatal(err)
	}
	request = httptest.NewRequest(http.MethodPost, "/v1/submissions", bytes.NewReader(payload))
	request.Header.Set("Authorization", "Bearer "+key)
	request.Header.Set("Idempotency-Key", "submit-1")
	response = httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusForbidden {
		t.Fatalf("read-only key returned %d: %s", response.Code, response.Body)
	}
}

func TestSubmissionHandlerReportsMissingQueue(t *testing.T) {
	handler := Handler{Store: NewStore(), DevMode: true}.Routes()
	request := httptest.NewRequest(
		http.MethodGet,
		"/v1/submissions",
		nil,
	)
	request.Header.Set("X-Dev-Subject", "submission-list-test")
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusServiceUnavailable {
		t.Fatalf("missing queue returned %d: %s", response.Code, response.Body)
	}
}

func TestDashboardResourceJSONUsesSnakeCase(t *testing.T) {
	store := NewStore()
	principal := store.Principal("dashboard-json-test")
	if _, err := store.AddDomain(principal, "example.test"); err != nil {
		t.Fatal(err)
	}
	if _, _, err := store.CreateKey(principal, "dashboard", []string{"delivery:read"}, nil); err != nil {
		t.Fatal(err)
	}
	handler := Handler{Store: store, DevMode: true}.Routes()

	domainRequest := httptest.NewRequest(http.MethodGet, "/v1/domains", nil)
	domainRequest.Header.Set("X-Dev-Subject", "dashboard-json-test")
	domainResponse := httptest.NewRecorder()
	handler.ServeHTTP(domainResponse, domainRequest)
	var domains struct {
		Items []map[string]any `json:"items"`
	}
	if err := json.Unmarshal(domainResponse.Body.Bytes(), &domains); err != nil {
		t.Fatal(err)
	}
	if domainResponse.Code != http.StatusOK || len(domains.Items) != 1 ||
		domains.Items[0]["id"] == nil || domains.Items[0]["domain"] != "example.test" {
		t.Fatalf("unexpected domain response: %s", domainResponse.Body)
	}

	keyRequest := httptest.NewRequest(http.MethodGet, "/v1/api-keys", nil)
	keyRequest.Header.Set("X-Dev-Subject", "dashboard-json-test")
	keyResponse := httptest.NewRecorder()
	handler.ServeHTTP(keyResponse, keyRequest)
	var keys struct {
		Items []map[string]any `json:"items"`
	}
	if err := json.Unmarshal(keyResponse.Body.Bytes(), &keys); err != nil {
		t.Fatal(err)
	}
	if keyResponse.Code != http.StatusOK || len(keys.Items) != 1 ||
		keys.Items[0]["id"] == nil || keys.Items[0]["prefix"] == nil ||
		keys.Items[0]["TenantID"] != nil || keys.Items[0]["tenant_id"] != nil {
		t.Fatalf("unexpected API key response: %s", keyResponse.Body)
	}
}

func TestPostgreSQLSubmissionQueueIntegration(t *testing.T) {
	databaseURL := strings.TrimSpace(os.Getenv("TEST_DATABASE_URL"))
	if databaseURL == "" {
		t.Skip("set TEST_DATABASE_URL to an isolated database with migrations applied")
	}
	config, err := pgxpool.ParseConfig(databaseURL)
	if err != nil {
		t.Fatal("invalid TEST_DATABASE_URL")
	}
	if !strings.Contains(strings.ToLower(config.ConnConfig.Database), "test") {
		t.Fatal("TEST_DATABASE_URL must identify an isolated test database")
	}
	config.MaxConns = 3
	pool, err := pgxpool.NewWithConfig(context.Background(), config)
	if err != nil {
		t.Fatal("connect to isolated PostgreSQL test database")
	}
	t.Cleanup(pool.Close)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := pool.Ping(ctx); err != nil {
		t.Fatal("ping isolated PostgreSQL test database")
	}

	id, err := newID()
	if err != nil {
		t.Fatal(err)
	}
	repository := NewPostgreSQLRepository(pool)
	tenantID := insertTestTenant(t, ctx, pool, "submission integration tenant")
	userID := insertTestUser(t, ctx, pool, tenantID, "submission-integration-user-"+id)
	principal := Principal{
		TenantID: tenantID,
		UserID:   userID,
		durable:  true,
	}
	t.Cleanup(func() {
		cleanupCtx, cleanupCancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cleanupCancel()
		cleanupTestIdentity(t, cleanupCtx, pool, []string{tenantID}, []string{userID})
	})
	idempotencyKey := "integration-" + id
	t.Cleanup(func() {
		cleanupCtx, cleanupCancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cleanupCancel()
		if _, err := pool.Exec(cleanupCtx, `
			DELETE FROM mail_messages
			WHERE id IN (
				SELECT message_id FROM mail_idempotency
				WHERE scope = $1 AND idempotency_key = $2
			)
		`, "api:"+principal.TenantID, idempotencyKey); err != nil {
			t.Errorf("clean up queued submission: %v", err)
		}
	})

	submission, err := normalizeSubmission(submissionRequest{
		From:    "sender@example.test",
		To:      []string{"person@example.test", "other@example.test"},
		Subject: "Durable submission",
		Text:    "Queue contract test",
	})
	if err != nil {
		t.Fatal(err)
	}
	if _, _, err := repository.EnqueueSubmission(ctx, principal, idempotencyKey, submission); !errors.Is(err, ErrForbidden) {
		t.Fatalf("unverified sender domain enqueue error=%v, want forbidden", err)
	}
	domain, err := repository.CreateDomain(ctx, principal, "example.test")
	if err != nil {
		t.Fatalf("create test sender domain: %v", err)
	}
	challenge, err := repository.CreateDomainChallenge(ctx, principal, domain.ID)
	if err != nil {
		t.Fatalf("create sender verification challenge: %v", err)
	}
	resolver := fixedVerificationTXTResolver{
		name:  "_smtp-platform-verification.example.test",
		value: challenge.RecordValue,
	}
	if err := repository.CheckDomainChallenge(ctx, principal, domain.ID, resolver); err != nil {
		t.Fatalf("verify sender domain: %v", err)
	}
	first, created, err := repository.EnqueueSubmission(ctx, principal, idempotencyKey, submission)
	if err != nil || !created || first.Status != "queued" {
		t.Fatalf("first enqueue=%#v created=%t err=%v", first, created, err)
	}
	if _, err := pool.Exec(ctx, `
		INSERT INTO mail_attempts
		  (message_id, attempt_number, started_at, finished_at, outcome)
		VALUES ($1, 1, NOW(), NOW(), 'simulated_success')
	`, first.ID); err != nil {
		t.Fatalf("insert simulated attempt fixture: %v", err)
	}
	attempts, err := repository.ListSubmissionAttempts(ctx, principal, first.ID)
	if err != nil || len(attempts) != 1 || attempts[0].Outcome != "simulated_success" {
		t.Fatalf("submission attempts=%#v err=%v", attempts, err)
	}
	replayed, created, err := repository.EnqueueSubmission(ctx, principal, idempotencyKey, submission)
	if err != nil || created || replayed.ID != first.ID {
		t.Fatalf("replayed enqueue=%#v created=%t err=%v", replayed, created, err)
	}
	changed := submission
	changed.Subject = "Different content"
	if _, _, err := repository.EnqueueSubmission(ctx, principal, idempotencyKey, changed); err != errIdempotencyConflict {
		t.Fatalf("expected idempotency conflict, got %v", err)
	}
	items, err := repository.ListSubmissions(ctx, principal, 20)
	if err != nil || len(items) != 1 || items[0].ID != first.ID || items[0].Subject != submission.Subject {
		t.Fatalf("submission history=%#v err=%v", items, err)
	}
	otherTenant := Principal{TenantID: "other-" + id}
	if _, err := repository.GetSubmission(ctx, otherTenant, first.ID); err != ErrNotFound {
		t.Fatalf("cross-tenant submission lookup returned %v", err)
	}
	if _, err := repository.ListSubmissionAttempts(ctx, otherTenant, first.ID); err != ErrNotFound {
		t.Fatalf("cross-tenant activity lookup returned %v", err)
	}
}

func TestSubmissionAttemptsRejectsInvalidIDsAndMissingQueue(t *testing.T) {
	handler := Handler{Store: NewStore(), DevMode: true}.Routes()

	request := httptest.NewRequest(http.MethodGet, "/v1/submissions/not-a-uuid/attempts", nil)
	request.Header.Set("X-Dev-Subject", "activity-test")
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusBadRequest {
		t.Fatalf("invalid submission ID status=%d, want %d", response.Code, http.StatusBadRequest)
	}

	request = httptest.NewRequest(
		http.MethodGet,
		"/v1/submissions/00000000-0000-4000-8000-000000000001/attempts",
		nil,
	)
	request.Header.Set("X-Dev-Subject", "activity-test")
	response = httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusServiceUnavailable {
		t.Fatalf("missing PostgreSQL status=%d, want %d", response.Code, http.StatusServiceUnavailable)
	}
}
