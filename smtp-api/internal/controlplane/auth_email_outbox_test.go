package controlplane

import (
	"testing"

	"example.com/smtp-platform/smtp-api/internal/authmail"
)

func TestAuthEmailOutboxEncryptsSecretBearingBody(t *testing.T) {
	key := []byte("test-auth-email-encryption-key-with-sufficient-length")
	message := authmail.Message{
		To: "user@example.test", Subject: "Verify",
		Body: "https://example.test/verify?token=one-time-secret",
	}
	ciphertext, err := encryptAuthEmail(key, message)
	if err != nil {
		t.Fatal(err)
	}
	if string(ciphertext) == message.Body {
		t.Fatal("outbox ciphertext contains plaintext message body")
	}
	decoded, err := decryptAuthEmail(key, ciphertext)
	if err != nil {
		t.Fatal(err)
	}
	if decoded != message {
		t.Fatalf("decoded message = %#v, want %#v", decoded, message)
	}
	if _, err := decryptAuthEmail([]byte("wrong-key-with-sufficient-length"), ciphertext); err == nil {
		t.Fatal("wrong encryption key decrypted outbox message")
	}
}
