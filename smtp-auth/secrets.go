package smtpauth

import (
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"strings"
)

const secretBytes = 32

func NewSecret() (string, [sha256.Size]byte, error) {
	var raw [secretBytes]byte
	if _, err := rand.Read(raw[:]); err != nil {
		return "", [sha256.Size]byte{}, err
	}
	secret := base64.RawURLEncoding.EncodeToString(raw[:])
	return secret, sha256.Sum256([]byte(secret)), nil
}

func HashSecret(secret string) [sha256.Size]byte {
	return sha256.Sum256([]byte(secret))
}

func VerifyCSRF(cookieValue, headerValue string, expected [sha256.Size]byte) bool {
	if cookieValue == "" || headerValue == "" || len(cookieValue) > 128 ||
		len(headerValue) > 128 || strings.ContainsAny(cookieValue+headerValue, "\r\n") {
		return false
	}
	if subtle.ConstantTimeCompare([]byte(cookieValue), []byte(headerValue)) != 1 {
		return false
	}
	actual := HashSecret(headerValue)
	return subtle.ConstantTimeCompare(actual[:], expected[:]) == 1
}
