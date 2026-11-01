package controlplane

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"net/http"
	"strings"
	"time"
	"unicode/utf8"

	"example.com/smtp-platform/smtp-api/internal/authmail"
	"example.com/smtp-platform/smtp-api/internal/identity"
	"example.com/smtp-platform/smtp-api/internal/observability"
	smtpAuth "example.com/smtp-platform/smtp-auth"
)

type OIDCVerifier interface {
	Verify(context.Context, string) (identity.Claims, error)
}

type Handler struct {
	Store             *Store
	Postgres          *PostgreSQLRepository
	MailboxProxy      *MailboxProxy
	OIDC              OIDCVerifier
	OIDCLogin         *smtpAuth.OIDCClient
	AuthMailer        authmail.Sender
	DummyPasswordHash string
	AuthSessions      BrowserSessionConfig
	SocialOAuth       SocialOAuthConfig
	JWT               JWTConfig
	RateLimitHMACKey  []byte
	AuthEmailKey      []byte
	DevMode           bool
	TXT               TXTResolver
	DNS               DNSResolver
	Metrics           *observability.Metrics
}

func (h Handler) Routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /v1/auth/providers", h.authProviders)
	mux.HandleFunc("POST /v1/auth/signup", h.signUp)
	mux.HandleFunc("POST /v1/auth/login", h.passwordLogin)
	mux.HandleFunc("POST /v1/auth/refresh", h.refreshAuthSession)
	mux.HandleFunc("POST /v1/auth/change-password", h.changePassword)
	mux.HandleFunc("POST /v1/auth/email/verify", h.verifyEmail)
	mux.HandleFunc("POST /v1/auth/email/resend-verification", h.resendVerification)
	mux.HandleFunc("POST /v1/auth/password/forgot", h.forgotPassword)
	mux.HandleFunc("POST /v1/auth/password/reset", h.resetPassword)
	mux.HandleFunc("POST /v1/auth/sessions", h.createAuthSession)
	mux.HandleFunc("GET /v1/auth/sessions", h.authSessions)
	mux.HandleFunc("DELETE /v1/auth/sessions/{id}", h.revokeAuthSession)
	mux.HandleFunc("POST /v1/auth/logout", h.logout)
	mux.HandleFunc("POST /v1/auth/logout-all", h.logoutAll)
	mux.HandleFunc("GET /v1/auth/oidc/start", h.startOIDCLogin)
	mux.HandleFunc("GET /v1/auth/oidc/callback", h.completeOIDCLogin)
	mux.HandleFunc("GET /v1/auth/oauth/google/start", h.startGoogleOAuth)
	mux.HandleFunc("GET /v1/auth/oauth/google/callback", h.completeGoogleOAuth)
	mux.HandleFunc("GET /v1/auth/oauth/github/start", h.startGitHubOAuth)
	mux.HandleFunc("GET /v1/auth/oauth/github/callback", h.completeGitHubOAuth)
	mux.HandleFunc("GET /v1/me", h.me)
	mux.HandleFunc("GET /v1/tenants/current", h.tenant)
	mux.HandleFunc("GET /v1/domains", h.domains)
	mux.HandleFunc("POST /v1/domains", h.createDomain)
	mux.HandleFunc("POST /v1/domains/{id}/verification", h.challenge)
	mux.HandleFunc("POST /v1/domains/{id}/verification/check", h.check)
	mux.HandleFunc("POST /v1/domains/{id}/checks", h.checksRun)
	mux.HandleFunc("GET /v1/domains/{id}/checks", h.checksGet)
	mux.HandleFunc("GET /v1/domains/{id}/readiness", h.readiness)
	mux.HandleFunc("GET /v1/usage", h.usage)
	mux.HandleFunc("GET /v1/sending/config", h.sendingConfig)
	mux.HandleFunc("POST /v1/submissions", h.submit)
	mux.HandleFunc("GET /v1/submissions", h.submissions)
	mux.HandleFunc("GET /v1/submissions/{id}/attempts", h.submissionAttempts)
	mux.HandleFunc("GET /v1/delivery-events", h.deliveryEvents)
	mux.HandleFunc("POST /v1/bounces", h.recordBounce)
	mux.HandleFunc("POST /v1/complaints", h.recordComplaint)
	mux.HandleFunc("GET /v1/webhooks", h.webhooks)
	mux.HandleFunc("POST /v1/webhooks", h.createWebhook)
	mux.HandleFunc("DELETE /v1/webhooks/{id}", h.deleteWebhook)
	mux.HandleFunc("GET /v1/webhook-dead-letters", h.webhookDeadLetters)
	mux.HandleFunc("POST /v1/webhook-deliveries/{id}/replay", h.replayWebhook)
	mux.HandleFunc("GET /v1/suppressions", h.suppressions)
	mux.HandleFunc("POST /v1/suppressions", h.createSuppression)
	mux.HandleFunc("DELETE /v1/suppressions/{recipient}", h.deleteSuppression)
	mux.HandleFunc("DELETE /v1/domains/{id}", h.disableDomain)
	mux.HandleFunc("GET /v1/api-keys", h.keys)
	mux.HandleFunc("POST /v1/api-keys", h.createKey)
	mux.HandleFunc("POST /v1/api-keys/{id}/revoke", h.revokeKey)
	mux.HandleFunc("GET /v1/smtp-credentials", h.smtpCredentials)
	mux.HandleFunc("POST /v1/smtp-credentials", h.createSMTPCredential)
	mux.HandleFunc("POST /v1/smtp-credentials/{id}/rotate", h.rotateSMTPCredential)
	mux.HandleFunc("POST /v1/smtp-credentials/{id}/revoke", h.revokeSMTPCredential)
	mux.HandleFunc("GET /v1/mailboxes", h.mailboxes)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/messages", h.messages)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/messages/{message_id}", h.message)
	mux.HandleFunc("POST /v1/mailboxes", h.mailboxServiceManage)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}", h.mailboxServiceRead)
	mux.HandleFunc("PATCH /v1/mailboxes/{mailbox_id}", h.mailboxServiceManage)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/aliases", h.mailboxServiceManage)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/folders", h.mailboxServiceRead)
	mux.HandleFunc("POST /v1/mailboxes/{mailbox_id}/folders", h.mailboxServiceWrite)
	mux.HandleFunc("PATCH /v1/mailboxes/{mailbox_id}/folders/{folder_id}", h.mailboxServiceWrite)
	mux.HandleFunc("DELETE /v1/mailboxes/{mailbox_id}/folders/{folder_id}", h.mailboxServiceWrite)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/messages/{message_id}/content", h.mailboxServiceRead)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/messages/{message_id}/headers", h.mailboxServiceRead)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/messages/{message_id}/body", h.mailboxServiceRead)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/export", h.mailboxServiceRead)
	mux.HandleFunc("POST /v1/mailboxes/{mailbox_id}/messages", h.mailboxServiceWrite)
	mux.HandleFunc("POST /v1/mailboxes/{mailbox_id}/messages/{message_id}/copy", h.mailboxServiceWrite)
	mux.HandleFunc("POST /v1/mailboxes/{mailbox_id}/messages/{message_id}/move", h.mailboxServiceWrite)
	mux.HandleFunc("PATCH /v1/mailboxes/{mailbox_id}/messages/{message_id}/flags", h.mailboxServiceWrite)
	mux.HandleFunc("PUT /v1/mailboxes/{mailbox_id}/messages/{message_id}/retention", h.mailboxServiceManage)
	mux.HandleFunc("DELETE /v1/mailboxes/{mailbox_id}/messages/{message_id}", h.mailboxServiceWrite)
	mux.HandleFunc("POST /v1/mailboxes/{mailbox_id}/folders/{folder_id}/expunge", h.mailboxServiceWrite)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/labels", h.mailboxServiceRead)
	mux.HandleFunc("POST /v1/mailboxes/{mailbox_id}/labels", h.mailboxServiceManage)
	mux.HandleFunc("PUT /v1/mailboxes/{mailbox_id}/messages/{message_id}/labels/{label_id}", h.mailboxServiceWrite)
	mux.HandleFunc("DELETE /v1/mailboxes/{mailbox_id}/messages/{message_id}/labels/{label_id}", h.mailboxServiceWrite)
	mux.HandleFunc("POST /v1/mailboxes/{mailbox_id}/aliases", h.mailboxServiceManage)
	mux.HandleFunc("DELETE /v1/mailboxes/{mailbox_id}/aliases/{alias_id}", h.mailboxServiceManage)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/permissions", h.mailboxServiceManage)
	mux.HandleFunc("PUT /v1/mailboxes/{mailbox_id}/permissions/{user_id}", h.mailboxServiceManage)
	mux.HandleFunc("DELETE /v1/mailboxes/{mailbox_id}/permissions/{user_id}", h.mailboxServiceManage)
	mux.HandleFunc("GET /v1/mailboxes/{mailbox_id}/quota", h.mailboxServiceRead)
	mux.HandleFunc("PUT /v1/mailboxes/{mailbox_id}/quota", h.mailboxServiceManage)
	if h.Metrics == nil {
		h.Metrics = observability.NewMetrics()
	}
	return requestContext(h.Metrics.Middleware(mux))
}

const requestIDHeader = "X-Request-ID"

func requestContext(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestID := strings.TrimSpace(r.Header.Get(requestIDHeader))
		if !validRequestID(requestID) {
			requestID = newRequestID()
		}
		w.Header().Set(requestIDHeader, requestID)
		next.ServeHTTP(w, r)
	})
}

func validRequestID(value string) bool {
	if value == "" || len(value) > 64 || !utf8.ValidString(value) {
		return false
	}
	for _, r := range value {
		if !(r == '-' || r == '_' || r == '.' ||
			r >= '0' && r <= '9' || r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z') {
			return false
		}
	}
	return true
}

func newRequestID() string {
	var raw [16]byte
	if _, err := rand.Read(raw[:]); err != nil {
		return fmt.Sprintf("request-%d", time.Now().UnixNano())
	}
	return hex.EncodeToString(raw[:])
}
