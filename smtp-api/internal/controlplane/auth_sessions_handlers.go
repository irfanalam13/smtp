package controlplane

import (
	"crypto/subtle"
	"errors"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"

	smtpAuth "example.com/smtp-platform/smtp-auth"
)

type BrowserSessionConfig struct {
	Enabled       bool
	PublicOrigin  string
	SessionCookie string
	CSRFCookie    string
	SecureCookies bool
	IdleTTL       time.Duration
	AbsoluteTTL   time.Duration
}

func (c BrowserSessionConfig) valid() bool {
	if !c.Enabled || c.PublicOrigin == "" || c.SessionCookie == "" ||
		c.CSRFCookie == "" || c.IdleTTL <= 0 || c.AbsoluteTTL < c.IdleTTL {
		return false
	}
	if strings.ContainsAny(c.SessionCookie+c.CSRFCookie, "\r\n; ") ||
		(c.SecureCookies &&
			(!strings.HasPrefix(c.SessionCookie, "__Host-") ||
				!strings.HasPrefix(c.CSRFCookie, "__Host-"))) {
		return false
	}
	parsed, err := url.Parse(c.PublicOrigin)
	return err == nil && parsed.Scheme != "" && parsed.Host != "" &&
		parsed.User == nil && parsed.Path == "" && parsed.RawQuery == "" &&
		parsed.Fragment == "" && parsed.String() == c.PublicOrigin
}

func (h Handler) createAuthSession(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil || h.OIDC == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "browser authentication is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
		return
	}
	if !h.allow(r.Context(), "auth-session:"+requestClientIP(r), 10, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many authentication attempts")
		return
	}
	authorization := r.Header.Get("Authorization")
	if !strings.HasPrefix(authorization, "Bearer ") {
		Error(w, http.StatusUnauthorized, "unauthorized", "a verified OIDC access token is required")
		return
	}
	accessToken := strings.TrimSpace(strings.TrimPrefix(authorization, "Bearer "))
	if accessToken == "" || strings.ContainsAny(accessToken, "\r\n\t ") {
		Error(w, http.StatusUnauthorized, "unauthorized", "a verified OIDC access token is required")
		return
	}
	claims, err := h.OIDC.Verify(r.Context(), accessToken)
	if err != nil {
		Error(w, http.StatusUnauthorized, "unauthorized", "a verified OIDC access token is required")
		return
	}
	principal, err := h.Postgres.AuthenticateOIDCSubject(
		r.Context(), claims.Issuer, claims.Subject, r.Header.Get("X-Tenant-ID"),
	)
	if err != nil {
		if errors.Is(err, ErrTenantSelectionRequired) {
			Error(w, http.StatusBadRequest, "tenant_selection_required", "specify a tenant with the X-Tenant-ID header")
			return
		}
		if errors.Is(err, ErrUnauthorized) {
			Error(w, http.StatusUnauthorized, "unauthorized", "authentication is required")
			return
		}
		h.databaseError(w, "resolve browser session identity", err)
		return
	}

	if !h.allow(r.Context(), "auth-session-user:"+principal.UserID, 10, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many authentication attempts")
		return
	}

	session, err := h.issueBrowserSession(w, r, principal)
	if err != nil {
		if errors.Is(err, ErrUnauthorized) {
			Error(w, http.StatusUnauthorized, "unauthorized", "authentication is required")
			return
		}
		h.databaseError(w, "create browser session", err)
		return
	}
	JSON(w, http.StatusCreated, map[string]any{
		"session":   session,
		"user_id":   principal.UserID,
		"tenant_id": principal.TenantID,
	})
}

func (h Handler) issueBrowserSession(
	w http.ResponseWriter,
	r *http.Request,
	principal Principal,
) (AuthSession, error) {
	token, tokenHash, err := smtpAuth.NewSecret()
	if err != nil {
		return AuthSession{}, err
	}
	csrf, csrfHash, err := smtpAuth.NewSecret()
	if err != nil {
		return AuthSession{}, err
	}
	now := time.Now().UTC()
	expiresAt := now.Add(h.AuthSessions.AbsoluteTTL)
	idleExpiresAt := now.Add(h.AuthSessions.IdleTTL)
	if idleExpiresAt.After(expiresAt) {
		idleExpiresAt = expiresAt
	}
	session, err := h.Postgres.CreateAuthSession(
		r.Context(), principal, tokenHash, csrfHash, idleExpiresAt, expiresAt, r.UserAgent(),
	)
	if err != nil {
		return AuthSession{}, err
	}
	if previous, cookieErr := r.Cookie(h.AuthSessions.SessionCookie); cookieErr == nil {
		if err := h.Postgres.RevokeAuthSessionByToken(
			r.Context(), smtpAuth.HashSecret(previous.Value), principal,
		); err != nil {
			return AuthSession{}, err
		}
	}
	h.setSessionCookies(w, token, csrf, expiresAt)
	session.Current = true
	return session, nil
}

func (h Handler) authProviders(w http.ResponseWriter, _ *http.Request) {
	JSON(w, http.StatusOK, map[string]any{
		"oidc_enabled":           h.AuthSessions.valid() && (h.OIDCLogin != nil || h.DevMode),
		"google_enabled":         h.AuthSessions.valid() && (h.SocialOAuth.GoogleClientID != "" || h.DevMode),
		"github_enabled":         h.AuthSessions.valid() && (h.SocialOAuth.GitHubClientID != "" || h.DevMode),
		"password_login_enabled": h.AuthSessions.valid() && h.Postgres != nil,
		"signup_enabled":         h.AuthSessions.valid() && h.Postgres != nil && h.AuthMailer != nil,
		"password_reset_enabled": h.AuthSessions.valid() && h.Postgres != nil && h.AuthMailer != nil,
	})
}

func (h Handler) startOIDCLogin(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.OIDCLogin == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "OIDC login is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
		return
	}
	if !h.allow(r.Context(), "oidc-start:"+requestClientIP(r), 20, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many authentication attempts")
		return
	}
	tenantID := strings.TrimSpace(r.URL.Query().Get("tenant_id"))
	if tenantID == "" {
		tenantID = strings.TrimSpace(r.Header.Get("X-Tenant-ID"))
	}
	if tenantID != "" && !validUUID(tenantID) {
		Error(w, http.StatusBadRequest, "invalid_request", "tenant selection is invalid")
		return
	}
	h.clearOIDCFlowCookies(w)
	state, _, err := smtpAuth.NewSecret()
	if err != nil {
		h.databaseError(w, "start OIDC login", err)
		return
	}
	nonce, _, err := smtpAuth.NewSecret()
	if err != nil {
		h.databaseError(w, "start OIDC login", err)
		return
	}
	verifier := smtpAuth.GeneratePKCEVerifier()
	destination, err := h.OIDCLogin.AuthorizationURL(state, nonce, verifier)
	if err != nil {
		h.databaseError(w, "build OIDC authorization request", err)
		return
	}
	h.setOIDCFlowCookies(w, state, nonce, verifier, tenantID)
	http.Redirect(w, r, destination, http.StatusFound)
}

func (h Handler) completeOIDCLogin(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.OIDCLogin == nil || h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "OIDC login is not configured")
		return
	}
	stateCookie, stateErr := r.Cookie(h.oidcCookieName("state"))
	nonceCookie, nonceErr := r.Cookie(h.oidcCookieName("nonce"))
	verifierCookie, verifierErr := r.Cookie(h.oidcCookieName("pkce"))
	tenantCookie, _ := r.Cookie(h.oidcCookieName("tenant"))
	queryState := r.URL.Query().Get("state")
	queryCode := r.URL.Query().Get("code")
	if stateErr != nil || nonceErr != nil || verifierErr != nil ||
		queryState == "" || queryCode == "" ||
		subtle.ConstantTimeCompare([]byte(queryState), []byte(stateCookie.Value)) != 1 {
		h.clearOIDCFlowCookies(w)
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}
	h.clearOIDCFlowCookies(w)
	identity, err := h.OIDCLogin.AuthenticateCode(
		r.Context(), queryCode, verifierCookie.Value, nonceCookie.Value,
	)
	if err != nil {
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}
	tenantID := cookieValue(tenantCookie)
	principal, err := h.Postgres.AuthenticateOrProvisionOIDCSubject(
		r.Context(), identity.Issuer, identity.Subject, "", "", tenantID,
	)
	if err != nil {
		if errors.Is(err, ErrTenantSelectionRequired) {
			h.redirectOIDCError(w, r, "tenant_selection_required")
			return
		}
		if errors.Is(err, ErrUnauthorized) {
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}
		h.databaseError(w, "resolve OIDC identity", err)
		return
	}
	if !h.allow(r.Context(), "oidc-user:"+principal.UserID, 10, time.Minute) {
		w.Header().Set("Retry-After", "60")
		Error(w, http.StatusTooManyRequests, "rate_limited", "too many authentication attempts")
		return
	}
	if _, err := h.issueBrowserSession(w, r, principal); err != nil {
		if errors.Is(err, ErrUnauthorized) {
			Error(w, http.StatusUnauthorized, "unauthorized", "authentication is required")
			return
		}
		h.databaseError(w, "create OIDC browser session", err)
		return
	}
	http.Redirect(w, r, h.AuthSessions.PublicOrigin, http.StatusSeeOther)
}

func (h Handler) setOIDCFlowCookies(
	w http.ResponseWriter,
	state string,
	nonce string,
	verifier string,
	tenantID string,
) {
	values := map[string]string{
		"state": state, "nonce": nonce, "pkce": verifier, "tenant": tenantID,
	}
	expires := time.Now().Add(5 * time.Minute)
	for suffix, value := range values {
		if suffix == "tenant" && value == "" {
			continue
		}
		http.SetCookie(w, &http.Cookie{
			Name: h.oidcCookieName(suffix), Value: value,
			Path: "/", Expires: expires, MaxAge: 300,
			HttpOnly: true, Secure: h.AuthSessions.SecureCookies, SameSite: http.SameSiteLaxMode,
		})
	}
}

func (h Handler) clearOIDCFlowCookies(w http.ResponseWriter) {
	for _, suffix := range []string{"state", "nonce", "pkce", "tenant"} {
		http.SetCookie(w, &http.Cookie{
			Name: h.oidcCookieName(suffix), Value: "", Path: "/", MaxAge: -1,
			HttpOnly: true, Secure: h.AuthSessions.SecureCookies,
			SameSite: http.SameSiteLaxMode, Expires: time.Unix(1, 0).UTC(),
		})
	}
}

func (h Handler) oidcCookieName(suffix string) string {
	prefix := "smtp_oidc_"
	if h.AuthSessions.SecureCookies {
		prefix = "__Host-smtp_oidc_"
	}
	return prefix + suffix
}

func (h Handler) redirectOIDCError(w http.ResponseWriter, r *http.Request, code string) {
	http.Redirect(
		w, r,
		h.AuthSessions.PublicOrigin+"/login?error="+url.QueryEscape(code),
		http.StatusSeeOther,
	)
}

func requestClientIP(r *http.Request) string {
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err == nil {
		return host
	}
	return r.RemoteAddr
}

func (h Handler) authSessions(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "session storage is not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	if !isBrowserSessionPrincipal(principal) {
		Error(w, http.StatusForbidden, "forbidden", "session management requires a user identity")
		return
	}
	sessions, err := h.Postgres.ListAuthSessions(r.Context(), principal)
	if err != nil {
		h.databaseError(w, "list browser sessions", err)
		return
	}
	for index := range sessions {
		sessions[index].Current = sessions[index].ID == principal.sessionID
	}
	JSON(w, http.StatusOK, map[string]any{"items": sessions})
}

func (h Handler) revokeAuthSession(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "session storage is not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	if !isBrowserSessionPrincipal(principal) {
		Error(w, http.StatusForbidden, "forbidden", "session management requires a browser session")
		return
	}
	if err := h.Postgres.RevokeAuthSession(r.Context(), principal, r.PathValue("id")); err != nil {
		if errors.Is(err, ErrNotFound) {
			Error(w, http.StatusNotFound, "not_found", "session not found")
			return
		}
		if principal.sessionID == r.PathValue("id") {
			h.clearSessionCookies(w)
		}
		h.databaseError(w, "revoke browser session", err)
		return
	}
	if principal.sessionID == r.PathValue("id") {
		h.clearSessionCookies(w)
	}
	JSON(w, http.StatusOK, map[string]string{"status": "revoked"})
}

func (h Handler) logout(w http.ResponseWriter, r *http.Request) {
	h.revokeCurrentSession(w, r)
}

func (h Handler) refreshAuthSession(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil || !h.AuthSessions.valid() {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "browser sessions are not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok || !isBrowserSessionPrincipal(principal) {
		return
	}
	session, err := h.issueBrowserSession(w, r, principal)
	if err != nil {
		h.databaseError(w, "refresh browser session", err)
		return
	}
	JSON(w, http.StatusOK, map[string]any{"status": "refreshed", "session": session})
}

func (h Handler) logoutAll(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "session storage is not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	if !isBrowserSessionPrincipal(principal) {
		Error(w, http.StatusForbidden, "forbidden", "session management requires a user identity")
		return
	}
	if err := h.Postgres.RevokeAllAuthSessions(r.Context(), principal); err != nil {
		h.databaseError(w, "revoke all browser sessions", err)
		return
	}
	h.clearSessionCookies(w)
	JSON(w, http.StatusOK, map[string]string{"status": "revoked"})
}

func (h Handler) revokeCurrentSession(w http.ResponseWriter, r *http.Request) {
	if h.Postgres == nil || !h.AuthSessions.valid() {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "browser sessions are not configured")
		return
	}
	principal, ok := h.principal(w, r, "")
	if !ok {
		return
	}
	cookie, err := r.Cookie(h.AuthSessions.SessionCookie)
	if err != nil || !isBrowserSessionPrincipal(principal) {
		h.clearSessionCookies(w)
		JSON(w, http.StatusOK, map[string]string{"status": "logged_out"})
		return
	}
	if err := h.Postgres.RevokeAuthSessionByToken(
		r.Context(), smtpAuth.HashSecret(cookie.Value), principal,
	); err != nil {
		h.databaseError(w, "revoke browser session", err)
		return
	}
	h.clearSessionCookies(w)
	JSON(w, http.StatusOK, map[string]string{"status": "logged_out"})
}

func isBrowserSessionPrincipal(principal Principal) bool {
	return principal.durable && principal.UserID != "" &&
		strings.HasPrefix(principal.Subject, "session:")
}

func (h Handler) sameOriginRequest(r *http.Request) bool {
	origin := strings.TrimSpace(r.Header.Get("Origin"))
	if origin == "" {
		referer := strings.TrimSpace(r.Header.Get("Referer"))
		if referer == "" {
			return false
		}
		parsed, err := url.Parse(referer)
		if err != nil {
			return false
		}
		origin = parsed.Scheme + "://" + parsed.Host
	}
	if origin == h.AuthSessions.PublicOrigin {
		return true
	}
	// In development environments, allow loopback origins (localhost or 127.0.0.1 on any port)
	if h.DevMode {
		parsedOrigin, err := url.Parse(origin)
		if err == nil {
			host := parsedOrigin.Hostname()
			if strings.EqualFold(host, "localhost") || host == "127.0.0.1" {
				return true
			}
		}
	}
	return false
}

func (h Handler) setSessionCookies(w http.ResponseWriter, token, csrf string, expiresAt time.Time) {
	maxAge := int(time.Until(expiresAt).Seconds())
	if maxAge < 1 {
		maxAge = 1
	}
	http.SetCookie(w, &http.Cookie{
		Name: h.AuthSessions.SessionCookie, Value: token,
		Path: "/", Expires: expiresAt, MaxAge: maxAge,
		HttpOnly: true, Secure: h.AuthSessions.SecureCookies, SameSite: http.SameSiteLaxMode,
	})
	http.SetCookie(w, &http.Cookie{
		Name: h.AuthSessions.CSRFCookie, Value: csrf,
		Path: "/", Expires: expiresAt, MaxAge: maxAge,
		HttpOnly: false, Secure: h.AuthSessions.SecureCookies, SameSite: http.SameSiteLaxMode,
	})
}

func (h Handler) clearSessionCookies(w http.ResponseWriter) {
	for _, name := range []string{h.AuthSessions.SessionCookie, h.AuthSessions.CSRFCookie} {
		http.SetCookie(w, &http.Cookie{
			Name: name, Value: "", Path: "/", MaxAge: -1,
			HttpOnly: name == h.AuthSessions.SessionCookie,
			Secure:   h.AuthSessions.SecureCookies, SameSite: http.SameSiteLaxMode,
			Expires: time.Unix(1, 0).UTC(),
		})
	}
}

func (h Handler) authenticateBrowserSession(r *http.Request) (Principal, error) {
	cookie, err := r.Cookie(h.AuthSessions.SessionCookie)
	if err != nil {
		return Principal{}, ErrUnauthorized
	}
	requireCSRF := r.Method != http.MethodGet && r.Method != http.MethodHead && r.Method != http.MethodOptions
	if requireCSRF && !h.sameOriginRequest(r) {
		return Principal{}, ErrForbidden
	}
	csrfCookie, _ := r.Cookie(h.AuthSessions.CSRFCookie)
	csrfHeader := r.Header.Get("X-CSRF-Token")
	principal, session, err := h.Postgres.AuthenticateAuthSession(
		r.Context(), cookie.Value, cookieValue(csrfCookie), csrfHeader, requireCSRF,
		h.AuthSessions.IdleTTL,
	)
	if err != nil {
		return Principal{}, err
	}
	principal.sessionID = session.ID
	return principal, nil
}

func cookieValue(cookie *http.Cookie) string {
	if cookie == nil {
		return ""
	}
	return cookie.Value
}
