package controlplane

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

func TestSMTPCredentialLifecycleIntegration(t *testing.T) {
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
	config.MaxConns = 4
	pool, err := pgxpool.NewWithConfig(context.Background(), config)
	if err != nil {
		t.Fatal("connect to isolated PostgreSQL test database")
	}
	t.Cleanup(pool.Close)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	tenantA := insertTestTenant(t, ctx, pool, "smtp credential tenant a")
	tenantB := insertTestTenant(t, ctx, pool, "smtp credential tenant b")
	tenantIDs := []string{tenantA, tenantB}
	userA := insertTestUser(t, ctx, pool, tenantA, "smtp-credential-owner-a")
	userB := insertTestUser(t, ctx, pool, tenantB, "smtp-credential-owner-b")
	userIDs := []string{userA, userB}
	t.Cleanup(func() {
		cleanupCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if _, err := pool.Exec(cleanupCtx, `
			DELETE FROM smtp_credential_audit_events WHERE tenant_id::text = ANY($1::text[])
		`, tenantIDs); err != nil {
			t.Errorf("remove test SMTP credential audit events: %v", err)
		}
		if _, err := pool.Exec(cleanupCtx, `
			DELETE FROM smtp_credentials WHERE tenant_id::text = ANY($1::text[])
		`, tenantIDs); err != nil {
			t.Errorf("remove test SMTP credentials: %v", err)
		}
		cleanupTestIdentity(t, cleanupCtx, pool, tenantIDs, userIDs)
	})

	repository := NewPostgreSQLRepository(pool)
	ownerA := Principal{
		UserID: userA, TenantID: tenantA, durable: true,
		Scopes: map[string]bool{
			"keys:write":             true,
			"smtp:credentials:read":  true,
			"smtp:credentials:write": true,
		},
	}
	_, secretA, err := repository.CreateAPIKey(
		ctx, ownerA, "smtp-credential-manager",
		[]string{"smtp:credentials:read", "smtp:credentials:write"}, nil,
	)
	if err != nil {
		t.Fatalf("create tenant A API key: %v", err)
	}
	ownerB := Principal{
		UserID: userB, TenantID: tenantB, durable: true,
		Scopes: map[string]bool{
			"keys:write":             true,
			"smtp:credentials:read":  true,
			"smtp:credentials:write": true,
		},
	}
	_, secretB, err := repository.CreateAPIKey(
		ctx, ownerB, "smtp-credential-manager",
		[]string{"smtp:credentials:read", "smtp:credentials:write"}, nil,
	)
	if err != nil {
		t.Fatalf("create tenant B API key: %v", err)
	}
	_, readOnlySecret, err := repository.CreateAPIKey(
		ctx, ownerA, "smtp-credential-reader", []string{"smtp:credentials:read"}, nil,
	)
	if err != nil {
		t.Fatalf("create read-only SMTP credential API key: %v", err)
	}
	handler := (Handler{Store: NewStore(), Postgres: repository}).Routes()

	createRequest := httptest.NewRequest(
		http.MethodPost,
		"/v1/smtp-credentials",
		bytes.NewBufferString(`{"username":"mailer@example.test"}`),
	)
	createRequest.Header.Set("Authorization", "Bearer "+secretA)
	createResponse := httptest.NewRecorder()
	handler.ServeHTTP(createResponse, createRequest)
	if createResponse.Code != http.StatusCreated {
		t.Fatalf("create status=%d body=%s", createResponse.Code, createResponse.Body.String())
	}
	var created struct {
		ID       string `json:"id"`
		Username string `json:"username"`
		AuthCID  string `json:"authcid"`
		Password string `json:"password"`
	}
	if err := json.Unmarshal(createResponse.Body.Bytes(), &created); err != nil {
		t.Fatalf("decode credential response: %v", err)
	}
	if created.ID == "" || created.Username != "mailer@example.test" ||
		created.AuthCID != tenantA+":mailer@example.test" || len(created.Password) < 40 {
		t.Fatalf("unexpected credential response: %#v", created)
	}
	var passwordHash string
	var credentialUserID string
	if err := pool.QueryRow(ctx, `
		SELECT password_hash, user_id::text FROM smtp_credentials WHERE id = $1
	`, created.ID).Scan(&passwordHash, &credentialUserID); err != nil {
		t.Fatalf("read stored credential: %v", err)
	}
	if credentialUserID != userA {
		t.Fatalf("credential user binding=%q, want creator %q", credentialUserID, userA)
	}
	if !strings.HasPrefix(passwordHash, "$argon2id$v=19$m=19456,t=2,p=1$") {
		t.Fatalf("stored credential is not Argon2id with the Rust verifier parameters: %s", passwordHash)
	}

	listRequest := httptest.NewRequest(http.MethodGet, "/v1/smtp-credentials", nil)
	listRequest.Header.Set("Authorization", "Bearer "+secretA)
	listResponse := httptest.NewRecorder()
	handler.ServeHTTP(listResponse, listRequest)
	if listResponse.Code != http.StatusOK || strings.Contains(listResponse.Body.String(), created.Password) {
		t.Fatalf("list response status=%d body=%s", listResponse.Code, listResponse.Body.String())
	}
	readOnlyCreate := httptest.NewRequest(
		http.MethodPost,
		"/v1/smtp-credentials",
		bytes.NewBufferString(`{"username":"read-only-should-fail"}`),
	)
	readOnlyCreate.Header.Set("Authorization", "Bearer "+readOnlySecret)
	readOnlyResponse := httptest.NewRecorder()
	handler.ServeHTTP(readOnlyResponse, readOnlyCreate)
	if readOnlyResponse.Code != http.StatusForbidden {
		t.Fatalf("read-only credential key create status=%d, want 403", readOnlyResponse.Code)
	}

	rotateRequest := httptest.NewRequest(
		http.MethodPost,
		"/v1/smtp-credentials/"+created.ID+"/rotate",
		bytes.NewBufferString("{}"),
	)
	rotateRequest.Header.Set("Authorization", "Bearer "+secretA)
	rotateResponse := httptest.NewRecorder()
	handler.ServeHTTP(rotateResponse, rotateRequest)
	if rotateResponse.Code != http.StatusOK {
		t.Fatalf("rotate status=%d body=%s", rotateResponse.Code, rotateResponse.Body.String())
	}
	var rotated struct {
		Password string `json:"password"`
	}
	if err := json.Unmarshal(rotateResponse.Body.Bytes(), &rotated); err != nil {
		t.Fatalf("decode rotated credential: %v", err)
	}
	if rotated.Password == "" || rotated.Password == created.Password {
		t.Fatal("rotation did not issue a new one-time password")
	}

	crossTenantRequest := httptest.NewRequest(
		http.MethodPost,
		"/v1/smtp-credentials/"+created.ID+"/revoke",
		nil,
	)
	crossTenantRequest.Header.Set("Authorization", "Bearer "+secretB)
	crossTenantResponse := httptest.NewRecorder()
	handler.ServeHTTP(crossTenantResponse, crossTenantRequest)
	if crossTenantResponse.Code != http.StatusNotFound {
		t.Fatalf("cross-tenant revoke status=%d body=%s", crossTenantResponse.Code, crossTenantResponse.Body.String())
	}

	revokeRequest := httptest.NewRequest(
		http.MethodPost,
		"/v1/smtp-credentials/"+created.ID+"/revoke",
		nil,
	)
	revokeRequest.Header.Set("Authorization", "Bearer "+secretA)
	revokeResponse := httptest.NewRecorder()
	handler.ServeHTTP(revokeResponse, revokeRequest)
	if revokeResponse.Code != http.StatusOK {
		t.Fatalf("revoke status=%d body=%s", revokeResponse.Code, revokeResponse.Body.String())
	}
	var auditEvents int
	if err := pool.QueryRow(ctx, `
		SELECT count(*) FROM smtp_credential_audit_events
		WHERE tenant_id = $1 AND credential_id = $2
	`, tenantA, created.ID).Scan(&auditEvents); err != nil {
		t.Fatalf("count credential audit events: %v", err)
	}
	if auditEvents != 3 {
		t.Fatalf("audit event count=%d, want 3", auditEvents)
	}
}
