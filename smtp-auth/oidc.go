package smtpauth

import (
	"context"
	"crypto/subtle"
	"errors"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/coreos/go-oidc/v3/oidc"
	"golang.org/x/oauth2"
)

const maxAuthorizationCodeBytes = 4096

type OIDCConfig struct {
	Issuer       string
	ClientID     string
	ClientSecret string
	RedirectURL  string
}

type VerifiedIdentity struct {
	Issuer  string
	Subject string
}

type OIDCClient struct {
	provider *oidc.Provider
	verifier *oidc.IDTokenVerifier
	oauth    oauth2.Config
	client   *http.Client
}

func NewOIDCClient(ctx context.Context, config OIDCConfig) (*OIDCClient, error) {
	config.Issuer = strings.TrimSpace(config.Issuer)
	config.ClientID = strings.TrimSpace(config.ClientID)
	config.ClientSecret = strings.TrimSpace(config.ClientSecret)
	config.RedirectURL = strings.TrimSpace(config.RedirectURL)
	if config.ClientID == "" || config.ClientSecret == "" {
		return nil, errors.New("OIDC client credentials are required")
	}
	if err := validateOIDCURL(config.Issuer, true); err != nil {
		return nil, errors.New("OIDC issuer URL is invalid")
	}
	if err := validateOIDCURL(config.RedirectURL, false); err != nil {
		return nil, errors.New("OIDC redirect URL is invalid")
	}

	client := &http.Client{
		Timeout: 8 * time.Second,
		CheckRedirect: func(_ *http.Request, _ []*http.Request) error {
			return http.ErrUseLastResponse
		},
	}
	provider, err := oidc.NewProvider(oidc.ClientContext(ctx, client), config.Issuer)
	if err != nil {
		return nil, errors.New("discover configured OIDC issuer")
	}
	endpoint := provider.Endpoint()
	if err := validateOIDCURL(endpoint.AuthURL, false); err != nil {
		return nil, errors.New("OIDC authorization endpoint is invalid")
	}
	if err := validateOIDCURL(endpoint.TokenURL, false); err != nil {
		return nil, errors.New("OIDC token endpoint is invalid")
	}
	oauthConfig := oauth2.Config{
		ClientID:     config.ClientID,
		ClientSecret: config.ClientSecret,
		Endpoint:     endpoint,
		RedirectURL:  config.RedirectURL,
		Scopes:       []string{oidc.ScopeOpenID},
	}
	return &OIDCClient{
		provider: provider,
		verifier: provider.Verifier(&oidc.Config{ClientID: config.ClientID}),
		oauth:    oauthConfig,
		client:   client,
	}, nil
}

func GeneratePKCEVerifier() string {
	return oauth2.GenerateVerifier()
}

func (client *OIDCClient) AuthorizationURL(state, nonce, verifier string) (string, error) {
	if client == nil || client.provider == nil || len(state) < 32 || len(nonce) < 32 ||
		len(verifier) < 43 || strings.ContainsAny(state+nonce+verifier, "\r\n\t ") {
		return "", errors.New("invalid OIDC authorization request")
	}
	return client.oauth.AuthCodeURL(
		state,
		oauth2.S256ChallengeOption(verifier),
		oauth2.SetAuthURLParam("nonce", nonce),
	), nil
}

func (client *OIDCClient) AuthenticateCode(
	ctx context.Context,
	code string,
	verifier string,
	expectedNonce string,
) (VerifiedIdentity, error) {
	if client == nil || client.provider == nil || client.verifier == nil ||
		len(code) == 0 || len(code) > maxAuthorizationCodeBytes ||
		strings.ContainsAny(code+verifier+expectedNonce, "\r\n") ||
		len(verifier) < 43 || len(verifier) > 128 || len(expectedNonce) < 32 {
		return VerifiedIdentity{}, errors.New("invalid OIDC authorization response")
	}
	exchangeContext := context.WithValue(ctx, oauth2.HTTPClient, client.client)
	token, err := client.oauth.Exchange(
		exchangeContext,
		code,
		oauth2.VerifierOption(verifier),
	)
	if err != nil {
		return VerifiedIdentity{}, errors.New("exchange OIDC authorization code")
	}
	rawIDToken, ok := token.Extra("id_token").(string)
	if !ok || rawIDToken == "" || len(rawIDToken) > 32*1024 {
		return VerifiedIdentity{}, errors.New("OIDC response did not contain a valid ID token")
	}
	idToken, err := client.verifier.Verify(oidc.ClientContext(ctx, client.client), rawIDToken)
	now := time.Now()
	if err != nil || idToken == nil ||
		subtle.ConstantTimeCompare([]byte(idToken.Nonce), []byte(expectedNonce)) != 1 ||
		idToken.IssuedAt.IsZero() ||
		idToken.IssuedAt.After(now.Add(time.Minute)) ||
		now.Sub(idToken.IssuedAt) > 10*time.Minute ||
		strings.TrimSpace(idToken.Subject) == "" || len(idToken.Subject) > 255 {
		return VerifiedIdentity{}, errors.New("OIDC ID token validation failed")
	}
	return VerifiedIdentity{Issuer: idToken.Issuer, Subject: idToken.Subject}, nil
}

func validateOIDCURL(raw string, issuer bool) error {
	parsed, err := url.Parse(raw)
	if err != nil || parsed.Host == "" || parsed.User != nil || parsed.RawQuery != "" ||
		parsed.Fragment != "" {
		return errors.New("invalid OIDC URL")
	}
	if parsed.Scheme == "https" {
		return nil
	}
	host := parsed.Hostname()
	ip := net.ParseIP(host)
	if parsed.Scheme == "http" && (strings.EqualFold(host, "localhost") ||
		(ip != nil && ip.IsLoopback())) {
		return nil
	}
	if issuer {
		return errors.New("OIDC issuer must use HTTPS")
	}
	return errors.New("OIDC endpoint must use HTTPS")
}
