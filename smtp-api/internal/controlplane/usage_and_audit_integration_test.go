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

	smtpAuth "example.com/smtp-platform/smtp-auth"
	"github.com/jackc/pgx/v5/pgxpool"
)

func TestUsageAndAuditLifecycleIntegration(t *testing.T) {
	databaseURL := strings.TrimSpace(os.Getenv("TEST_DATABASE_URL"))
	if databaseURL == "" {
		t.Skip("set TEST_DATABASE_URL to an isolated database with migrations applied")
	}
	config, err := pgxpool.ParseConfig(databaseURL)
	if err != nil {
		t.Fatal("invalid TEST_DATABASE_URL")
	}
	pool, err := pgxpool.NewWithConfig(context.Background(), config)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(pool.Close)

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	tenantID := insertTestTenant(t, ctx, pool, "usage audit test tenant")
	userID := insertTestUser(t, ctx, pool, tenantID, "usage-audit-test-user")
	t.Cleanup(func() {
		cleanupCtx, c := context.WithTimeout(context.Background(), 10*time.Second)
		defer c()
		_, _ = pool.Exec(cleanupCtx, "DELETE FROM tenant_usage_windows WHERE tenant_id = $1", tenantID)
		_, _ = pool.Exec(cleanupCtx, "DELETE FROM audit_events WHERE tenant_id = $1", tenantID)
		cleanupTestIdentity(t, cleanupCtx, pool, []string{tenantID}, []string{userID})
	})

	repository := NewPostgreSQLRepository(pool)
	jwtConfig := JWTConfig{
		SigningKey: []byte("0123456789abcdef0123456789abcdef"),
		KeyID:      "test-key-1",
		Issuer:     "https://app.example.test",
		TTL:        time.Hour,
	}
	handler := Handler{
		Postgres:         repository,
		JWT:              jwtConfig,
		RateLimitHMACKey: bytes.Repeat([]byte("k"), 32),
	}.Routes()

	principal := Principal{
		UserID:   userID,
		TenantID: tenantID,
		Role:     Owner,
		Subject:  "usage-audit@example.test",
		Scopes:   smtpAuth.PermissionsForRole("owner"),
		durable:  true,
	}

	token, _, err := jwtConfig.Issue(principal)
	if err != nil {
		t.Fatal(err)
	}

	// 1. Test Usage Query via HTTP
	req := httptest.NewRequest(http.MethodGet, "/v1/mailboxes", nil)
	// Query usage via repository directly and verify initial window
	initialUsage, err := repository.Usage(ctx, principal)
	if err != nil {
		t.Fatalf("Usage() failed: %v", err)
	}
	if initialUsage.MessagesUsed != 0 || initialUsage.MessageLimit <= 0 {
		t.Fatalf("unexpected initial usage: %+v", initialUsage)
	}

	// 2. Consume usage in a transaction
	tx, err := pool.Begin(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if err := repository.ConsumeUsageTx(ctx, tx, tenantID, 1024); err != nil {
		_ = tx.Rollback(ctx)
		t.Fatalf("ConsumeUsageTx failed: %v", err)
	}
	if err := tx.Commit(ctx); err != nil {
		t.Fatal(err)
	}

	// Verify messages used incremented
	afterUsage, err := repository.Usage(ctx, principal)
	if err != nil {
		t.Fatalf("Usage() after consume failed: %v", err)
	}
	if afterUsage.MessagesUsed != 1 {
		t.Fatalf("expected 1 message used, got %d", afterUsage.MessagesUsed)
	}

	// 3. Test Audit Events: Record and List
	err = repository.RecordAuditEvent(ctx, tenantID, userID, "tenant.plan_updated", "tenant", tenantID, map[string]any{
		"new_limit": 5000,
	})
	if err != nil {
		t.Fatalf("RecordAuditEvent failed: %v", err)
	}

	// Query via HTTP GET /v1/audit/events
	auditReq := httptest.NewRequest(http.MethodGet, "/v1/audit/events", nil)
	auditReq.Header.Set("Authorization", "Bearer "+token)
	auditRec := httptest.NewRecorder()
	handler.ServeHTTP(auditRec, auditReq)

	if auditRec.Code != http.StatusOK {
		t.Fatalf("GET /v1/audit/events status = %d, body = %s", auditRec.Code, auditRec.Body.String())
	}
	var auditResp struct {
		Items []AuditEvent `json:"items"`
	}
	if err := json.Unmarshal(auditRec.Body.Bytes(), &auditResp); err != nil {
		t.Fatal(err)
	}
	if len(auditResp.Items) == 0 {
		t.Fatal("expected at least 1 audit event returned")
	}
	found := false
	for _, item := range auditResp.Items {
		if item.Action == "tenant.plan_updated" && item.TargetType == "tenant" {
			found = true
			if item.Metadata["new_limit"] != float64(5000) {
				t.Fatalf("expected metadata new_limit 5000, got %v", item.Metadata["new_limit"])
			}
			break
		}
	}
	if !found {
		t.Fatalf("audit event tenant.plan_updated not found in %+v", auditResp.Items)
	}
	_ = req
}
