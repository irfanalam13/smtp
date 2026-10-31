package controlplane

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"net/mail"
	"net/url"
	"strings"
	"time"
	"unicode"
	"unicode/utf8"

	"example.com/smtp-platform/smtp-api/internal/authmail"
	smtpAuth "example.com/smtp-platform/smtp-auth"
)

const (
	emailVerificationTokenLifetime = 24 * time.Hour
	passwordResetTokenLifetime     = 30 * time.Minute
)

type signUpRequest struct {
	Email            string `json:"email"`
	Password         string `json:"password"`
	OrganizationName string `json:"organization_name"`
}

type passwordLoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
	TenantID string `json:"tenant_id"`
}

type emailTokenRequest struct {
	Token string `json:"token"`
}

type emailRequest struct {
	Email string `json:"email"`
}

type resetPasswordRequest struct {
	Token       string `json:"token"`
	NewPassword string `json:"new_password"`
}

func (h Handler) signUp(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil || h.AuthMailer == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "account registration is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
		return
	}
	if !h.Store.Allow("auth-signup:"+requestClientIP(r), 3, time.Hour) {
		w.Header().Set("Retry-After", "3600")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many account registration attempts")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 8192)
	var request signUpRequest
	if err := decodeJSON(r, &request); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "request body is invalid")
		return
	}
	email, ok := normalizeEmail(request.Email)
	if !ok || !validPassword(request.Password) ||
		!validOrganizationName(request.OrganizationName) {
		Error(w, http.StatusBadRequest, "invalid_request", "email, password, or organization name is invalid")
		return
	}
	passwordHash, err := smtpAuth.HashPasswordBounded(request.Password)
	if errors.Is(err, smtpAuth.ErrPasswordCapacity) {
		w.Header().Set("Retry-After", "10")
		Error(w, http.StatusTooManyRequests, "rate_limited", "password service is temporarily busy")
		return
	}
	if err != nil {
		h.databaseError(w, "hash account password", err)
		return
	}
	token, tokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		h.databaseError(w, "create email verification token", err)
		return
	}
	registered, err := h.Postgres.RegisterPasswordAccount(
		r.Context(), email, passwordHash, strings.TrimSpace(request.OrganizationName),
		tokenHash, time.Now().Add(emailVerificationTokenLifetime),
	)
	if errors.Is(err, ErrConflict) {
		JSON(w, http.StatusAccepted, map[string]string{
			"status":  "verification_pending",
			"message": "If registration can proceed, a verification message will be sent to this address.",
		})
		return
	}
	if err != nil {
		h.databaseError(w, "register password account", err)
		return
	}
	verifyURL := h.AuthSessions.PublicOrigin + "/verify-email?token=" + url.QueryEscape(token)
	if err := h.sendAccountEmail(r, email, "Verify your SMTP Platform account",
		fmt.Sprintf("Verify your email address to activate the account for %s:\n\n%s\n\nThis link expires in 24 hours.", strings.TrimSpace(request.OrganizationName), verifyURL)); err != nil {
		slog.Error("account verification email delivery failed", "operation", "account.signup")
		Error(w, http.StatusServiceUnavailable, "email_unavailable", "account created, but the verification message could not be sent; use resend verification after email delivery is restored")
		return
	}
	JSON(w, http.StatusAccepted, map[string]string{
		"status":    "verification_pending",
		"message":   "Check your email for a verification link before signing in.",
		"tenant_id": registered.TenantID,
	})
}

func (h Handler) passwordLogin(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "password sign-in is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 8192)
	var request passwordLoginRequest
	if err := decodeJSON(r, &request); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "request body is invalid")
		return
	}
	email, ok := normalizeEmail(request.Email)
	if !ok || request.Password == "" || len(request.Password) > 1024 ||
		(request.TenantID != "" && !validUUID(request.TenantID)) {
		Error(w, http.StatusBadRequest, "invalid_request", "email, password, or tenant selection is invalid")
		return
	}
	emailHash := smtpAuth.HashSecret(email)
	if !h.Store.Allow("auth-login:"+requestClientIP(r)+":"+string(emailHash[:8]), 10, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many sign-in attempts")
		return
	}
	account, err := h.Postgres.PasswordAccount(r.Context(), email)
	if err != nil {
		if errors.Is(err, ErrUnauthorized) {
			if h.DummyPasswordHash != "" {
				if _, verifyErr := smtpAuth.VerifyPasswordBounded(h.DummyPasswordHash, request.Password); errors.Is(verifyErr, smtpAuth.ErrPasswordCapacity) {
					w.Header().Set("Retry-After", "10")
					Error(w, http.StatusTooManyRequests, "rate_limited", "password service is temporarily busy")
					return
				} else if verifyErr != nil {
					h.databaseError(w, "verify account password", verifyErr)
					return
				}
			}
			Error(w, http.StatusUnauthorized, "unauthorized", "email or password is incorrect")
			return
		}
		h.databaseError(w, "load password account", err)
		return
	}
	passwordValid, err := smtpAuth.VerifyPasswordBounded(account.PasswordHash, request.Password)
	if errors.Is(err, smtpAuth.ErrPasswordCapacity) {
		w.Header().Set("Retry-After", "10")
		Error(w, http.StatusTooManyRequests, "rate_limited", "password service is temporarily busy")
		return
	}
	if err != nil {
		h.databaseError(w, "verify account password", err)
		return
	}
	if !passwordValid {
		Error(w, http.StatusUnauthorized, "unauthorized", "email or password is incorrect")
		return
	}
	if !account.Verified {
		Error(w, http.StatusForbidden, "email_not_verified", "verify your email address before signing in")
		return
	}
	principal, err := selectPasswordMembership(account.Memberships, request.TenantID)
	if err != nil {
		if errors.Is(err, ErrTenantSelectionRequired) {
			Error(w, http.StatusBadRequest, "tenant_selection_required", "specify a tenant_id for this account")
			return
		}
		Error(w, http.StatusUnauthorized, "unauthorized", "email or password is incorrect")
		return
	}
	session, err := h.issueBrowserSession(w, r, principal)
	if err != nil {
		if errors.Is(err, ErrUnauthorized) {
			Error(w, http.StatusUnauthorized, "unauthorized", "email or password is incorrect")
			return
		}
		h.databaseError(w, "create password browser session", err)
		return
	}
	JSON(w, http.StatusOK, map[string]any{
		"status": "authenticated", "user_id": principal.UserID,
		"tenant_id": principal.TenantID, "expires_at": session.ExpiresAt,
	})
}

func (h Handler) verifyEmail(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "account storage is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 2048)
	var request emailTokenRequest
	if err := decodeJSON(r, &request); err != nil || !validOpaqueToken(request.Token) {
		Error(w, http.StatusBadRequest, "invalid_request", "verification token is invalid or expired")
		return
	}
	consumed, err := h.Postgres.ConsumePasswordEmailToken(
		r.Context(), smtpAuth.HashSecret(request.Token), "verify_email", "",
	)
	if err != nil {
		h.databaseError(w, "verify account email", err)
		return
	}
	if !consumed {
		Error(w, http.StatusBadRequest, "invalid_request", "verification token is invalid or expired")
		return
	}
	JSON(w, http.StatusOK, map[string]string{"status": "verified"})
}

func (h Handler) resendVerification(w http.ResponseWriter, r *http.Request) {
	h.sendAccountLink(w, r, "verify_email")
}

func (h Handler) forgotPassword(w http.ResponseWriter, r *http.Request) {
	h.sendAccountLink(w, r, "reset_password")
}

func (h Handler) sendAccountLink(w http.ResponseWriter, r *http.Request, purpose string) {
	if !h.AuthSessions.valid() || h.Postgres == nil || h.AuthMailer == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "account email delivery is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
		return
	}
	if !h.Store.Allow("auth-email:"+purpose+":"+requestClientIP(r), 5, time.Hour) {
		w.Header().Set("Retry-After", "3600")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many email requests")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 2048)
	var request emailRequest
	if err := decodeJSON(r, &request); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "request body is invalid")
		return
	}
	email, ok := normalizeEmail(request.Email)
	if !ok {
		Error(w, http.StatusBadRequest, "invalid_request", "email address is invalid")
		return
	}
	response := map[string]string{
		"status":  "accepted",
		"message": "If an eligible account exists, an email with the requested link will be sent.",
	}
	limit := emailVerificationTokenLifetime
	subject := "Verify your SMTP Platform account"
	path := "/verify-email?token="
	bodyText := "Verify your email address using this link. It expires in 24 hours:\n\n"
	if purpose == "reset_password" {
		limit = passwordResetTokenLifetime
		subject = "Reset your SMTP Platform password"
		path = "/reset-password?token="
		bodyText = "Reset your password using this link. It expires in 30 minutes:\n\n"
	}
	token, tokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		h.databaseError(w, "create account email token", err)
		return
	}
	created, err := h.Postgres.CreatePasswordEmailToken(
		r.Context(), email, purpose, tokenHash, time.Now().Add(limit),
	)
	if err != nil {
		h.databaseError(w, "create account email token", err)
		return
	}
	if created {
		link := h.AuthSessions.PublicOrigin + path + url.QueryEscape(token)
		if err := h.sendAccountEmail(r, email, subject, bodyText+link); err != nil {
			slog.Error("account email delivery failed", "operation", "account."+purpose)
		}
	}
	JSON(w, http.StatusAccepted, response)
}

func (h Handler) resetPassword(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "password recovery is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
		return
	}
	if !h.Store.Allow("auth-password-reset:"+requestClientIP(r), 5, time.Hour) {
		w.Header().Set("Retry-After", "3600")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many password recovery attempts")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var request resetPasswordRequest
	if err := decodeJSON(r, &request); err != nil ||
		!validOpaqueToken(request.Token) || !validPassword(request.NewPassword) {
		Error(w, http.StatusBadRequest, "invalid_request", "token or new password is invalid")
		return
	}
	passwordHash, err := smtpAuth.HashPasswordBounded(request.NewPassword)
	if errors.Is(err, smtpAuth.ErrPasswordCapacity) {
		w.Header().Set("Retry-After", "10")
		Error(w, http.StatusTooManyRequests, "rate_limited", "password service is temporarily busy")
		return
	}
	if err != nil {
		h.databaseError(w, "hash reset password", err)
		return
	}
	consumed, err := h.Postgres.ConsumePasswordEmailToken(
		r.Context(), smtpAuth.HashSecret(request.Token), "reset_password", passwordHash,
	)
	if err != nil {
		h.databaseError(w, "reset account password", err)
		return
	}
	if !consumed {
		Error(w, http.StatusBadRequest, "invalid_request", "password reset token is invalid or expired")
		return
	}
	JSON(w, http.StatusOK, map[string]string{"status": "password_reset"})
}

func (h Handler) sendAccountEmail(r *http.Request, recipient, subject, body string) error {
	if h.AuthMailer == nil {
		return errors.New("authentication email sender is not configured")
	}
	ctx, cancel := context.WithTimeout(r.Context(), 15*time.Second)
	defer cancel()
	return h.AuthMailer.Send(ctx, authmail.Message{
		To: recipient, Subject: subject, Body: body,
	})
}

func selectPasswordMembership(memberships []PasswordMembership, tenantID string) (Principal, error) {
	if len(memberships) == 0 {
		return Principal{}, ErrUnauthorized
	}
	if tenantID == "" && len(memberships) > 1 {
		return Principal{}, ErrTenantSelectionRequired
	}
	for _, membership := range memberships {
		if tenantID != "" && membership.TenantID != tenantID {
			continue
		}
		if membership.Role != Owner && membership.Role != Admin && membership.Role != Member {
			return Principal{}, ErrUnauthorized
		}
		return Principal{
			Subject: "password:" + membership.UserID,
			UserID:  membership.UserID, TenantID: membership.TenantID,
			Role:    membership.Role,
			Scopes:  smtpAuth.PermissionsForRole(string(membership.Role)),
			durable: true,
		}, nil
	}
	return Principal{}, ErrUnauthorized
}

func normalizeEmail(value string) (string, bool) {
	if !utf8.ValidString(value) || len(value) > 320 ||
		strings.TrimSpace(value) != value || strings.ContainsAny(value, "\r\n\t ") {
		return "", false
	}
	address, err := mail.ParseAddress(value)
	if err != nil || address.Address != value || strings.Count(value, "@") != 1 {
		return "", false
	}
	email := strings.ToLower(value)
	if !validEmail(email) {
		return "", false
	}
	return email, true
}

func validPassword(value string) bool {
	if !utf8.ValidString(value) || len(value) < 12 || len(value) > 1024 {
		return false
	}
	for _, char := range value {
		if unicode.IsControl(char) {
			return false
		}
	}
	return true
}

func validOrganizationName(value string) bool {
	value = strings.TrimSpace(value)
	return utf8.ValidString(value) && len(value) >= 1 && len(value) <= 160 &&
		!strings.ContainsRune(value, '\x00')
}

func validOpaqueToken(value string) bool {
	if len(value) < 40 || len(value) > 128 ||
		strings.ContainsAny(value, "\r\n\t ") {
		return false
	}
	for _, char := range value {
		if !(char >= 'a' && char <= 'z') &&
			!(char >= 'A' && char <= 'Z') &&
			!(char >= '0' && char <= '9') &&
			char != '-' && char != '_' {
			return false
		}
	}
	return true
}
