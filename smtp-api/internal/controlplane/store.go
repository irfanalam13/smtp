package controlplane

import (
	"crypto/rand"
	"encoding/hex"
	smtpAuth "example.com/smtp-platform/smtp-auth"
	"time"
)

type Principal struct {
	Subject   string
	TenantID  string
	UserID    string
	Role      Role
	Scopes    map[string]bool
	sessionID string
	durable   bool
}

type Tenant struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	Status    string    `json:"status"`
	CreatedAt time.Time `json:"created_at"`
}

func NewStore() *Store {
	return &Store{tenants: map[string]Tenant{}, users: map[string]string{}, memberships: map[string]map[string]Role{}, domains: map[string]Domain{}, challenges: map[string]Challenge{}, keys: map[string]APIKey{}, limits: map[string]window{}, mailboxes: map[string]Mailbox{}, messages: map[string][]MessageSummary{}, checks: map[string]DomainCheck{}, usage: map[string]Usage{}}
}

func newID() (string, error) {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}

func (s *Store) Principal(subject string) Principal {
	s.mu.Lock()
	defer s.mu.Unlock()
	if subject == "" {
		return Principal{}
	}
	userID, ok := s.users[subject]
	if !ok {
		var err error
		userID, err = newID()
		if err != nil {
			return Principal{}
		}
		tenantID, err := newID()
		if err != nil {
			return Principal{}
		}
		s.users[subject] = userID
		s.tenants[tenantID] = Tenant{ID: tenantID, Name: "Local development account", Status: "active", CreatedAt: time.Now().UTC()}
		s.memberships[userID] = map[string]Role{tenantID: Owner}
		return Principal{Subject: subject, UserID: userID, TenantID: tenantID, Role: Owner, Scopes: smtpAuth.PermissionsForRole(string(Owner))}
	}
	for tenantID, role := range s.memberships[userID] {
		return Principal{Subject: subject, UserID: userID, TenantID: tenantID, Role: role, Scopes: smtpAuth.PermissionsForRole(string(role))}
	}
	return Principal{}
}

func (s *Store) Tenant(id string) (Tenant, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	t, ok := s.tenants[id]
	return t, ok
}

func (s *Store) auditLocked(p Principal, action, target, id string) {
	s.events = append(s.events, AuditEvent{TenantID: p.TenantID, Actor: p.UserID, Action: action, TargetType: target, TargetID: id, At: time.Now().UTC()})
}

func (s *Store) Allow(key string, limit int, period time.Duration) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	now := time.Now()
	w := s.limits[key]
	if now.Sub(w.started) >= period {
		w = window{started: now}
	}
	if w.count >= limit {
		return false
	}
	w.count++
	s.limits[key] = w
	return true
}
