package security

import "testing"

func TestAuthorizeDefaultsToDeny(t *testing.T) {
	identity := Identity{
		Subject:  "user-1",
		UserID:   "user-1",
		TenantID: "tenant-1",
		Type:     HumanIdentity,
		Scopes:   map[string]bool{},
	}
	if err := Authorize(identity, "domains:write", Resource{TenantID: "tenant-1"}); err != ErrDenied {
		t.Fatalf("expected denial, got %v", err)
	}
}

func TestAuthorizeRejectsCrossTenantResource(t *testing.T) {
	identity := Identity{
		Subject:  "service-1",
		TenantID: "tenant-1",
		Type:     ServiceIdentity,
		Scopes:   map[string]bool{"domains:read": true},
	}
	if err := Authorize(identity, "domains:read", Resource{TenantID: "tenant-2"}); err != ErrDenied {
		t.Fatalf("expected cross-tenant denial, got %v", err)
	}
}

func TestAuthorizeAllowsExplicitPermission(t *testing.T) {
	identity := Identity{
		Subject:  "user-1",
		TenantID: "tenant-1",
		Type:     HumanIdentity,
		Scopes:   map[string]bool{"domains:read": true},
	}
	if err := Authorize(identity, "domains:read", Resource{TenantID: "tenant-1"}); err != nil {
		t.Fatalf("expected allow, got %v", err)
	}
}
