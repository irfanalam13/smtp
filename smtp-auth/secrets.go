package smtpauth

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"io"
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

func EncryptSecret(key, plaintext []byte) ([]byte, error) {
	if len(key) < 16 {
		return nil, errors.New("encryption key too short")
	}
	derivedKey := sha256.Sum256(key)
	block, err := aes.NewCipher(derivedKey[:])
	if err != nil {
		return nil, err
	}
	aead, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	nonce := make([]byte, aead.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}
	return append(nonce, aead.Seal(nil, nonce, plaintext, nil)...), nil
}

func DecryptSecret(key, ciphertext []byte) ([]byte, error) {
	if len(key) < 16 {
		return nil, errors.New("encryption key too short")
	}
	derivedKey := sha256.Sum256(key)
	block, err := aes.NewCipher(derivedKey[:])
	if err != nil {
		return nil, err
	}
	aead, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	if len(ciphertext) < aead.NonceSize() {
		return nil, errors.New("ciphertext too short")
	}
	nonce, payload := ciphertext[:aead.NonceSize()], ciphertext[aead.NonceSize():]
	return aead.Open(nil, nonce, payload, nil)
}
