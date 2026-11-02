package smtpauth

import (
	"bytes"
	"testing"
)

func TestEncryptDecryptSecret(t *testing.T) {
	key := []byte("a-very-secure-32-byte-master-key")
	plaintext := []byte("totp-secret-or-recovery-code-data")

	ciphertext, err := EncryptSecret(key, plaintext)
	if err != nil {
		t.Fatalf("EncryptSecret failed: %v", err)
	}
	if bytes.Equal(ciphertext, plaintext) {
		t.Fatal("ciphertext equals plaintext")
	}

	decrypted, err := DecryptSecret(key, ciphertext)
	if err != nil {
		t.Fatalf("DecryptSecret failed: %v", err)
	}
	if !bytes.Equal(decrypted, plaintext) {
		t.Fatalf("decrypted %q != plaintext %q", string(decrypted), string(plaintext))
	}

	// Tampered ciphertext
	corrupt := append([]byte(nil), ciphertext...)
	corrupt[len(corrupt)-1] ^= 0xff
	if _, err := DecryptSecret(key, corrupt); err == nil {
		t.Fatal("DecryptSecret should fail on tampered ciphertext")
	}

	// Wrong key
	wrongKey := []byte("wrong-key-that-is-at-least-16-bytes")
	if _, err := DecryptSecret(wrongKey, ciphertext); err == nil {
		t.Fatal("DecryptSecret should fail with wrong key")
	}

	// Key too short
	if _, err := EncryptSecret([]byte("short"), plaintext); err == nil {
		t.Fatal("EncryptSecret should fail with key too short")
	}
}
