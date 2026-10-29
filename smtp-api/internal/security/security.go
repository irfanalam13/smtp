package security

import (
	"errors"
	"strings"
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
	if strings.TrimSpace(identity.Subject) == "" ||
		strings.TrimSpace(identity.TenantID) == "" ||
		(identity.Type != HumanIdentity && identity.Type != ServiceIdentity) {
		return ErrInvalidIdentity
	}
	if resource.TenantID == "" || resource.TenantID != identity.TenantID {
		return ErrDenied
	}
	if permission != "" && !identity.Scopes[permission] {
		return ErrDenied
	}
	return nil
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
