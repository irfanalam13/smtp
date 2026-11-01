package controlplane

import (
	"context"
	"encoding/json"
	"fmt"
	"html"
	"io"
	"log/slog"
	"net/http"
	"strconv"
	"strings"
	"time"

	smtpAuth "example.com/smtp-platform/smtp-auth"
	"golang.org/x/oauth2"
)

type SocialOAuthConfig struct {
	GoogleClientID     string
	GoogleClientSecret string
	GoogleRedirectURI  string

	GitHubClientID     string
	GitHubClientSecret string
	GitHubRedirectURI  string
}

func (h Handler) startGoogleOAuth(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "authentication is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
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
		h.databaseError(w, "start google oauth", err)
		return
	}
	nonce, _, err := smtpAuth.NewSecret()
	if err != nil {
		h.databaseError(w, "start google oauth", err)
		return
	}
	verifier := smtpAuth.GeneratePKCEVerifier()
	if err := h.createOAuthAuthorizationFlow(r, "google", state, nonce, verifier, tenantID); err != nil {
		h.databaseError(w, "persist Google OAuth authorization flow", err)
		return
	}
	h.setOIDCFlowCookies(w, state, nonce, verifier, tenantID)

	redirectURI := h.SocialOAuth.GoogleRedirectURI
	if redirectURI == "" {
		redirectURI = h.AuthSessions.PublicOrigin + "/api/backend/v1/auth/oauth/google/callback"
	}

	if h.SocialOAuth.GoogleClientID != "" && h.SocialOAuth.GoogleClientSecret != "" {
		oauthConf := &oauth2.Config{
			ClientID:     h.SocialOAuth.GoogleClientID,
			ClientSecret: h.SocialOAuth.GoogleClientSecret,
			RedirectURL:  redirectURI,
			Scopes:       []string{"openid", "email", "profile"},
			Endpoint: oauth2.Endpoint{
				AuthURL:  "https://accounts.google.com/o/oauth2/v2/auth",
				TokenURL: "https://oauth2.googleapis.com/token",
			},
		}
		authURL := oauthConf.AuthCodeURL(
			state,
			oauth2.AccessTypeOffline,
			oauth2.S256ChallengeOption(verifier),
			oauth2.SetAuthURLParam("nonce", nonce),
		)
		http.Redirect(w, r, authURL, http.StatusFound)
		return
	}

	// Development / simulated mode
	if h.DevMode {
		renderDevOAuthPage(w, "Google", "developer@gmail.com", "Google Developer", state, "/api/backend/v1/auth/oauth/google/callback")
		return
	}

	Error(w, http.StatusServiceUnavailable, "auth_unavailable", "Google OAuth credentials are not configured in production")
}

func (h Handler) completeGoogleOAuth(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "authentication is not configured")
		return
	}
	queryCode := r.URL.Query().Get("code")
	if queryCode == "" {
		h.clearOIDCFlowCookies(w)
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}
	tenantID, _, verifier, ok := h.consumeOAuthAuthorizationFlow(r, "google")
	if !ok {
		h.clearOIDCFlowCookies(w)
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}
	var subject, email, name string

	if queryCode == "dev-google-code" && h.DevMode {
		h.clearOIDCFlowCookies(w)
		email = strings.TrimSpace(r.URL.Query().Get("email"))
		if email == "" {
			email = "developer@gmail.com"
		}
		name = strings.TrimSpace(r.URL.Query().Get("name"))
		if name == "" {
			name = "Google Developer"
		}
		subject = "google:dev-" + email
	} else if h.SocialOAuth.GoogleClientID != "" && h.SocialOAuth.GoogleClientSecret != "" {
		h.clearOIDCFlowCookies(w)
		redirectURI := h.SocialOAuth.GoogleRedirectURI
		if redirectURI == "" {
			redirectURI = h.AuthSessions.PublicOrigin + "/api/backend/v1/auth/oauth/google/callback"
		}
		oauthConf := &oauth2.Config{
			ClientID:     h.SocialOAuth.GoogleClientID,
			ClientSecret: h.SocialOAuth.GoogleClientSecret,
			RedirectURL:  redirectURI,
			Endpoint: oauth2.Endpoint{
				AuthURL:  "https://accounts.google.com/o/oauth2/v2/auth",
				TokenURL: "https://oauth2.googleapis.com/token",
			},
		}
		ctx, cancel := context.WithTimeout(r.Context(), 10*time.Second)
		defer cancel()

		token, err := oauthConf.Exchange(ctx, queryCode, oauth2.VerifierOption(verifier))
		if err != nil {
			slog.Error("google token exchange failed", "error", err)
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}

		client := oauthConf.Client(ctx, token)
		userResp, err := client.Get("https://openidconnect.googleapis.com/v1/userinfo")
		if err != nil {
			slog.Error("google userinfo request failed", "error", err)
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}
		defer func() { _ = userResp.Body.Close() }()

		var userInfo struct {
			Sub           string `json:"sub"`
			Email         string `json:"email"`
			EmailVerified bool   `json:"email_verified"`
			Name          string `json:"name"`
		}
		if err := decodeOAuthJSON(userResp, &userInfo); err != nil {
			slog.Error("google userinfo decode failed", "error", err)
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}
		if userInfo.Sub == "" {
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}
		subject = "google:" + userInfo.Sub
		// Do not use an unverified provider email to attach this identity to an
		// existing password account.
		if userInfo.EmailVerified {
			email = userInfo.Email
		}
		name = userInfo.Name
	} else {
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}

	principal, err := h.Postgres.AuthenticateOrProvisionOIDCSubject(
		r.Context(), "https://accounts.google.com", subject, email, name, tenantID,
	)
	if err != nil {
		slog.Error("google oidc user provision failed", "error", err)
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}

	if _, err := h.issueBrowserSession(w, r, principal); err != nil {
		h.databaseError(w, "create google session", err)
		return
	}
	http.Redirect(w, r, h.AuthSessions.PublicOrigin+"/home", http.StatusFound)
}

func (h Handler) startGitHubOAuth(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "authentication is not configured")
		return
	}
	if !h.sameOriginRequest(r) {
		Error(w, http.StatusForbidden, "forbidden", "request origin is not allowed")
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
		h.databaseError(w, "start github oauth", err)
		return
	}
	nonce, _, err := smtpAuth.NewSecret()
	if err != nil {
		h.databaseError(w, "start github oauth", err)
		return
	}
	verifier := smtpAuth.GeneratePKCEVerifier()
	if err := h.createOAuthAuthorizationFlow(r, "github", state, nonce, verifier, tenantID); err != nil {
		h.databaseError(w, "persist GitHub OAuth authorization flow", err)
		return
	}
	h.setOIDCFlowCookies(w, state, nonce, verifier, tenantID)

	redirectURI := h.SocialOAuth.GitHubRedirectURI
	if redirectURI == "" {
		redirectURI = h.AuthSessions.PublicOrigin + "/api/backend/v1/auth/oauth/github/callback"
	}

	if h.SocialOAuth.GitHubClientID != "" && h.SocialOAuth.GitHubClientSecret != "" {
		oauthConf := &oauth2.Config{
			ClientID:     h.SocialOAuth.GitHubClientID,
			ClientSecret: h.SocialOAuth.GitHubClientSecret,
			RedirectURL:  redirectURI,
			Scopes:       []string{"read:user", "user:email"},
			Endpoint: oauth2.Endpoint{
				AuthURL:  "https://github.com/login/oauth/authorize",
				TokenURL: "https://github.com/login/oauth/access_token",
			},
		}
		authURL := oauthConf.AuthCodeURL(state, oauth2.S256ChallengeOption(verifier))
		http.Redirect(w, r, authURL, http.StatusFound)
		return
	}

	// Development / simulated mode
	if h.DevMode {
		renderDevOAuthPage(w, "GitHub", "dev@github.com", "github-developer", state, "/api/backend/v1/auth/oauth/github/callback")
		return
	}

	Error(w, http.StatusServiceUnavailable, "auth_unavailable", "GitHub OAuth credentials are not configured in production")
}

func (h Handler) completeGitHubOAuth(w http.ResponseWriter, r *http.Request) {
	if !h.AuthSessions.valid() || h.Postgres == nil {
		Error(w, http.StatusServiceUnavailable, "auth_unavailable", "authentication is not configured")
		return
	}
	queryCode := r.URL.Query().Get("code")
	if queryCode == "" {
		h.clearOIDCFlowCookies(w)
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}
	tenantID, _, verifier, ok := h.consumeOAuthAuthorizationFlow(r, "github")
	if !ok {
		h.clearOIDCFlowCookies(w)
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}
	var subject, email, name string

	if queryCode == "dev-github-code" && h.DevMode {
		h.clearOIDCFlowCookies(w)
		email = strings.TrimSpace(r.URL.Query().Get("email"))
		if email == "" {
			email = "dev@github.com"
		}
		name = strings.TrimSpace(r.URL.Query().Get("name"))
		if name == "" {
			name = "github-developer"
		}
		subject = "github:dev-" + name
	} else if h.SocialOAuth.GitHubClientID != "" && h.SocialOAuth.GitHubClientSecret != "" {
		h.clearOIDCFlowCookies(w)
		redirectURI := h.SocialOAuth.GitHubRedirectURI
		if redirectURI == "" {
			redirectURI = h.AuthSessions.PublicOrigin + "/api/backend/v1/auth/oauth/github/callback"
		}
		oauthConf := &oauth2.Config{
			ClientID:     h.SocialOAuth.GitHubClientID,
			ClientSecret: h.SocialOAuth.GitHubClientSecret,
			RedirectURL:  redirectURI,
			Endpoint: oauth2.Endpoint{
				AuthURL:  "https://github.com/login/oauth/authorize",
				TokenURL: "https://github.com/login/oauth/access_token",
			},
		}
		ctx, cancel := context.WithTimeout(r.Context(), 10*time.Second)
		defer cancel()

		token, err := oauthConf.Exchange(ctx, queryCode, oauth2.VerifierOption(verifier))
		if err != nil {
			slog.Error("github token exchange failed", "error", err)
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}

		client := oauthConf.Client(ctx, token)
		userResp, err := client.Get("https://api.github.com/user")
		if err != nil {
			slog.Error("github user request failed", "error", err)
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}
		defer func() { _ = userResp.Body.Close() }()

		var ghUser struct {
			ID    int64  `json:"id"`
			Login string `json:"login"`
			Name  string `json:"name"`
			Email string `json:"email"`
		}
		if err := json.NewDecoder(userResp.Body).Decode(&ghUser); err != nil {
			slog.Error("github user decode failed", "error", err)
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}
		if ghUser.ID <= 0 {
			h.redirectOIDCError(w, r, "oidc_failed")
			return
		}
		subject = "github:" + strconv.FormatInt(ghUser.ID, 10)
		name = ghUser.Name
		if name == "" {
			name = ghUser.Login
		}
		// The public-profile email does not state whether it is verified. Use the
		// scoped emails endpoint and only attach a verified primary address.
		emailsResp, err := client.Get("https://api.github.com/user/emails")
		if err == nil {
			defer func() { _ = emailsResp.Body.Close() }()
			var emails []struct {
				Email    string `json:"email"`
				Primary  bool   `json:"primary"`
				Verified bool   `json:"verified"`
			}
			if decodeOAuthJSON(emailsResp, &emails) == nil {
				for _, item := range emails {
					if item.Primary && item.Verified {
						email = item.Email
						break
					}
				}
			}
		}
	} else {
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}

	principal, err := h.Postgres.AuthenticateOrProvisionOIDCSubject(
		r.Context(), "https://github.com", subject, email, name, tenantID,
	)
	if err != nil {
		slog.Error("github oidc user provision failed", "error", err)
		h.redirectOIDCError(w, r, "oidc_failed")
		return
	}

	if _, err := h.issueBrowserSession(w, r, principal); err != nil {
		h.databaseError(w, "create github session", err)
		return
	}
	http.Redirect(w, r, h.AuthSessions.PublicOrigin+"/home", http.StatusFound)
}

func decodeOAuthJSON(response *http.Response, target any) error {
	if response == nil || response.StatusCode < http.StatusOK || response.StatusCode >= http.StatusMultipleChoices {
		return fmt.Errorf("oauth provider returned an unsuccessful response")
	}
	contentType := strings.ToLower(response.Header.Get("Content-Type"))
	if !strings.HasPrefix(contentType, "application/json") {
		return fmt.Errorf("oauth provider returned an unexpected content type")
	}
	decoder := json.NewDecoder(io.LimitReader(response.Body, 64<<10))
	if err := decoder.Decode(target); err != nil {
		return fmt.Errorf("decode oauth provider response: %w", err)
	}
	return nil
}

func renderDevOAuthPage(w http.ResponseWriter, provider, defaultEmail, defaultName, state, callbackURI string) {
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.WriteHeader(http.StatusOK)
	_, _ = io.WriteString(w, fmt.Sprintf(`<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>%s Sign-In (Development Mode)</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #f4f6f8; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; padding: 16px; }
    .card { background: #fff; border: 1px solid #e1e4e8; border-radius: 12px; padding: 32px; max-width: 440px; width: 100%%; box-shadow: 0 4px 12px rgba(0,0,0,0.06); }
    h2 { margin-top: 0; color: #1f2937; font-size: 20px; }
    p { color: #4b5563; font-size: 14px; line-height: 1.5; }
    .badge { display: inline-block; background: #dcfce7; color: #166534; font-size: 11px; font-weight: 600; padding: 2px 8px; border-radius: 6px; margin-bottom: 12px; }
    label { display: block; font-size: 13px; font-weight: 500; color: #374151; margin-top: 14px; }
    input { width: 100%%; box-sizing: border-box; padding: 9px 12px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 14px; margin-top: 4px; }
    button { width: 100%%; background: #059669; color: #fff; border: none; border-radius: 8px; padding: 12px; font-size: 14px; font-weight: 600; cursor: pointer; margin-top: 20px; transition: background 0.15s; }
    button:hover { background: #047857; }
    .hint { font-size: 12px; color: #6b7280; margin-top: 18px; border-top: 1px solid #f3f4f6; pt: 12px; }
  </style>
</head>
<body>
  <div class="card">
    <span class="badge">DEVELOPMENT ENVIRONMENT</span>
    <h2>Sign in with %s</h2>
    <p>Simulating OAuth2 callback for local development. Confirm your account details below:</p>
    <form method="GET" action="%s">
      <input type="hidden" name="state" value="%s">
      <input type="hidden" name="code" value="dev-%s-code">
      <label>Email Address
        <input type="email" name="email" value="%s" required>
      </label>
      <label>Full Name / Username
        <input type="text" name="name" value="%s" required>
      </label>
      <button type="submit">Authorize %s Identity &rarr;</button>
    </form>
    <p class="hint">To connect real accounts, configure %s_CLIENT_ID and %s_CLIENT_SECRET in <code>.env</code>.</p>
  </div>
</body>
</html>`,
		html.EscapeString(provider),
		html.EscapeString(provider),
		html.EscapeString(callbackURI),
		html.EscapeString(state),
		strings.ToLower(html.EscapeString(provider)),
		html.EscapeString(defaultEmail),
		html.EscapeString(defaultName),
		html.EscapeString(provider),
		strings.ToUpper(html.EscapeString(provider)),
		strings.ToUpper(html.EscapeString(provider)),
	))
}
