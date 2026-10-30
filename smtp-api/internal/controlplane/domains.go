package controlplane

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"net"
	"strings"
	"time"
	"unicode"
)

type Challenge struct {
	Hash      [32]byte
	ExpiresAt time.Time
	Used      bool
}

type DomainCheck struct {
	DomainID  string        `json:"domain_id"`
	CheckedAt time.Time     `json:"checked_at"`
	Stale     bool          `json:"stale"`
	Results   []CheckResult `json:"results"`
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
	txt, err := resolver.LookupTXT("_smtp-platform-verification." + d.Domain)
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
			delete(s.checks, id)
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
	delete(s.checks, id)
	s.auditLocked(p, "domain.disabled", "domain", id)
	return nil
}

type DNSResolver interface {
	TXT(domain string) ([]string, error)
	MX(domain string) ([]string, error)
}

type NetDNSResolver struct{}

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
	check := inspectDomain(d, resolver)
	s.cacheDomainCheck(check)
	return check, nil
}

func inspectDomain(d Domain, resolver DNSResolver) DomainCheck {
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
	return DomainCheck{d.ID, now, false, results}
}

func (s *Store) DomainCheck(p Principal, id string) (DomainCheck, error) {
	s.mu.RLock()
	d, ok := s.domains[id]
	if !ok || d.TenantID != p.TenantID {
		s.mu.RUnlock()
		return DomainCheck{}, ErrNotFound
	}
	s.mu.RUnlock()
	check, found := s.cachedDomainCheck(id)
	if !found {
		return DomainCheck{DomainID: id, Stale: true}, nil
	}
	return check, nil
}

func (s *Store) cacheDomainCheck(check DomainCheck) {
	s.mu.Lock()
	s.checks[check.DomainID] = check
	s.mu.Unlock()
}

func (s *Store) invalidateDomainCheck(id string) {
	s.mu.Lock()
	delete(s.checks, id)
	s.mu.Unlock()
}

func (s *Store) cachedDomainCheck(id string) (DomainCheck, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	check, ok := s.checks[id]
	if !ok {
		return DomainCheck{}, false
	}
	check.Stale = time.Since(check.CheckedAt) > 15*time.Minute
	return check, true
}

type TXTResolver interface {
	LookupTXT(string) ([]string, error)
}

type NetTXTResolver struct{ Resolver *net.Resolver }

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

func (r NetTXTResolver) LookupTXT(domain string) ([]string, error) {
	res := r.Resolver
	if res == nil {
		res = net.DefaultResolver
	}
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	return res.LookupTXT(ctx, domain)
}

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
