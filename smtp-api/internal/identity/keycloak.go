package identity

import (
	"context"
	"errors"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/coreos/go-oidc/v3/oidc"
)

const maxAccessTokenBytes = 32 << 10

type KeycloakVerifier struct {
	verifier *oidc.IDTokenVerifier
	client   *http.Client
}

type Claims struct {
	Issuer  string
	Subject string
}

func NewKeycloakVerifier(
	ctx context.Context,
	issuer string,
	audience string,
) (*KeycloakVerifier, error) {
	issuer = strings.TrimSpace(issuer)
	audience = strings.TrimSpace(audience)
	if audience == "" {
		return nil, errors.New("OIDC API audience is required")
	}
	if err := validateIssuer(issuer); err != nil {
		return nil, err
	}

	client := &http.Client{
		Timeout: 5 * time.Second,
		CheckRedirect: func(_ *http.Request, _ []*http.Request) error {
			return http.ErrUseLastResponse
		},
	}
	provider, err := oidc.NewProvider(oidc.ClientContext(ctx, client), issuer)
	if err != nil {
		return nil, errors.New("discover configured OIDC issuer")
	}
	return &KeycloakVerifier{
		verifier: provider.Verifier(&oidc.Config{ClientID: audience}),
		client:   client,
	}, nil
}

func (v *KeycloakVerifier) Verify(ctx context.Context, rawToken string) (Claims, error) {
	if v == nil || v.verifier == nil || len(rawToken) == 0 || len(rawToken) > maxAccessTokenBytes {
		return Claims{}, errors.New("invalid OIDC access token")
	}
	token, err := v.verifier.Verify(oidc.ClientContext(ctx, v.client), rawToken)
	if err != nil || token == nil {
		return Claims{}, errors.New("invalid OIDC access token")
	}
	var keycloakClaims struct {
		Type string `json:"typ"`
	}
	if token.Claims(&keycloakClaims) != nil || keycloakClaims.Type != "Bearer" ||
		strings.TrimSpace(token.Subject) == "" || len(token.Subject) > 255 ||
		strings.TrimSpace(token.Issuer) == "" {
		return Claims{}, errors.New("invalid OIDC access token")
	}
	return Claims{Issuer: token.Issuer, Subject: token.Subject}, nil
}

func validateIssuer(issuer string) error {
	parsed, err := url.Parse(issuer)
	if err != nil || parsed.Host == "" || parsed.User != nil ||
		parsed.RawQuery != "" || parsed.Fragment != "" {
		return errors.New("OIDC issuer URL is invalid")
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
	return errors.New("OIDC issuer must use HTTPS")
}
