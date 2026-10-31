package controlplane

import (
	"bytes"
	"context"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
	"time"

	"example.com/smtp-platform/smtp-api/internal/identity"
	"github.com/jackc/pgx/v5/pgxpool"
)

func TestPostgreSQLPoolConfigValidation(t *testing.T) {
	tests := []struct {
		name string
		env  map[string]string
		want bool
	}{
		{
			name: "missing URL",
			env:  map[string]string{},
			want: false,
		},
		{
			name: "invalid pool limit",
			env: map[string]string{
				"DATABASE_URL":                 "postgres://localhost/db",
				"API_DATABASE_MAX_CONNECTIONS": "0",
			},
			want: false,
		},
		{
			name: "local development database",
			env: map[string]string{
				"DATABASE_URL": "postgres://localhost/db?sslmode=disable",
			},
			want: true,
		},
		{
			name: "remote production database requires TLS",
			env: map[string]string{
				"DATABASE_URL":    "postgres://db.example.test/db?sslmode=disable",
				"APP_ENVIRONMENT": "production",
			},
			want: false,
		},
		{
			name: "remote production database rejects TLS fallback",
			env: map[string]string{
				"DATABASE_URL":    "postgres://db.example.test/db?sslmode=prefer",
				"APP_ENVIRONMENT": "production",
			},
			want: false,
		},
		{
			name: "remote production database with TLS",
			env: map[string]string{
				"DATABASE_URL":    "postgres://db.example.test/db?sslmode=verify-full",
				"APP_ENVIRONMENT": "production",
			},
			want: true,
		},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			config, err := PostgreSQLPoolConfigFromEnv(func(key string) string {
				return test.env[key]
			})
			if (err == nil) != test.want {
				t.Fatalf("config_valid=%t err=%v, want success=%v", config != nil, err, test.want)
			}
			if err == nil && (config.MaxConns < 1 || config.MaxConns > maximumDatabaseConnections) {
				t.Fatalf("unexpected pool limit: %d", config.MaxConns)
			}
		})
	}
}

func TestDevelopmentIdentityCannotReadPostgreSQLMailboxes(t *testing.T) {
	handler := Handler{
		Store:    NewStore(),
		Postgres: &PostgreSQLRepository{},
		DevMode:  true,
	}.Routes()
	request := httptest.NewRequest(http.MethodGet, "/v1/mailboxes", nil)
	request.Header.Set("X-Dev-Subject", "local-dashboard-user")
	response := httptest.NewRecorder()

	handler.ServeHTTP(response, request)

	if response.Code != http.StatusUnauthorized {
		t.Fatalf("development identity mailbox status=%d, want %d", response.Code, http.StatusUnauthorized)
	}
	if !strings.Contains(response.Body.String(), "user-bound API key") {
		t.Fatalf("development identity mailbox response lacks setup guidance: %s", response.Body.String())
	}
}

func TestValidAPIKey(t *testing.T) {
	key := "sp_" + strings.Repeat("a", 43)
	if !validAPIKey(key) {
		t.Fatal("expected correctly sized base64url API key to be valid")
	}
	for _, invalid := range []string{
		"",
		"sp_short",
		"sp_" + strings.Repeat("a", 42),
		key + "\n",
	} {
		if validAPIKey(invalid) {
			t.Errorf("accepted invalid key %q", invalid)
		}
	}
}

func TestPostgreSQLMailboxReadIntegration(t *testing.T) {
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
	if err := pool.Ping(ctx); err != nil {
		t.Fatal("ping isolated PostgreSQL test database")
	}

	tenantA := insertTestTenant(t, ctx, pool, "mailbox api tenant a")
	tenantB := insertTestTenant(t, ctx, pool, "mailbox api tenant b")
	tenantIDs := []string{tenantA, tenantB}
	userIDs := make([]string, 0, 4)
	t.Cleanup(func() {
		cleanupCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		cleanupTestIdentity(t, cleanupCtx, pool, tenantIDs, userIDs)
	})
	ownerA := insertTestUser(t, ctx, pool, tenantA, "mailbox-api-owner-a")
	userIDs = append(userIDs, ownerA)
	const keycloakIssuer = "https://idp.example.test/realms/smtp"
	if _, err := pool.Exec(ctx, `
		UPDATE users SET identity_issuer = $2 WHERE id = $1
	`, ownerA, keycloakIssuer); err != nil {
		t.Fatalf("configure test user's Keycloak issuer: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		INSERT INTO tenant_memberships (tenant_id, user_id, role)
		VALUES ($1, $2, 'member')
	`, tenantB, ownerA); err != nil {
		t.Fatalf("add second tenant membership: %v", err)
	}
	ownerB := insertTestUser(t, ctx, pool, tenantB, "mailbox-api-owner-b")
	userIDs = append(userIDs, ownerB)
	unauthorizedUser := insertTestUser(t, ctx, pool, tenantA, "mailbox-api-no-grant")
	userIDs = append(userIDs, unauthorizedUser)
	removedMember := insertTestUser(t, ctx, pool, tenantA, "mailbox-api-removed-member")
	userIDs = append(userIDs, removedMember)
	mailboxID, messageID := insertTestMailboxMessage(t, ctx, pool, tenantA)
	emptyMailboxID, _ := insertTestMailboxMessage(t, ctx, pool, tenantA)
	if _, err := pool.Exec(ctx, `
		DELETE FROM mailbox_messages WHERE mailbox_id = $1
	`, emptyMailboxID); err != nil {
		t.Fatalf("prepare empty mailbox fixture: %v", err)
	}

	grantMailboxRead(t, ctx, pool, tenantA, mailboxID, ownerA)
	grantMailboxRead(t, ctx, pool, tenantA, emptyMailboxID, ownerA)
	secretA := insertTestAPIKey(t, ctx, pool, tenantA, ownerA, []string{"mailbox:read"})
	secretB := insertTestAPIKey(t, ctx, pool, tenantB, ownerB, []string{"mailbox:read"})
	noScopeSecret := insertTestAPIKey(t, ctx, pool, tenantA, ownerA, []string{"domains:read"})
	deliveryOnlySecret := insertTestAPIKey(t, ctx, pool, tenantA, ownerA, []string{"delivery:read"})
	noGrantSecret := insertTestAPIKey(t, ctx, pool, tenantA, unauthorizedUser, []string{"mailbox:read"})
	revokedSecret := insertTestAPIKey(t, ctx, pool, tenantA, ownerA, []string{"mailbox:read"})
	expiredSecret := insertTestAPIKey(t, ctx, pool, tenantA, ownerA, []string{"mailbox:read"})
	adminSecret := insertTestAPIKey(t, ctx, pool, tenantA, ownerA, []string{"mailbox:read", "keys:write"})
	legacySecret := insertLegacyTenantAPIKey(
		t, ctx, pool, tenantA, []string{"delivery:read", "domains:read"},
	)
	orphanedSecret := insertTestAPIKey(t, ctx, pool, tenantA, removedMember, []string{"mailbox:read"})
	if _, err := pool.Exec(ctx, `
		DELETE FROM tenant_memberships
		WHERE tenant_id = $1 AND user_id = $2
	`, tenantA, removedMember); err != nil {
		t.Fatalf("remove API key owner's membership: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		UPDATE api_keys SET revoked_at = NOW() WHERE prefix = $1
	`, revokedSecret[:apiKeyPrefixLength]); err != nil {
		t.Fatalf("revoke API key fixture: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		UPDATE api_keys SET expires_at = NOW() - INTERVAL '1 minute' WHERE prefix = $1
	`, expiredSecret[:apiKeyPrefixLength]); err != nil {
		t.Fatalf("expire API key fixture: %v", err)
	}

	repository := NewPostgreSQLRepository(pool)
	handler := Handler{
		Store:    NewStore(),
		Postgres: repository,
		OIDC:     fixedOIDCVerifier{claims: identity.Claims{Issuer: keycloakIssuer, Subject: "mailbox-api-owner-a"}},
		DevMode:  true,
	}.Routes()
	if _, err := repository.AuthenticateOIDCSubject(ctx, keycloakIssuer, "mailbox-api-owner-a", ""); !errors.Is(err, ErrTenantSelectionRequired) {
		t.Fatalf("OIDC authentication without tenant selection error=%v, want tenant selection required", err)
	}
	oidcPrincipal, err := repository.AuthenticateOIDCSubject(ctx, keycloakIssuer, "mailbox-api-owner-a", tenantA)
	if err != nil {
		t.Fatalf("authenticate provisioned Keycloak user: %v", err)
	}
	if oidcPrincipal.UserID != ownerA || oidcPrincipal.TenantID != tenantA ||
		!oidcPrincipal.Scopes["domains:write"] || !oidcPrincipal.Scopes["delivery:read"] {
		t.Fatalf("OIDC principal has unexpected membership permissions: %#v", oidcPrincipal)
	}
	memberPrincipal, err := repository.AuthenticateOIDCSubject(ctx, keycloakIssuer, "mailbox-api-owner-a", tenantB)
	if err != nil {
		t.Fatalf("authenticate user as tenant member: %v", err)
	}
	if memberPrincipal.Role != Member || memberPrincipal.Scopes["domains:write"] || memberPrincipal.Scopes["keys:write"] {
		t.Fatalf("member role received elevated permissions: %#v", memberPrincipal)
	}
	if _, err := repository.AuthenticateOIDCSubject(ctx, keycloakIssuer, "mailbox-api-owner-a", tenantIDs[0]+"-invalid"); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("OIDC authentication with malformed tenant selector error=%v, want unauthorized", err)
	}
	legacyPrincipal, err := repository.AuthenticateAPIKey(ctx, legacySecret)
	if err != nil {
		t.Fatalf("authenticate legacy tenant key for existing tenant-scoped API access: %v", err)
	}
	if legacyPrincipal.TenantID != tenantA || legacyPrincipal.UserID != "" ||
		!legacyPrincipal.Scopes["delivery:read"] {
		t.Fatalf("legacy API key resolved to unexpected principal: %#v", legacyPrincipal)
	}
	if _, err := repository.AuthenticateAPIKey(ctx, orphanedSecret); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("key after owner membership removal error=%v, want unauthorized", err)
	}
	tamperedSecret := []byte(secretA)
	if tamperedSecret[apiKeyPrefixLength] == 'A' {
		tamperedSecret[apiKeyPrefixLength] = 'B'
	} else {
		tamperedSecret[apiKeyPrefixLength] = 'A'
	}
	if _, err := repository.AuthenticateAPIKey(ctx, string(tamperedSecret)); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("tampered key authentication error=%v, want unauthorized", err)
	}
	request := func(path, secret string) *httptest.ResponseRecorder {
		req := httptest.NewRequest(http.MethodGet, path, nil)
		if secret != "" {
			req.Header.Set("Authorization", "Bearer "+secret)
		}
		response := httptest.NewRecorder()
		handler.ServeHTTP(response, req)
		return response
	}

	if response := request("/v1/mailboxes", secretA); response.Code != http.StatusOK {
		t.Fatalf("authorized mailbox list status=%d body=%s", response.Code, response.Body.String())
	} else {
		assertJSONItems(t, response.Body.Bytes(), 2)
	}
	if response := request("/v1/mailboxes?limit=1", secretA); response.Code != http.StatusOK {
		t.Fatalf("bounded mailbox list status=%d body=%s", response.Code, response.Body.String())
	} else {
		assertJSONItems(t, response.Body.Bytes(), 1)
	}
	if response := request("/v1/mailboxes?limit=101", secretA); response.Code != http.StatusBadRequest {
		t.Fatalf("mailbox list with oversized limit status=%d, want 400", response.Code)
	}
	if response := request("/v1/me", secretA); response.Code != http.StatusOK {
		t.Fatalf("durable current-principal status=%d body=%s", response.Code, response.Body.String())
	}
	oidcRequest := httptest.NewRequest(http.MethodGet, "/v1/me", nil)
	oidcRequest.Header.Set("Authorization", "Bearer keycloak-access-token")
	oidcRequestWithoutTenant := httptest.NewRequest(http.MethodGet, "/v1/me", nil)
	oidcRequestWithoutTenant.Header.Set("Authorization", "Bearer keycloak-access-token")
	oidcResponseWithoutTenant := httptest.NewRecorder()
	handler.ServeHTTP(oidcResponseWithoutTenant, oidcRequestWithoutTenant)
	if oidcResponseWithoutTenant.Code != http.StatusBadRequest {
		t.Fatalf("OIDC current-principal without tenant status=%d body=%s", oidcResponseWithoutTenant.Code, oidcResponseWithoutTenant.Body.String())
	}
	oidcRequest.Header.Set("X-Tenant-ID", tenantA)
	oidcResponse := httptest.NewRecorder()
	handler.ServeHTTP(oidcResponse, oidcRequest)
	if oidcResponse.Code != http.StatusOK {
		t.Fatalf("OIDC current-principal status=%d body=%s", oidcResponse.Code, oidcResponse.Body.String())
	}
	if response := request("/v1/me", legacySecret); response.Code != http.StatusOK {
		t.Fatalf("legacy tenant-key current-principal status=%d body=%s", response.Code, response.Body.String())
	} else {
		var current struct {
			UserID string `json:"user_id"`
		}
		if err := json.Unmarshal(response.Body.Bytes(), &current); err != nil {
			t.Fatal(err)
		}
		if current.UserID != "" {
			t.Fatalf("legacy tenant key resolved user_id=%q, want no user identity", current.UserID)
		}
	}
	if response := request("/v1/tenants/current", legacySecret); response.Code != http.StatusOK {
		t.Fatalf("legacy tenant-key current tenant status=%d body=%s", response.Code, response.Body.String())
	}
	if response := request("/v1/domains", legacySecret); response.Code != http.StatusOK {
		t.Fatalf("legacy tenant-key scoped API status=%d body=%s", response.Code, response.Body.String())
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", secretA); response.Code != http.StatusOK {
		t.Fatalf("authorized message list status=%d body=%s", response.Code, response.Body.String())
	} else {
		assertJSONItems(t, response.Body.Bytes(), 1)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages/"+messageID, secretA); response.Code != http.StatusOK {
		t.Fatalf("authorized message get status=%d body=%s", response.Code, response.Body.String())
	} else {
		var result MessageSummary
		if err := json.Unmarshal(response.Body.Bytes(), &result); err != nil {
			t.Fatal(err)
		}
		if result.ID != messageID || result.Subject != "PostgreSQL integration message" {
			t.Fatalf("unexpected message response: %#v", result)
		}
	}
	if response := request("/v1/mailboxes/"+emptyMailboxID+"/messages", secretA); response.Code != http.StatusOK {
		t.Fatalf("empty authorized mailbox status=%d body=%s", response.Code, response.Body.String())
	} else {
		assertJSONItems(t, response.Body.Bytes(), 0)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", secretB); response.Code != http.StatusNotFound {
		t.Fatalf("cross-tenant mailbox list status=%d, want 404", response.Code)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", noGrantSecret); response.Code != http.StatusNotFound {
		t.Fatalf("user without mailbox permission status=%d, want 404", response.Code)
	}
	if response := request("/v1/mailboxes/not-a-uuid/messages", secretA); response.Code != http.StatusNotFound {
		t.Fatalf("malformed mailbox ID status=%d, want 404", response.Code)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", noScopeSecret); response.Code != http.StatusForbidden {
		t.Fatalf("insufficient key scope status=%d, want 403", response.Code)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", deliveryOnlySecret); response.Code != http.StatusForbidden {
		t.Fatalf("delivery-only key scope status=%d, want 403", response.Code)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", revokedSecret); response.Code != http.StatusUnauthorized {
		t.Fatalf("revoked API key status=%d, want 401", response.Code)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", expiredSecret); response.Code != http.StatusUnauthorized {
		t.Fatalf("expired API key status=%d, want 401", response.Code)
	}
	if response := request("/v1/mailboxes", legacySecret); response.Code != http.StatusForbidden {
		t.Fatalf("tenant-only legacy mailbox list status=%d, want 403", response.Code)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", legacySecret); response.Code != http.StatusForbidden {
		t.Fatalf("tenant-only legacy message list status=%d, want 403", response.Code)
	}
	if response := request(
		"/v1/mailboxes/"+mailboxID+"/messages/"+messageID, legacySecret,
	); response.Code != http.StatusForbidden {
		t.Fatalf("tenant-only legacy message get status=%d, want 403", response.Code)
	}
	if response := request("/v1/mailboxes/"+mailboxID+"/messages", ""); response.Code != http.StatusUnauthorized {
		t.Fatalf("missing authentication status=%d, want 401", response.Code)
	}
	devRequest := httptest.NewRequest(http.MethodGet, "/v1/mailboxes", nil)
	devRequest.Header.Set("X-Dev-Subject", "must-not-read-database-mailboxes")
	devResponse := httptest.NewRecorder()
	handler.ServeHTTP(devResponse, devRequest)
	if devResponse.Code != http.StatusUnauthorized {
		t.Fatalf("development identity mailbox status=%d, want 401", devResponse.Code)
	}
	if !strings.Contains(devResponse.Body.String(), "user-bound API key") {
		t.Fatalf("development identity mailbox response lacks setup guidance: %s", devResponse.Body.String())
	}

	createKeyRequest := httptest.NewRequest(
		http.MethodPost,
		"/v1/api-keys",
		bytes.NewBufferString(`{"name":"route-issued","scopes":["mailbox:read"]}`),
	)
	createKeyRequest.Header.Set("Authorization", "Bearer "+adminSecret)
	createKeyRequest.Header.Set("Content-Type", "application/json")
	createKeyResponse := httptest.NewRecorder()
	handler.ServeHTTP(createKeyResponse, createKeyRequest)
	if createKeyResponse.Code != http.StatusCreated {
		t.Fatalf("create API key status=%d body=%s", createKeyResponse.Code, createKeyResponse.Body.String())
	}
	var issued struct {
		Secret string `json:"secret"`
	}
	if err := json.Unmarshal(createKeyResponse.Body.Bytes(), &issued); err != nil {
		t.Fatal(err)
	}
	if issued.Secret == "" {
		t.Fatal("API key endpoint omitted the one-time secret")
	}
	issuedPrincipal, err := repository.AuthenticateAPIKey(ctx, issued.Secret)
	if err != nil {
		t.Fatalf("authenticate API key issued by route: %v", err)
	}
	if issuedPrincipal.UserID != ownerA {
		t.Fatalf("API key issued by route user_id=%q, want %q", issuedPrincipal.UserID, ownerA)
	}

	principal := Principal{
		Subject:  "api-key:fixture",
		TenantID: tenantA,
		UserID:   ownerA,
		Scopes:   map[string]bool{"keys:write": true, "mailbox:read": true},
		durable:  true,
	}
	created, createdSecret, err := repository.CreateAPIKey(
		ctx, principal, "integration-issued", []string{"mailbox:read"}, nil,
	)
	if err != nil {
		t.Fatalf("create durable API key: %v", err)
	}
	if created.Prefix != createdSecret[:apiKeyPrefixLength] {
		t.Fatal("created API key prefix does not match returned key")
	}
	if _, err := repository.AuthenticateAPIKey(ctx, createdSecret); err != nil {
		t.Fatalf("authenticate created API key: %v", err)
	}
	if err := repository.RevokeAPIKey(ctx, principal, created.ID); err != nil {
		t.Fatalf("revoke durable API key: %v", err)
	}
	if _, err := repository.AuthenticateAPIKey(ctx, createdSecret); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("revoked key authentication error=%v, want unauthorized", err)
	}
	if _, err := pool.Exec(ctx, `UPDATE users SET status = 'disabled' WHERE id = $1`, ownerA); err != nil {
		t.Fatalf("disable OIDC test user: %v", err)
	}
	if _, err := repository.AuthenticateOIDCSubject(ctx, keycloakIssuer, "mailbox-api-owner-a", tenantA); !errors.Is(err, ErrUnauthorized) {
		t.Fatalf("disabled OIDC user authentication error=%v, want unauthorized", err)
	}

}

type fixedOIDCVerifier struct {
	claims identity.Claims
}

func (v fixedOIDCVerifier) Verify(context.Context, string) (identity.Claims, error) {
	return v.claims, nil
}

func insertTestTenant(t *testing.T, ctx context.Context, pool *pgxpool.Pool, name string) string {
	t.Helper()
	var id string
	if err := pool.QueryRow(ctx, `
		INSERT INTO tenants (id, name) VALUES (gen_random_uuid(), $1)
		RETURNING id::text
	`, name).Scan(&id); err != nil {
		t.Fatalf("insert test tenant: %v", err)
	}
	return id
}

func insertTestUser(t *testing.T, ctx context.Context, pool *pgxpool.Pool, tenantID, subject string) string {
	t.Helper()
	var userID string
	if err := pool.QueryRow(ctx, `
		INSERT INTO users (id, external_subject) VALUES (gen_random_uuid(), $1)
		RETURNING id::text
	`, subject).Scan(&userID); err != nil {
		t.Fatalf("insert test user: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		INSERT INTO tenant_memberships (tenant_id, user_id, role)
		VALUES ($1, $2, 'owner')
	`, tenantID, userID); err != nil {
		t.Fatalf("insert test membership: %v", err)
	}
	return userID
}

func insertTestMailboxMessage(
	t *testing.T,
	ctx context.Context,
	pool *pgxpool.Pool,
	tenantID string,
) (string, string) {
	t.Helper()
	var domainID string
	if err := pool.QueryRow(ctx, `
		INSERT INTO domains (id, tenant_id, domain, status)
		VALUES (gen_random_uuid(), $1, gen_random_uuid()::text || '.integration.test', 'verified')
		RETURNING id::text
	`, tenantID).Scan(&domainID); err != nil {
		t.Fatalf("insert mailbox test domain: %v", err)
	}
	var mailboxID string
	if err := pool.QueryRow(ctx, `
		INSERT INTO mailboxes
		  (id, tenant_id, domain_id, local_part, mailbox_kind, display_name,
		   owner_user_id, quota_bytes, used_bytes, status)
		VALUES (gen_random_uuid(), $1, $2, 'inbox', 'shared', 'Integration mailbox',
		        NULL, 1048576, 0, 'active')
		RETURNING id::text
	`, tenantID, domainID).Scan(&mailboxID); err != nil {
		t.Fatalf("insert mailbox test mailbox: %v", err)
	}
	var folderID string
	if err := pool.QueryRow(ctx, `
		INSERT INTO mailbox_folders (id, mailbox_id, name, uid_validity)
		VALUES (gen_random_uuid(), $1, 'INBOX', 1)
		RETURNING id::text
	`, mailboxID).Scan(&folderID); err != nil {
		t.Fatalf("insert mailbox test folder: %v", err)
	}
	var messageID string
	if err := pool.QueryRow(ctx, `
		INSERT INTO mailbox_messages
		  (id, mailbox_id, folder_id, uid, raw_message, raw_size,
		   sender_address, subject, internal_date)
		VALUES (gen_random_uuid(), $1, $2, 1, $3, $4,
		        'sender@example.test', 'PostgreSQL integration message', NOW())
		RETURNING id::text
	`, mailboxID, folderID, []byte("Subject: PostgreSQL integration message\r\n\r\nbody"), 48).Scan(&messageID); err != nil {
		t.Fatalf("insert mailbox test message: %v", err)
	}
	return mailboxID, messageID
}

func grantMailboxRead(
	t *testing.T,
	ctx context.Context,
	pool *pgxpool.Pool,
	tenantID string,
	mailboxID string,
	userID string,
) {
	t.Helper()
	if _, err := pool.Exec(ctx, `
		INSERT INTO mailbox_permissions (tenant_id, mailbox_id, user_id, permission)
		VALUES ($1, $2, $3, 'read')
	`, tenantID, mailboxID, userID); err != nil {
		t.Fatalf("grant mailbox read: %v", err)
	}
}

func insertTestAPIKey(
	t *testing.T,
	ctx context.Context,
	pool *pgxpool.Pool,
	tenantID string,
	userID string,
	scopes []string,
) string {
	t.Helper()
	raw := make([]byte, 32)
	if _, err := rand.Read(raw); err != nil {
		t.Fatal("generate test API key")
	}
	secret := "sp_" + base64.RawURLEncoding.EncodeToString(raw)
	hash := sha256.Sum256([]byte(secret))
	name := "integration key " + secret[:apiKeyPrefixLength]
	if _, err := pool.Exec(ctx, `
		INSERT INTO api_keys
		  (id, tenant_id, created_by_user_id, principal_type, name, prefix, secret_hash, scopes)
		VALUES (gen_random_uuid(), $1, $2, 'user', $3, $4, $5, $6)
	`, tenantID, userID, name, secret[:apiKeyPrefixLength], hash[:], scopes); err != nil {
		t.Fatalf("insert test API key: %v", err)
	}
	return secret
}

func insertLegacyTenantAPIKey(
	t *testing.T,
	ctx context.Context,
	pool *pgxpool.Pool,
	tenantID string,
	scopes []string,
) string {
	t.Helper()
	raw := make([]byte, 32)
	if _, err := rand.Read(raw); err != nil {
		t.Fatal("generate legacy test API key")
	}
	secret := "sp_" + base64.RawURLEncoding.EncodeToString(raw)
	hash := sha256.Sum256([]byte(secret))
	if _, err := pool.Exec(ctx, `
		INSERT INTO api_keys (id, tenant_id, name, prefix, secret_hash, scopes)
		VALUES (gen_random_uuid(), $1, 'legacy tenant key', $2, $3, $4)
	`, tenantID, secret[:apiKeyPrefixLength], hash[:], scopes); err != nil {
		t.Fatalf("insert legacy tenant API key: %v", err)
	}
	return secret
}

func cleanupTestIdentity(
	t *testing.T,
	ctx context.Context,
	pool *pgxpool.Pool,
	tenantIDs []string,
	userIDs []string,
) {
	t.Helper()
	if _, err := pool.Exec(ctx, `
		DELETE FROM api_keys WHERE tenant_id::text = ANY($1::text[])
	`, tenantIDs); err != nil {
		t.Errorf("remove test API keys: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		DELETE FROM mailboxes WHERE tenant_id::text = ANY($1::text[])
	`, tenantIDs); err != nil {
		t.Errorf("remove test mailboxes: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		DELETE FROM domain_verification_challenges
		WHERE domain_id IN (
			SELECT id FROM domains WHERE tenant_id::text = ANY($1::text[])
		)
	`, tenantIDs); err != nil {
		t.Errorf("remove test domain verification challenges: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		DELETE FROM domains WHERE tenant_id::text = ANY($1::text[])
	`, tenantIDs); err != nil {
		t.Errorf("remove test domains: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		DELETE FROM tenant_memberships WHERE tenant_id::text = ANY($1::text[])
	`, tenantIDs); err != nil {
		t.Errorf("remove test memberships: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		DELETE FROM users WHERE id::text = ANY($1::text[])
	`, userIDs); err != nil {
		t.Errorf("remove test users: %v", err)
	}
	if _, err := pool.Exec(ctx, `
		DELETE FROM tenants WHERE id::text = ANY($1::text[])
	`, tenantIDs); err != nil {
		t.Errorf("remove test tenants: %v", err)
	}
}

func assertJSONItems(t *testing.T, body []byte, expected int) {
	t.Helper()
	var response struct {
		Items []json.RawMessage `json:"items"`
	}
	if err := json.Unmarshal(body, &response); err != nil {
		t.Fatalf("decode API response: %v", err)
	}
	if len(response.Items) != expected {
		t.Fatalf("item count=%d, want %d", len(response.Items), expected)
	}
}
