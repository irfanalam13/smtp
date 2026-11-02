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

func TestMFAAndRefreshTokenLifecycleIntegration(t *testing.T) {
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

	tenantID := insertTestTenant(t, ctx, pool, "mfa test tenant")
	userID := insertTestUser(t, ctx, pool, tenantID, "mfa-test-user")
	t.Cleanup(func() {
		cleanupCtx, c := context.WithTimeout(context.Background(), 10*time.Second)
		defer c()
		_, _ = pool.Exec(cleanupCtx, "DELETE FROM auth_refresh_tokens WHERE tenant_id = $1", tenantID)
		_, _ = pool.Exec(cleanupCtx, "DELETE FROM auth_mfa_challenges WHERE tenant_id = $1", tenantID)
		_, _ = pool.Exec(cleanupCtx, "DELETE FROM auth_mfa_recovery_codes WHERE user_id = $1", userID)
		cleanupTestIdentity(t, cleanupCtx, pool, []string{tenantID}, []string{userID})
	})

	repository := NewPostgreSQLRepository(pool)
	jwtConfig := JWTConfig{
		SigningKey: []byte("0123456789abcdef0123456789abcdef"),
		KeyID:      "test-key-1",
		Issuer:     "https://app.example.test",
		TTL:        time.Hour,
	}
	authSessions := BrowserSessionConfig{
		Enabled:       true,
		PublicOrigin:  "https://app.example.test",
		SessionCookie: "sid",
		CSRFCookie:    "csrf",
		IdleTTL:       time.Hour,
		AbsoluteTTL:   24 * time.Hour,
	}
	handler := Handler{
		Postgres:         repository,
		JWT:              jwtConfig,
		AuthSessions:     authSessions,
		AuthEmailKey:     []byte("test-encryption-key-at-least-16"),
		RateLimitHMACKey: bytes.Repeat([]byte("k"), 32),
	}.Routes()

	principal := Principal{
		UserID:   userID,
		TenantID: tenantID,
		Role:     Owner,
		Subject:  "mfa-test-user@example.test",
		Scopes:   smtpAuth.PermissionsForRole("owner"),
		durable:  true,
	}

	// Step 1: Create a refresh token and rotate it
	rawRefresh, familyID, expiresAt, err := repository.CreateRefreshToken(ctx, principal, nil, nil, time.Hour)
	if err != nil || rawRefresh == "" || familyID == "" || expiresAt.Before(time.Now()) {
		t.Fatalf("CreateRefreshToken failed: %v", err)
	}

	// Rotate the refresh token via HTTP endpoint
	rotateBody, _ := json.Marshal(map[string]string{"refresh_token": rawRefresh})
	req := httptest.NewRequest(http.MethodPost, "/v1/auth/tokens/refresh", bytes.NewReader(rotateBody))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("rotate refresh token status = %d, body = %s", rec.Code, rec.Body.String())
	}
	var rotateResp struct {
		AccessToken  string `json:"access_token"`
		RefreshToken string `json:"refresh_token"`
	}
	if err := json.Unmarshal(rec.Body.Bytes(), &rotateResp); err != nil || rotateResp.RefreshToken == "" || rotateResp.AccessToken == "" {
		t.Fatalf("invalid rotate response: %s", rec.Body.String())
	}

	// Verify the access token issued during rotation works
	verifiedPrincipal, err := jwtConfig.Verify(rotateResp.AccessToken)
	if err != nil || verifiedPrincipal.UserID != userID || verifiedPrincipal.TenantID != tenantID {
		t.Fatalf("Verify rotated JWT failed: %v", err)
	}

	// Step 2: Replay detection: using the OLD refresh token must fail and invalidate the family!
	replayReq := httptest.NewRequest(http.MethodPost, "/v1/auth/tokens/refresh", bytes.NewReader(rotateBody))
	replayReq.Header.Set("Content-Type", "application/json")
	replayRec := httptest.NewRecorder()
	handler.ServeHTTP(replayRec, replayReq)

	if replayRec.Code != http.StatusUnauthorized {
		t.Fatalf("replay of old refresh token did not fail with 401, got %d", replayRec.Code)
	}

	// Now, because of replay detection, the NEW refresh token in the same family must ALSO be rejected!
	newRotateBody, _ := json.Marshal(map[string]string{"refresh_token": rotateResp.RefreshToken})
	familyReq := httptest.NewRequest(http.MethodPost, "/v1/auth/tokens/refresh", bytes.NewReader(newRotateBody))
	familyReq.Header.Set("Content-Type", "application/json")
	familyRec := httptest.NewRecorder()
	handler.ServeHTTP(familyRec, familyReq)

	if familyRec.Code != http.StatusUnauthorized {
		t.Fatalf("family after replay was not invalidated, got %d", familyRec.Code)
	}

	// Step 3: MFA TOTP Setup and Confirmation
	// Setup TOTP
	_, rawSecret, err := smtpAuth.NewTOTPSecret()
	if err != nil {
		t.Fatal(err)
	}
	encryptedSecret, err := smtpAuth.EncryptSecret([]byte("test-encryption-key-at-least-16"), rawSecret)
	if err != nil {
		t.Fatal(err)
	}
	if err := repository.SetupTOTP(ctx, userID, encryptedSecret); err != nil {
		t.Fatalf("SetupTOTP failed: %v", err)
	}

	// Generate code from the secret
	code := smtpAuth.TOTPCode(rawSecret, time.Now())
	recoveryCodes, recoveryHashes, err := smtpAuth.NewRecoveryCodes()
	if err != nil {
		t.Fatal(err)
	}
	if err := repository.ConfirmTOTP(ctx, userID, recoveryHashes); err != nil {
		t.Fatalf("ConfirmTOTP failed: %v", err)
	}

	// Verify TOTP is confirmed
	_, confirmed, err := repository.GetEncryptedTOTPSecret(ctx, userID)
	if err != nil || !confirmed {
		t.Fatalf("GetEncryptedTOTPSecret: confirmed = %v, err = %v", confirmed, err)
	}

	// Step 4: MFA Challenge and Login Verification with TOTP code
	challengeToken, err := repository.CreateMFAChallenge(ctx, userID, tenantID, "login", 5*time.Minute)
	if err != nil || challengeToken == "" {
		t.Fatalf("CreateMFAChallenge failed: %v", err)
	}

	mfaVerifyBody, _ := json.Marshal(map[string]string{
		"challenge_token": challengeToken,
		"code":            code,
	})
	mfaReq := httptest.NewRequest(http.MethodPost, "/v1/auth/mfa/challenge/verify", bytes.NewReader(mfaVerifyBody))
	mfaReq.Header.Set("Content-Type", "application/json")
	mfaRec := httptest.NewRecorder()
	handler.ServeHTTP(mfaRec, mfaReq)

	if mfaRec.Code != http.StatusOK {
		t.Fatalf("MFA verify with code failed, status=%d body=%s", mfaRec.Code, mfaRec.Body.String())
	}

	// Verify reusing the challenge token fails (single-use)
	mfaReplayReq := httptest.NewRequest(http.MethodPost, "/v1/auth/mfa/challenge/verify", bytes.NewReader(mfaVerifyBody))
	mfaReplayReq.Header.Set("Content-Type", "application/json")
	mfaReplayRec := httptest.NewRecorder()
	handler.ServeHTTP(mfaReplayRec, mfaReplayReq)
	if mfaReplayRec.Code != http.StatusUnauthorized {
		t.Fatalf("MFA challenge replay expected 401, got %d", mfaReplayRec.Code)
	}

	// Step 5: MFA Challenge verification with Recovery Code
	challengeToken2, err := repository.CreateMFAChallenge(ctx, userID, tenantID, "login", 5*time.Minute)
	if err != nil {
		t.Fatal(err)
	}
	recoveryVerifyBody, _ := json.Marshal(map[string]string{
		"challenge_token": challengeToken2,
		"recovery_code":   recoveryCodes[0],
	})
	recovReq := httptest.NewRequest(http.MethodPost, "/v1/auth/mfa/challenge/verify", bytes.NewReader(recoveryVerifyBody))
	recovReq.Header.Set("Content-Type", "application/json")
	recovRec := httptest.NewRecorder()
	handler.ServeHTTP(recovRec, recovReq)

	if recovRec.Code != http.StatusOK {
		t.Fatalf("MFA verify with recovery code failed, status=%d body=%s", recovRec.Code, recovRec.Body.String())
	}

	// Verify using the SAME recovery code a second time fails (one-time use)
	challengeToken3, err := repository.CreateMFAChallenge(ctx, userID, tenantID, "login", 5*time.Minute)
	if err != nil {
		t.Fatal(err)
	}
	recovReplayBody, _ := json.Marshal(map[string]string{
		"challenge_token": challengeToken3,
		"recovery_code":   recoveryCodes[0],
	})
	recovReplayReq := httptest.NewRequest(http.MethodPost, "/v1/auth/mfa/challenge/verify", bytes.NewReader(recovReplayBody))
	recovReplayReq.Header.Set("Content-Type", "application/json")
	recovReplayRec := httptest.NewRecorder()
	handler.ServeHTTP(recovReplayRec, recovReplayReq)

	if recovReplayRec.Code != http.StatusUnauthorized {
		t.Fatalf("used recovery code must be rejected with 401, got %d", recovReplayRec.Code)
	}
}
