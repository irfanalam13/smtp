package controlplane

import (
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"sort"
	"strings"
	"time"
)

func (s *Store) CreateKey(p Principal, name string, scopes []string, expiry *time.Time) (APIKey, string, error) {
	if !p.Scopes["keys:write"] {
		return APIKey{}, "", ErrForbidden
	}
	name = strings.TrimSpace(name)
	if name == "" || len(name) > 120 || len(scopes) == 0 {
		return APIKey{}, "", ErrInvalid
	}
	for _, scope := range scopes {
		if scope != "domains:read" && scope != "domains:write" && scope != "delivery:read" && scope != "delivery:write" {
			return APIKey{}, "", ErrInvalid
		}
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, existing := range s.keys {
		if existing.TenantID == p.TenantID && existing.Name == name {
			return APIKey{}, "", ErrConflict
		}
	}
	for {
		raw := make([]byte, 32)
		if _, err := rand.Read(raw); err != nil {
			return APIKey{}, "", err
		}
		secret := "sp_" + base64.RawURLEncoding.EncodeToString(raw)
		prefix := secret[:apiKeyPrefixLength]
		if s.hasKeyPrefixLocked(prefix) {
			continue
		}
		hash := sha256.Sum256([]byte(secret))
		id, err := newID()
		if err != nil {
			return APIKey{}, "", err
		}
		item := APIKey{ID: id, Name: name, Prefix: prefix, Scopes: append([]string(nil), scopes...), ExpiresAt: expiry, CreatedAt: time.Now().UTC(), hash: hash, TenantID: p.TenantID}
		s.keys[item.ID] = item
		s.auditLocked(p, "api_key.created", "api_key", item.ID)
		return item, secret, nil
	}
}

func (s *Store) hasKeyPrefixLocked(prefix string) bool {
	for _, existing := range s.keys {
		if existing.Prefix == prefix {
			return true
		}
	}
	return false
}

func (s *Store) Keys(p Principal, limit int) []APIKey {
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
	sort.Slice(out, func(i, j int) bool {
		if out[i].CreatedAt.Equal(out[j].CreatedAt) {
			return out[i].ID > out[j].ID
		}
		return out[i].CreatedAt.After(out[j].CreatedAt)
	})
	if limit >= 0 && len(out) > limit {
		out = out[:limit]
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
