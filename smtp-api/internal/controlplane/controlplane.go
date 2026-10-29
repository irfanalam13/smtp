package controlplane

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"io"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"
	"unicode"

	"example.com/smtp-platform/smtp-api/internal/security"
)

type Role string

const (
	Owner  Role = "owner"
	Admin  Role = "admin"
	Member Role = "member"
)

type Principal struct {
	Subject  string
	TenantID string
	UserID   string
	Role     Role
	Scopes   map[string]bool
}
type Tenant struct {
	ID, Name, Status string
	CreatedAt        time.Time
}
type Domain struct {
	ID, TenantID, Domain, Status string
	CreatedAt                    time.Time
}
type APIKey struct {
	ID, Name, Prefix string
	Scopes           []string
	ExpiresAt        *time.Time
	RevokedAt        *time.Time
	CreatedAt        time.Time
	hash             [32]byte
	TenantID         string
}
type Challenge struct {
	Hash      [32]byte
	ExpiresAt time.Time
	Used      bool
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
type DomainCheck struct {
	DomainID  string        `json:"domain_id"`
	CheckedAt time.Time     `json:"checked_at"`
	Stale     bool          `json:"stale"`
	Results   []CheckResult `json:"results"`
}
type Usage struct {
	WindowStarted   time.Time `json:"window_started"`
	WindowSeconds   int       `json:"window_seconds"`
	MessagesUsed    int       `json:"messages_used"`
	MessageLimit    int       `json:"message_limit"`
	MaxMessageBytes int       `json:"max_message_bytes"`
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
		return Principal{Subject: subject, UserID: userID, TenantID: tenantID, Role: Owner, Scopes: map[string]bool{"domains:read": true, "domains:write": true, "keys:write": true, "delivery:read": true}}
	}
	for tenantID, role := range s.memberships[userID] {
		return Principal{Subject: subject, UserID: userID, TenantID: tenantID, Role: role, Scopes: map[string]bool{"domains:read": true, "domains:write": role != Member, "keys:write": role != Member, "delivery:read": true}}
	}
	return Principal{}
}

func (s *Store) Tenant(id string) (Tenant, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	t, ok := s.tenants[id]
	return t, ok
}
func (s *Store) Mailboxes(tenantID string) []Mailbox {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := []Mailbox{}
	for _, mailbox := range s.mailboxes {
		if mailbox.TenantID == tenantID {
			out = append(out, mailbox)
		}
	}
	return out
}
func (s *Store) Messages(tenantID, mailboxID string) ([]MessageSummary, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	mailbox, ok := s.mailboxes[mailboxID]
	if !ok || mailbox.TenantID != tenantID {
		return nil, ErrNotFound
	}
	return append([]MessageSummary(nil), s.messages[mailboxID]...), nil
}
func (s *Store) Message(tenantID, mailboxID, messageID string) (MessageSummary, error) {
	items, err := s.Messages(tenantID, mailboxID)
	if err != nil {
		return MessageSummary{}, err
	}
	for _, item := range items {
		if item.ID == messageID {
			return item, nil
		}
	}
	return MessageSummary{}, ErrNotFound
}
func (s *Store) Domains(tenantID string) []Domain {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := []Domain{}
	for _, d := range s.domains {
		if d.TenantID == tenantID {
			out = append(out, d)
		}
	}
	return out
}
func (s *Store) AddDomain(p Principal, raw string) (Domain, error) {
	if !p.Scopes["domains:write"] {
		return Domain{}, ErrForbidden
	}
	d, err := NormalizeDomain(raw)
	if err != nil {
		return Domain{}, err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, existing := range s.domains {
		if existing.TenantID == p.TenantID && existing.Domain == d {
			return Domain{}, ErrConflict
		}
	}
	id, err := newID()
	if err != nil {
		return Domain{}, err
	}
	item := Domain{ID: id, TenantID: p.TenantID, Domain: d, Status: "pending_verification", CreatedAt: time.Now().UTC()}
	s.domains[item.ID] = item
	s.auditLocked(p, "domain.registered", "domain", item.ID)
	return item, nil
}
func (s *Store) Challenge(p Principal, id string) (string, error) {
	if !p.Scopes["domains:write"] {
		return "", ErrForbidden
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	d, ok := s.domains[id]
	if !ok || d.TenantID != p.TenantID {
		return "", ErrNotFound
	}
	raw := make([]byte, 32)
	if _, err := rand.Read(raw); err != nil {
		return "", err
	}
	token := base64.RawURLEncoding.EncodeToString(raw)
	s.challenges[id] = Challenge{Hash: sha256.Sum256([]byte(token)), ExpiresAt: time.Now().UTC().Add(24 * time.Hour)}
	s.auditLocked(p, "domain.challenge_created", "domain", id)
	return token, nil
}
func (s *Store) CheckChallenge(p Principal, id string, resolver TXTResolver) error {
	if !p.Scopes["domains:write"] {
		return ErrForbidden
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	d, ok := s.domains[id]
	if !ok || d.TenantID != p.TenantID {
		return ErrNotFound
	}
	c, ok := s.challenges[id]
	if !ok || c.Used || time.Now().UTC().After(c.ExpiresAt) {
		return ErrVerificationPending
	}
	txt, err := resolver.LookupTXT(d.Domain)
	if err != nil {
		return ErrVerificationPending
	}
	expected := "smtp-platform-verification=" + hex.EncodeToString(c.Hash[:])
	for _, value := range txt {
		if value == expected {
			d.Status = "verified"
			s.domains[id] = d
			c.Used = true
			s.challenges[id] = c
			s.auditLocked(p, "domain.verified", "domain", id)
			return nil
		}
	}
	return ErrVerificationPending
}
func (s *Store) DisableDomain(p Principal, id string) error {
	if !p.Scopes["domains:write"] {
		return ErrForbidden
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	d, ok := s.domains[id]
	if !ok || d.TenantID != p.TenantID {
		return ErrNotFound
	}
	d.Status = "disabled"
	s.domains[id] = d
	s.auditLocked(p, "domain.disabled", "domain", id)
	return nil
}

type DNSResolver interface {
	TXT(domain string) ([]string, error)
	MX(domain string) ([]string, error)
}

type NetDNSResolver struct{}

func (NetDNSResolver) TXT(domain string) ([]string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	return net.DefaultResolver.LookupTXT(ctx, domain)
}

func (NetDNSResolver) MX(domain string) ([]string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	records, err := net.DefaultResolver.LookupMX(ctx, domain)
	if err != nil {
		return nil, err
	}
	out := make([]string, 0, len(records))
	for _, record := range records {
		out = append(out, strings.TrimSuffix(strings.ToLower(record.Host), "."))
	}
	return out, nil
}

func (s *Store) CheckDomain(p Principal, id string, resolver DNSResolver) (DomainCheck, error) {
	if !p.Scopes["domains:read"] {
		return DomainCheck{}, ErrForbidden
	}
	s.mu.RLock()
	d, ok := s.domains[id]
	s.mu.RUnlock()
	if !ok || d.TenantID != p.TenantID {
		return DomainCheck{}, ErrNotFound
	}
	now := time.Now().UTC()
	results := []CheckResult{}
	if d.Status == "verified" {
		results = append(results, CheckResult{"ownership", "pass", "Domain ownership is verified.", "No action required."})
	} else {
		results = append(results, CheckResult{"ownership", "fail", "Domain ownership is not verified.", "Complete the existing verification challenge."})
	}
	txt, txtErr := resolver.TXT(d.Domain)
	spf := []string{}
	for _, value := range txt {
		if strings.HasPrefix(strings.ToLower(strings.TrimSpace(value)), "v=spf1") {
			spf = append(spf, value)
		}
	}
	switch {
	case txtErr != nil:
		results = append(results, CheckResult{"spf", "pending", "DNS TXT lookup could not be completed.", "Retry after resolver or DNS propagation issues are resolved."})
	case len(spf) == 0:
		results = append(results, CheckResult{"spf", "warning", "No SPF record was observed.", "Publish one SPF record for your sending infrastructure."})
	case len(spf) > 1:
		results = append(results, CheckResult{"spf", "fail", "Multiple SPF records were observed.", "Combine SPF mechanisms into one record."})
	default:
		results = append(results, CheckResult{"spf", "pass", "One SPF record was observed.", "Review its mechanisms separately; this check does not evaluate authorization."})
	}
	dmarc, dmarcErr := resolver.TXT("_dmarc." + d.Domain)
	dmarcFound := false
	for _, value := range dmarc {
		if strings.HasPrefix(strings.ToLower(strings.TrimSpace(value)), "v=dmarc1") {
			dmarcFound = true
		}
	}
	if dmarcErr != nil {
		results = append(results, CheckResult{"dmarc", "pending", "DMARC lookup could not be completed.", "Retry after resolver or DNS propagation issues are resolved."})
	} else if !dmarcFound {
		results = append(results, CheckResult{"dmarc", "warning", "No DMARC policy record was observed.", "Publish a conservative DMARC record at _dmarc." + d.Domain + "."})
	} else {
		results = append(results, CheckResult{"dmarc", "pass", "A DMARC policy record was observed.", "Review policy semantics; this does not guarantee inbox placement."})
	}
	mx, mxErr := resolver.MX(d.Domain)
	if mxErr != nil || len(mx) == 0 {
		results = append(results, CheckResult{"mx", "warning", "No inbound MX configuration was observed.", "Configure MX only if inbound reception is required."})
	} else {
		results = append(results, CheckResult{"mx", "pass", "Inbound MX records were observed.", "No further action is required for this check."})
	}
	check := DomainCheck{id, now, false, results}
	s.mu.Lock()
	s.checks[id] = check
	s.mu.Unlock()
	return check, nil
}

func (s *Store) DomainCheck(p Principal, id string) (DomainCheck, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	d, ok := s.domains[id]
	if !ok || d.TenantID != p.TenantID {
		return DomainCheck{}, ErrNotFound
	}
	check, ok := s.checks[id]
	if !ok {
		return DomainCheck{DomainID: id, Stale: true}, nil
	}
	check.Stale = time.Since(check.CheckedAt) > 15*time.Minute
	return check, nil
}

func (s *Store) Usage(p Principal) Usage {
	s.mu.Lock()
	defer s.mu.Unlock()
	usage := s.usage[p.TenantID]
	now := time.Now().UTC()
	if usage.WindowStarted.IsZero() || now.Sub(usage.WindowStarted) >= time.Hour {
		usage = Usage{WindowStarted: now, WindowSeconds: 3600, MessageLimit: 1000, MaxMessageBytes: 10 * 1024 * 1024}
		s.usage[p.TenantID] = usage
	}
	return usage
}

func (s *Store) ConsumeMessage(p Principal) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	usage := s.usage[p.TenantID]
	now := time.Now().UTC()
	if usage.WindowStarted.IsZero() || now.Sub(usage.WindowStarted) >= time.Hour {
		usage = Usage{WindowStarted: now, WindowSeconds: 3600, MessageLimit: 1000, MaxMessageBytes: 10 * 1024 * 1024}
	}
	if usage.MessagesUsed >= usage.MessageLimit {
		return ErrRateLimited
	}
	usage.MessagesUsed++
	s.usage[p.TenantID] = usage
	return nil
}

func (s *Store) CreateKey(p Principal, name string, scopes []string, expiry *time.Time) (APIKey, string, error) {
	if !p.Scopes["keys:write"] {
		return APIKey{}, "", ErrForbidden
	}
	if strings.TrimSpace(name) == "" || len(name) > 120 || len(scopes) == 0 {
		return APIKey{}, "", ErrInvalid
	}
	for _, scope := range scopes {
		if scope != "domains:read" && scope != "domains:write" && scope != "delivery:read" {
			return APIKey{}, "", ErrInvalid
		}
	}
	raw := make([]byte, 32)
	if _, err := rand.Read(raw); err != nil {
		return APIKey{}, "", err
	}
	secret := "sp_" + base64.RawURLEncoding.EncodeToString(raw)
	prefix := secret[:11]
	hash := sha256.Sum256([]byte(secret))
	id, err := newID()
	if err != nil {
		return APIKey{}, "", err
	}
	item := APIKey{ID: id, Name: name, Prefix: prefix, Scopes: append([]string(nil), scopes...), ExpiresAt: expiry, CreatedAt: time.Now().UTC(), hash: hash, TenantID: p.TenantID}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.keys[item.ID] = item
	s.auditLocked(p, "api_key.created", "api_key", item.ID)
	return item, secret, nil
}
func (s *Store) Keys(p Principal) []APIKey {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := []APIKey{}
	for _, k := range s.keys {
		if k.TenantID == p.TenantID {
			copy := k
			copy.hash = [32]byte{}
			out = append(out, copy)
		}
	}
	return out
}
func (s *Store) RevokeKey(p Principal, id string) error {
	if !p.Scopes["keys:write"] {
		return ErrForbidden
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	k, ok := s.keys[id]
	if !ok || k.TenantID != p.TenantID {
		return ErrNotFound
	}
	now := time.Now().UTC()
	k.RevokedAt = &now
	s.keys[id] = k
	s.auditLocked(p, "api_key.revoked", "api_key", id)
	return nil
}
func (s *Store) AuthenticateKey(secret string) (Principal, error) {
	if !strings.HasPrefix(secret, "sp_") {
		return Principal{}, ErrUnauthorized
	}
	hash := sha256.Sum256([]byte(secret))
	s.mu.RLock()
	defer s.mu.RUnlock()
	for _, k := range s.keys {
		if subtle.ConstantTimeCompare(k.hash[:], hash[:]) == 1 && (k.RevokedAt == nil) && (k.ExpiresAt == nil || time.Now().UTC().Before(*k.ExpiresAt)) {
			roles := s.memberships[s.userForTenantLocked(k.TenantID)]
			for uid, role := range roles {
				_ = uid
				return Principal{
					Subject:  "api-key:" + k.ID,
					UserID:   s.userForTenantLocked(k.TenantID),
					TenantID: k.TenantID,
					Role:     role,
					Scopes:   scopeSet(k.Scopes),
				}, nil
			}
		}
	}
	return Principal{}, ErrUnauthorized
}
func (s *Store) userForTenantLocked(tenant string) string {
	for uid, ms := range s.memberships {
		if _, ok := ms[tenant]; ok {
			return uid
		}
	}
	return ""
}
func scopeSet(scopes []string) map[string]bool {
	m := map[string]bool{}
	for _, x := range scopes {
		m[x] = true
	}
	return m
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

type TXTResolver interface {
	LookupTXT(string) ([]string, error)
}
type NetTXTResolver struct{ Resolver *net.Resolver }

func (r NetTXTResolver) LookupTXT(domain string) ([]string, error) {
	res := r.Resolver
	if res == nil {
		res = net.DefaultResolver
	}
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	return res.LookupTXT(ctx, domain)
}

var (
	ErrUnauthorized        = errors.New("unauthorized")
	ErrForbidden           = errors.New("forbidden")
	ErrNotFound            = errors.New("not found")
	ErrConflict            = errors.New("conflict")
	ErrInvalid             = errors.New("invalid request")
	ErrVerificationPending = errors.New("verification pending")
	ErrRateLimited         = errors.New("rate limited")
)

func NormalizeDomain(raw string) (string, error) {
	d := strings.ToLower(strings.TrimSuffix(strings.TrimSpace(raw), "."))
	if len(d) == 0 || len(d) > 253 || strings.ContainsAny(d, " /\\") || !strings.Contains(d, ".") {
		return "", ErrInvalid
	}
	for _, r := range d {
		if !(unicode.IsLetter(r) || unicode.IsDigit(r) || r == '.' || r == '-') {
			return "", ErrInvalid
		}
	}
	for _, part := range strings.Split(d, ".") {
		if part == "" || len(part) > 63 || part[0] == '-' || part[len(part)-1] == '-' {
			return "", ErrInvalid
		}
	}
	return d, nil
}

func JSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}
func Error(w http.ResponseWriter, status int, code, message string) {
	JSON(w, status, map[string]any{
		"error": map[string]string{
			"code":       strings.ToUpper(code),
			"message":    message,
			"request_id": w.Header().Get(requestIDHeader),
		},
	})
}

func decodeJSON(r *http.Request, destination any) error {
	if contentType := r.Header.Get("Content-Type"); contentType != "" &&
		!strings.HasPrefix(strings.ToLower(contentType), "application/json") {
		return ErrInvalid
	}
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(destination); err != nil {
		return err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != nil {
		if err == io.EOF {
			return nil
		}
		return err
	}
	return ErrInvalid
}
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
