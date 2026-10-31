package smtpauth

import (
	"errors"
	"testing"
)

func TestAuthorizeUsesRoleAndTenantBoundaries(t *testing.T) {
	base := Identity{
		Subject: "user-1", UserID: "user-1", TenantID: "tenant-1",
		Kind: HumanIdentity, Role: "member",
	}
	tests := []struct {
		name       string
		identity   Identity
		permission string
		resource   string
		wantErr    error
	}{
		{name: "member can read domains", identity: base, permission: "domains:read", resource: "tenant-1"},
		{name: "member cannot write domains", identity: base, permission: "domains:write", resource: "tenant-1", wantErr: ErrDenied},
		{name: "cross tenant is denied", identity: base, permission: "domains:read", resource: "tenant-2", wantErr: ErrDenied},
		{name: "missing role is denied", identity: Identity{Subject: "x", TenantID: "tenant-1", Kind: HumanIdentity, Role: "unknown"}, permission: "domains:read", resource: "tenant-1", wantErr: ErrDenied},
		{name: "service scope is explicit", identity: Identity{Subject: "key", TenantID: "tenant-1", Kind: ServiceIdentity, Scopes: map[string]bool{"domains:read": true}}, permission: "domains:read", resource: "tenant-1"},
		{name: "service missing scope is denied", identity: Identity{Subject: "key", TenantID: "tenant-1", Kind: ServiceIdentity}, permission: "domains:read", resource: "tenant-1", wantErr: ErrDenied},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			err := Authorize(test.identity, test.permission, test.resource)
			if !errors.Is(err, test.wantErr) {
				t.Fatalf("Authorize() error = %v, want %v", err, test.wantErr)
			}
		})
	}
}

func TestPermissionsForRoleReturnsCopy(t *testing.T) {
	permissions := PermissionsForRole("owner")
	permissions["domains:read"] = false
	if !PermissionsForRole("owner")["domains:read"] {
		t.Fatal("returned permissions mutate the registry")
	}
}

func TestPasswordHashVerification(t *testing.T) {
	password := "correct horse battery staple"
	encoded, err := HashPassword(password)
	if err != nil {
		t.Fatal(err)
	}
	if ok, err := VerifyPassword(encoded, password); err != nil || !ok {
		t.Fatalf("VerifyPassword(valid) = (%t, %v)", ok, err)
	}
	if ok, err := VerifyPassword(encoded, "a different password"); err != nil || ok {
		t.Fatalf("VerifyPassword(wrong) = (%t, %v)", ok, err)
	}
}

func TestVerifyPasswordRejectsUnsafeParameters(t *testing.T) {
	for _, encoded := range []string{
		"not-a-hash",
		"$argon2id$v=19$m=999999999,t=3,p=2$c2FsdC1mb3ItdGVzdC0xMjM0$AAAAAAAAAAAAAAAAAAAAAA",
	} {
		if _, err := VerifyPassword(encoded, "a password"); err == nil {
			t.Errorf("VerifyPassword(%q) succeeded", encoded)
		}
	}
}

func TestBoundedPasswordOperationsRejectSaturation(t *testing.T) {
	passwordSlots <- struct{}{}
	passwordSlots <- struct{}{}
	defer func() {
		<-passwordSlots
		<-passwordSlots
	}()
	if _, err := HashPasswordBounded("correct horse battery staple"); !errors.Is(err, ErrPasswordCapacity) {
		t.Fatalf("HashPasswordBounded() error=%v, want capacity error", err)
	}
	if _, err := VerifyPasswordBounded("unused", "correct horse battery staple"); !errors.Is(err, ErrPasswordCapacity) {
		t.Fatalf("VerifyPasswordBounded() error=%v, want capacity error", err)
	}
}

func TestCSRFVerification(t *testing.T) {
	secret, expected, err := NewSecret()
	if err != nil {
		t.Fatal(err)
	}
	if !VerifyCSRF(secret, secret, expected) {
		t.Fatal("valid CSRF token rejected")
	}
	if VerifyCSRF(secret, "other", expected) {
		t.Fatal("mismatched CSRF token accepted")
	}
	if VerifyCSRF(secret, secret, HashSecret("different")) {
		t.Fatal("CSRF token with wrong stored verifier accepted")
	}
}
