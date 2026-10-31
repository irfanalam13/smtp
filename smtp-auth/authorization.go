package smtpauth

import (
	"errors"
	"strings"
)

type IdentityKind string

const (
	HumanIdentity   IdentityKind = "human"
	ServiceIdentity IdentityKind = "service"
)

var (
	ErrUnauthenticated = errors.New("authentication required")
	ErrDenied          = errors.New("authorization denied")
	ErrInvalidIdentity = errors.New("invalid identity")
)

type Identity struct {
	Subject  string
	UserID   string
	TenantID string
	Kind     IdentityKind
	Role     string
	Scopes   map[string]bool
}

var rolePermissions = map[string]map[string]bool{
	"owner": {
		"domains:read": true, "domains:write": true,
		"keys:write":            true,
		"smtp:credentials:read": true, "smtp:credentials:write": true,
		"delivery:read": true, "delivery:write": true,
		"mailbox:read": true, "mailbox:write": true, "mailbox:manage": true,
	},
	"admin": {
		"domains:read": true, "domains:write": true,
		"keys:write":            true,
		"smtp:credentials:read": true, "smtp:credentials:write": true,
		"delivery:read": true, "delivery:write": true,
		"mailbox:read": true, "mailbox:write": true, "mailbox:manage": true,
	},
	"member": {
		"domains:read":  true,
		"delivery:read": true,
		"mailbox:read":  true, "mailbox:write": true,
	},
}

func Authorize(identity Identity, permission string, resourceTenant string) error {
	if strings.TrimSpace(identity.Subject) == "" ||
		strings.TrimSpace(identity.TenantID) == "" ||
		(identity.Kind != HumanIdentity && identity.Kind != ServiceIdentity) {
		return ErrInvalidIdentity
	}
	if resourceTenant == "" || resourceTenant != identity.TenantID {
		return ErrDenied
	}
	if permission == "" {
		return nil
	}

	switch identity.Kind {
	case HumanIdentity:
		if identity.Role != "" && !rolePermissions[identity.Role][permission] {
			return ErrDenied
		}
		if identity.Role == "" && !identity.Scopes[permission] {
			return ErrDenied
		}
	case ServiceIdentity:
		if !identity.Scopes[permission] {
			return ErrDenied
		}
	default:
		return ErrInvalidIdentity
	}
	return nil
}

func PermissionsForRole(role string) map[string]bool {
	permissions := rolePermissions[role]
	result := make(map[string]bool, len(permissions))
	for permission, allowed := range permissions {
		result[permission] = allowed
	}
	return result
}
