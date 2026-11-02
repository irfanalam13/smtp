package controlplane

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/json"
	"errors"
	"sort"
	"strings"
	"time"
)

// JWTConfig controls short-lived HS256 bearer tokens for API clients. Browser
// sessions remain opaque, HttpOnly cookies so a bearer token never needs to be
// stored in browser storage.
type JWTConfig struct {
	SigningKey   []byte
	KeyID        string
	RotationKeys map[string][]byte
	Issuer       string
	Audience     string
	TTL          time.Duration
}

func (c JWTConfig) valid() bool {
	return len(c.SigningKey) >= 32 && c.Issuer != "" && c.TTL >= time.Minute && c.TTL <= 24*time.Hour
}

type jwtClaims struct {
	Subject  string   `json:"sub"`
	UserID   string   `json:"uid"`
	TenantID string   `json:"tid"`
	Role     Role     `json:"role"`
	Scopes   []string `json:"scp"`
	Issuer   string   `json:"iss"`
	Audience string   `json:"aud,omitempty"`
	IssuedAt int64    `json:"iat"`
	Expires  int64    `json:"exp"`
}

func (c JWTConfig) Issue(principal Principal) (string, time.Time, error) {
	if !c.valid() || !principal.durable || principal.Subject == "" || principal.UserID == "" || principal.TenantID == "" {
		return "", time.Time{}, ErrUnauthorized
	}
	now := time.Now().UTC()
	scopes := make([]string, 0, len(principal.Scopes))
	for scope, granted := range principal.Scopes {
		if granted {
			scopes = append(scopes, scope)
		}
	}
	sort.Strings(scopes)
	claims := jwtClaims{
		Subject:  principal.Subject,
		UserID:   principal.UserID,
		TenantID: principal.TenantID,
		Role:     principal.Role,
		Scopes:   scopes,
		Issuer:   c.Issuer,
		Audience: c.Audience,
		IssuedAt: now.Unix(),
		Expires:  now.Add(c.TTL).Unix(),
	}
	headerMap := map[string]string{"alg": "HS256", "typ": "JWT"}
	if c.KeyID != "" {
		headerMap["kid"] = c.KeyID
	}
	header, err := json.Marshal(headerMap)
	if err != nil {
		return "", time.Time{}, err
	}
	payload, err := json.Marshal(claims)
	if err != nil {
		return "", time.Time{}, err
	}
	encoded := base64.RawURLEncoding.EncodeToString(header) + "." + base64.RawURLEncoding.EncodeToString(payload)
	mac := hmac.New(sha256.New, c.SigningKey)
	_, _ = mac.Write([]byte(encoded))
	return encoded + "." + base64.RawURLEncoding.EncodeToString(mac.Sum(nil)), time.Unix(claims.Expires, 0).UTC(), nil
}

func (c JWTConfig) Verify(token string) (Principal, error) {
	if !c.valid() {
		return Principal{}, ErrUnauthorized
	}
	parts := strings.Split(token, ".")
	if len(parts) != 3 {
		return Principal{}, ErrUnauthorized
	}
	headerBytes, err := base64.RawURLEncoding.DecodeString(parts[0])
	if err != nil {
		return Principal{}, ErrUnauthorized
	}
	var header struct {
		Algorithm string `json:"alg"`
		Type      string `json:"typ"`
		KeyID     string `json:"kid"`
	}
	if json.Unmarshal(headerBytes, &header) != nil || header.Algorithm != "HS256" || header.Type != "JWT" {
		return Principal{}, ErrUnauthorized
	}

	verificationKey := c.SigningKey
	if header.KeyID != "" {
		if c.RotationKeys != nil {
			if k, ok := c.RotationKeys[header.KeyID]; ok && len(k) >= 32 {
				verificationKey = k
			} else if header.KeyID == c.KeyID {
				verificationKey = c.SigningKey
			} else {
				return Principal{}, ErrUnauthorized
			}
		} else if header.KeyID != c.KeyID {
			return Principal{}, ErrUnauthorized
		}
	}

	provided, err := base64.RawURLEncoding.DecodeString(parts[2])
	if err != nil {
		return Principal{}, ErrUnauthorized
	}
	mac := hmac.New(sha256.New, verificationKey)
	_, _ = mac.Write([]byte(parts[0] + "." + parts[1]))
	if !hmac.Equal(provided, mac.Sum(nil)) {
		return Principal{}, ErrUnauthorized
	}

	payload, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return Principal{}, ErrUnauthorized
	}
	var claims jwtClaims
	if json.Unmarshal(payload, &claims) != nil || claims.Issuer != c.Issuer || claims.Expires <= time.Now().UTC().Unix() || claims.IssuedAt > time.Now().UTC().Add(time.Minute).Unix() || !validUUID(claims.UserID) || !validUUID(claims.TenantID) || claims.Subject == "" || len(claims.Scopes) == 0 {
		return Principal{}, ErrUnauthorized
	}
	if c.Audience != "" && claims.Audience != c.Audience {
		return Principal{}, ErrUnauthorized
	}
	if claims.Role != Owner && claims.Role != Admin && claims.Role != Member {
		return Principal{}, ErrUnauthorized
	}
	for _, scope := range claims.Scopes {
		if strings.TrimSpace(scope) == "" {
			return Principal{}, errors.New("invalid JWT scope")
		}
	}
	scopes := make(map[string]bool, len(claims.Scopes))
	for _, scope := range claims.Scopes {
		scopes[scope] = true
	}
	return Principal{Subject: claims.Subject, UserID: claims.UserID, TenantID: claims.TenantID, Role: claims.Role, Scopes: scopes, durable: true}, nil
}
