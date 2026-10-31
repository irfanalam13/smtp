package security

import (
	"errors"
	"strings"

	smtpAuth "example.com/smtp-platform/smtp-auth"
)

type IdentityType string

const (
	HumanIdentity   IdentityType = "human"
	ServiceIdentity IdentityType = "service"
)

type Identity struct {
	Subject  string
	UserID   string
	TenantID string
	Type     IdentityType
	Role     string
	Scopes   map[string]bool
}

type Resource struct {
	TenantID string
}

var (
	ErrUnauthenticated = errors.New("authentication required")
	ErrDenied          = errors.New("authorization denied")
	ErrInvalidIdentity = errors.New("invalid identity")
)

func Authorize(identity Identity, permission string, resource Resource) error {
	kind := smtpAuth.IdentityKind(identity.Type)
	err := smtpAuth.Authorize(smtpAuth.Identity{
		Subject: identity.Subject, UserID: identity.UserID, TenantID: identity.TenantID,
		Kind: kind, Role: identity.Role, Scopes: identity.Scopes,
	}, permission, resource.TenantID)
	switch {
	case errors.Is(err, smtpAuth.ErrUnauthenticated):
		return ErrUnauthenticated
	case errors.Is(err, smtpAuth.ErrInvalidIdentity):
		return ErrInvalidIdentity
	case errors.Is(err, smtpAuth.ErrDenied):
		return ErrDenied
	default:
		return err
	}
}

func RequireAuthenticated(identity Identity) error {
	if strings.TrimSpace(identity.Subject) == "" || strings.TrimSpace(identity.TenantID) == "" {
		return ErrUnauthenticated
	}
	if identity.Type != HumanIdentity && identity.Type != ServiceIdentity {
		return ErrInvalidIdentity
	}
	return nil
}
