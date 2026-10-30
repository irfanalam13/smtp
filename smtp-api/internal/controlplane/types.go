package controlplane

import (
	"sync"
	"time"
)

type Role string

const (
	Owner  Role = "owner"
	Admin  Role = "admin"
	Member Role = "member"
)

type Domain struct {
	ID        string    `json:"id"`
	TenantID  string    `json:"tenant_id"`
	Domain    string    `json:"domain"`
	Status    string    `json:"status"`
	CreatedAt time.Time `json:"created_at"`
}

type APIKey struct {
	ID        string     `json:"id"`
	Name      string     `json:"name"`
	Prefix    string     `json:"prefix"`
	Scopes    []string   `json:"scopes"`
	ExpiresAt *time.Time `json:"expires_at,omitempty"`
	RevokedAt *time.Time `json:"revoked_at,omitempty"`
	CreatedAt time.Time  `json:"created_at"`
	hash      [32]byte
	TenantID  string `json:"-"`
}

type Mailbox struct {
	ID        string    `json:"id"`
	TenantID  string    `json:"tenant_id"`
	Address   string    `json:"address"`
	Status    string    `json:"status"`
	CreatedAt time.Time `json:"created_at"`
}

type MessageSummary struct {
	ID         string    `json:"id"`
	MailboxID  string    `json:"mailbox_id"`
	Sender     string    `json:"sender"`
	Subject    string    `json:"subject"`
	Size       int       `json:"size"`
	ReceivedAt time.Time `json:"received_at"`
}

type CheckResult struct {
	Kind        string `json:"kind"`
	Status      string `json:"status"`
	Explanation string `json:"explanation"`
	Remediation string `json:"remediation"`
}

type Store struct {
	mu          sync.RWMutex
	tenants     map[string]Tenant
	users       map[string]string
	memberships map[string]map[string]Role
	domains     map[string]Domain
	challenges  map[string]Challenge
	keys        map[string]APIKey
	events      []AuditEvent
	limits      map[string]window
	mailboxes   map[string]Mailbox
	messages    map[string][]MessageSummary
	checks      map[string]DomainCheck
	usage       map[string]Usage
}

type AuditEvent struct {
	TenantID, Actor, Action, TargetType, TargetID string
	Metadata                                      map[string]string
	At                                            time.Time
}

type window struct {
	started time.Time
	count   int
}
