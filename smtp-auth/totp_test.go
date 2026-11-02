package smtpauth

import (
	"testing"
	"time"
)

func TestTOTPCodeIsStableWithinTheSamePeriod(t *testing.T) {
	secret := []byte("12345678901234567890")
	at := time.Unix(59, 0).UTC()
	code := TOTPCode(secret, at)
	if len(code) != 6 {
		t.Fatalf("TOTPCode length = %d", len(code))
	}
	if !VerifyTOTP(secret, code, at) {
		t.Fatal("VerifyTOTP rejected the current period code")
	}
	if !VerifyTOTP(secret, code, at.Add(20*time.Second)) {
		t.Fatal("VerifyTOTP rejected a code within the allowed skew window")
	}
	if VerifyTOTP(secret, code, at.Add(2*time.Minute)) {
		t.Fatal("VerifyTOTP accepted a code outside the skew window")
	}
	if VerifyTOTP(secret, "abcdef", at) || VerifyTOTP(secret, "12345", at) {
		t.Fatal("VerifyTOTP accepted a non-numeric or short code")
	}
}

func TestNewTOTPSecretRoundTrip(t *testing.T) {
	encoded, raw, err := NewTOTPSecret()
	if err != nil || encoded == "" || len(raw) != 20 {
		t.Fatalf("NewTOTPSecret() = (%q, %d, %v)", encoded, len(raw), err)
	}
	decoded, err := DecodeTOTPSecret(encoded)
	if err != nil {
		t.Fatal(err)
	}
	if string(decoded) != string(raw) {
		t.Fatal("decoded TOTP secret did not match the generated secret")
	}
	uri := TOTPURI("SMTP Platform", "user@example.test", encoded)
	if uri == "" {
		t.Fatal("TOTPURI returned an empty value")
	}
}

func TestRecoveryCodesAreUniqueAndHashable(t *testing.T) {
	codes, hashes, err := NewRecoveryCodes()
	if err != nil || len(codes) != 10 || len(hashes) != 10 {
		t.Fatalf("NewRecoveryCodes() = (%d, %d, %v)", len(codes), len(hashes), err)
	}
	seen := map[string]struct{}{}
	for i, code := range codes {
		if NormalizeRecoveryCode(code) != code || len(code) != 10 {
			t.Fatalf("recovery code %q is not normalized", code)
		}
		if _, ok := seen[code]; ok {
			t.Fatalf("duplicate recovery code %q", code)
		}
		seen[code] = struct{}{}
		if HashSecret(code) != hashes[i] {
			t.Fatal("recovery code hash mismatch")
		}
	}
}
