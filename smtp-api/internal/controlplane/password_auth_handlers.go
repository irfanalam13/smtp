package controlplane

import (
	"context"
	"errors"
	"net/http"
	"net/mail"
	"net/url"
	"strings"
	"time"
	"unicode/utf8"

	"example.com/smtp-platform/smtp-api/internal/authmail"
	smtpAuth "example.com/smtp-platform/smtp-auth"
)

const (
	emailVerificationTokenLifetime = 24 * time.Hour
	passwordResetTokenLifetime     = 30 * time.Minute
)

type signUpRequest struct {
	FirstName             string `json:"first_name"`
	LastName              string `json:"last_name"`
	Email                 string `json:"email"`
	Password              string `json:"password"`
	ConfirmPassword       string `json:"confirm_password"`
	TermsAccepted         bool   `json:"terms_accepted"`
	PhoneNumber           string `json:"phone_number"`
	Country               string `json:"country"`
	OrganizationName      string `json:"organization_name"`
	FirstNameLegacy       string `json:"firstName"`
	LastNameLegacy        string `json:"lastName"`
	ConfirmPasswordLegacy string `json:"confirmPassword"`
	TermsAcceptedLegacy   *bool  `json:"termsAccepted"`
	PhoneNumberLegacy     string `json:"phoneNumber"`
	OrganizationLegacy    string `json:"organizationName"`
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

type changePasswordRequest struct {
	CurrentPassword string `json:"current_password"`
	NewPassword     string `json:"new_password"`
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
	limit, window, retryAfter := 3, time.Hour, "3600"
	// A developer commonly corrects a form mistake and retries. Keep the
	// production anti-abuse limit, but avoid locking local development for an
	// hour after three attempts.
	if h.DevMode {
		limit, window, retryAfter = 30, time.Minute, "60"
	}
	if !h.allow(r.Context(), "auth-signup:"+requestClientIP(r), limit, window) {
		w.Header().Set("Retry-After", retryAfter)
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many account registration attempts")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 8192)
	var request signUpRequest
	if err := decodeJSON(r, &request); err != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "request body is invalid")
		return
	}
	request.normalizeLegacyFields()
	email, ok := normalizeEmail(request.Email)
	if !ok {
		Error(w, http.StatusBadRequest, "invalid_email", "enter a valid email address")
		return
	}
	if !validName(request.FirstName, 2, 50) {
		Error(w, http.StatusBadRequest, "invalid_first_name", "first name must be 2 to 50 characters")
		return
	}
	if !validName(request.LastName, 2, 50) {
		Error(w, http.StatusBadRequest, "invalid_last_name", "last name must be 2 to 50 characters")
		return
	}
	if err := smtpAuth.ValidatePassword(request.Password, smtpAuth.DefaultPasswordPolicy); err != nil {
		Error(w, http.StatusBadRequest, "invalid_password", "password must be at least 8 characters and include uppercase, lowercase, number, and special character")
		return
	}
	if request.Password != request.ConfirmPassword {
		Error(w, http.StatusBadRequest, "password_mismatch", "passwords do not match")
		return
	}
	if !request.TermsAccepted {
		Error(w, http.StatusBadRequest, "terms_required", "accept the terms and conditions to continue")
		return
	}
	if !validOptionalField(request.PhoneNumber, 7, 32) {
		Error(w, http.StatusBadRequest, "invalid_phone_number", "phone number must be 7 to 32 characters")
		return
	}
	if !validOptionalField(request.Country, 2, 100) {
		Error(w, http.StatusBadRequest, "invalid_country", "country must be 2 to 100 characters")
		return
	}
	if !validOrganizationName(request.OrganizationName) {
		Error(w, http.StatusBadRequest, "invalid_organization_name", "organization name must be at most 160 characters")
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
	organizationName := strings.TrimSpace(request.OrganizationName)
	if organizationName == "" {
		organizationName = strings.TrimSpace(request.FirstName + " " + request.LastName)
	}
	verifyURL := h.AuthSessions.PublicOrigin + "/verify-email?token=" + url.QueryEscape(token)
	registered, err := h.Postgres.RegisterPasswordAccountWithProfileAndEmail(
		r.Context(), request.FirstName, request.LastName, request.PhoneNumber,
		request.Country, email, passwordHash, organizationName,
		tokenHash, time.Now().Add(emailVerificationTokenLifetime), h.AuthEmailKey,
		authmail.Message{
			To: email, Subject: "Verify your SMTP Platform account",
			Body: "Verify your email address to activate your account. This link expires in 24 hours:\n\n" + verifyURL,
		},
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
	if h.DevMode {
		if err := h.Postgres.AutoVerifyUser(r.Context(), registered.UserID); err != nil {
			h.databaseError(w, "verify development account", err)
			return
		}
		JSON(w, http.StatusAccepted, map[string]string{
			"status":  "verified",
			"message": "Account created and verified. You can now sign in with your password.",
		})
		return
	}

	JSON(w, http.StatusAccepted, map[string]string{
		"status":  "verification_pending",
		"message": "If registration can proceed, a verification email will be sent.",
	})
}

func (request *signUpRequest) normalizeLegacyFields() {
	if request.FirstName == "" {
		request.FirstName = request.FirstNameLegacy
	}
	if request.LastName == "" {
		request.LastName = request.LastNameLegacy
	}
	if request.ConfirmPassword == "" {
		request.ConfirmPassword = request.ConfirmPasswordLegacy
	}
	if !request.TermsAccepted && request.TermsAcceptedLegacy != nil {
		request.TermsAccepted = *request.TermsAcceptedLegacy
	}
	if request.PhoneNumber == "" {
		request.PhoneNumber = request.PhoneNumberLegacy
	}
	if request.OrganizationName == "" {
		request.OrganizationName = request.OrganizationLegacy
	}
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
	if !h.allow(r.Context(), "auth-login:"+requestClientIP(r)+":"+string(emailHash[:8]), 10, time.Minute) {
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
	response := map[string]any{
		"status": "authenticated", "user_id": principal.UserID,
		"tenant_id": principal.TenantID, "expires_at": session.ExpiresAt,
	}
	if h.JWT.valid() {
		token, expiresAt, tokenErr := h.JWT.Issue(principal)
		if tokenErr != nil {
			h.databaseError(w, "issue access token", tokenErr)
			return
		}
		response["access_token"] = token
		response["token_type"] = "Bearer"
		response["access_token_expires_at"] = expiresAt
	}
	JSON(w, http.StatusOK, response)
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
	if !h.allow(r.Context(), "auth-email:"+purpose+":"+requestClientIP(r), 5, time.Hour) {
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
	link := h.AuthSessions.PublicOrigin + path + url.QueryEscape(token)
	_, err = h.Postgres.CreatePasswordEmailTokenWithEmail(
		r.Context(), email, purpose, tokenHash, time.Now().Add(limit), h.AuthEmailKey,
		authmail.Message{To: email, Subject: subject, Body: bodyText + link},
	)
	if err != nil {
		h.databaseError(w, "create account email token", err)
		return
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
	if !h.allow(r.Context(), "auth-password-reset:"+requestClientIP(r), 5, time.Hour) {
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

func (h Handler) changePassword(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "password changes are not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok || !isBrowserSessionPrincipal(principal) {
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	var request changePasswordRequest
	if err := decodeJSON(r, &request); err != nil ||
		request.CurrentPassword == "" ||
		smtpAuth.ValidatePassword(request.NewPassword, smtpAuth.DefaultPasswordPolicy) != nil {
		Error(w, http.StatusBadRequest, "invalid_request", "current or new password is invalid")
		return
	}
	currentHash, err := h.Postgres.PasswordHashForUser(r.Context(), principal.UserID)
	if err != nil {
		h.databaseError(w, "load current password", err)
		return
	}
	matches, err := smtpAuth.VerifyPasswordBounded(currentHash, request.CurrentPassword)
	if err != nil {
		h.databaseError(w, "verify current password", err)
		return
	}
	if !matches {
		Error(w, http.StatusUnauthorized, "unauthorized", "current password is incorrect")
		return
	}
	newHash, err := smtpAuth.HashPasswordBounded(request.NewPassword)
	if errors.Is(err, smtpAuth.ErrPasswordCapacity) {
		w.Header().Set("Retry-After", "10")
		Error(w, http.StatusTooManyRequests, "rate_limited", "password service is temporarily busy")
		return
	}
	if err != nil {
		h.databaseError(w, "hash new password", err)
		return
	}
	if err := h.Postgres.UpdatePasswordHash(r.Context(), principal.UserID, newHash); err != nil {
		h.databaseError(w, "update account password", err)
		return
	}
	h.clearSessionCookies(w)
	JSON(w, http.StatusOK, map[string]string{"status": "password_changed"})
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
	return smtpAuth.ValidatePassword(value, smtpAuth.DefaultPasswordPolicy) == nil
}

func validOrganizationName(value string) bool {
	value = strings.TrimSpace(value)
	return value == "" || (utf8.ValidString(value) && len(value) <= 160 &&
		!strings.ContainsRune(value, '\x00'))
}

func validName(value string, min, max int) bool {
	return utf8.ValidString(value) && strings.TrimSpace(value) == value &&
		len([]rune(value)) >= min && len([]rune(value)) <= max &&
		!strings.ContainsRune(value, '\x00') && !strings.ContainsAny(value, "\r\n")
}

func validOptionalField(value string, min, max int) bool {
	value = strings.TrimSpace(value)
	return value == "" || (utf8.ValidString(value) && len([]rune(value)) >= min &&
		len([]rune(value)) <= max && !strings.ContainsAny(value, "\r\n\x00"))
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
