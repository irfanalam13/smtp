package controlplane

import (
	"example.com/smtp-platform/smtp-api/internal/security"
	"net/http"
	"strings"
)

func (s *Store) Auth(r *http.Request, dev bool) (Principal, error) {
	if h := r.Header.Get("Authorization"); strings.HasPrefix(h, "Bearer ") {
		secret := strings.TrimSpace(strings.TrimPrefix(h, "Bearer "))
		if secret == "" || strings.ContainsAny(secret, "\r\n") {
			return Principal{}, ErrUnauthorized
		}
		return s.AuthenticateKey(secret)
	}
	if dev && r.Header.Get("X-Dev-Subject") != "" {
		return s.Principal(r.Header.Get("X-Dev-Subject")), nil
	}
	return Principal{}, ErrUnauthorized
}

func Require(p Principal, scope string) error {
	if err := security.RequireAuthenticated(security.Identity{
		Subject:  p.Subject,
		UserID:   p.UserID,
		TenantID: p.TenantID,
		Type:     identityType(p),
		Scopes:   p.Scopes,
	}); err != nil {
		return ErrUnauthorized
	}
	if err := security.Authorize(security.Identity{
		Subject:  p.Subject,
		UserID:   p.UserID,
		TenantID: p.TenantID,
		Type:     identityType(p),
		Scopes:   p.Scopes,
	}, scope, security.Resource{TenantID: p.TenantID}); err != nil {
		return ErrForbidden
	}
	return nil
}

func identityType(p Principal) security.IdentityType {
	if strings.HasPrefix(p.Subject, "api-key:") {
		return security.ServiceIdentity
	}
	return security.HumanIdentity
}
