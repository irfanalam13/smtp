package smtpauth

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha1"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base32"
	"encoding/binary"
	"fmt"
	"net/url"
	"strconv"
	"strings"
	"time"
)

const (
	totpPeriodSeconds = 30
	totpDigits        = 6
	totpSecretBytes   = 20
	totpSkewSteps     = 1
	recoveryCodeCount = 10
	recoveryCodeBytes = 5
)

func NewTOTPSecret() (string, []byte, error) {
	raw := make([]byte, totpSecretBytes)
	if _, err := rand.Read(raw); err != nil {
		return "", nil, err
	}
	return base32.StdEncoding.WithPadding(base32.NoPadding).EncodeToString(raw), raw, nil
}

func DecodeTOTPSecret(encoded string) ([]byte, error) {
	encoded = strings.ToUpper(strings.TrimSpace(strings.ReplaceAll(encoded, " ", "")))
	raw, err := base32.StdEncoding.WithPadding(base32.NoPadding).DecodeString(encoded)
	if err != nil || len(raw) < 10 || len(raw) > 64 {
		return nil, ErrPasswordPolicy
	}
	return raw, nil
}

func TOTPURI(issuer, account, secret string) string {
	issuer = strings.TrimSpace(issuer)
	account = strings.TrimSpace(account)
	if issuer == "" {
		issuer = "SMTP Platform"
	}
	query := url.Values{}
	query.Set("secret", secret)
	query.Set("issuer", issuer)
	query.Set("algorithm", "SHA1")
	query.Set("digits", strconv.Itoa(totpDigits))
	query.Set("period", strconv.Itoa(totpPeriodSeconds))
	return fmt.Sprintf("otpauth://totp/%s?%s", url.PathEscape(issuer+":"+account), query.Encode())
}

func TOTPCode(secret []byte, at time.Time) string {
	counter := uint64(at.UTC().Unix() / totpPeriodSeconds)
	return hotp(secret, counter)
}

func VerifyTOTP(secret []byte, code string, at time.Time) bool {
	code = strings.TrimSpace(code)
	if len(code) != totpDigits {
		return false
	}
	for i := 0; i < len(code); i++ {
		if code[i] < '0' || code[i] > '9' {
			return false
		}
	}
	counter := int64(at.UTC().Unix() / totpPeriodSeconds)
	var matched int
	for delta := -totpSkewSteps; delta <= totpSkewSteps; delta++ {
		expected := hotp(secret, uint64(counter+int64(delta)))
		matched |= subtle.ConstantTimeCompare([]byte(expected), []byte(code))
	}
	return matched == 1
}

func hotp(secret []byte, counter uint64) string {
	var buf [8]byte
	binary.BigEndian.PutUint64(buf[:], counter)
	mac := hmac.New(sha1.New, secret)
	_, _ = mac.Write(buf[:])
	sum := mac.Sum(nil)
	offset := sum[len(sum)-1] & 0x0f
	truncated := binary.BigEndian.Uint32(sum[offset:offset+4]) & 0x7fffffff
	mod := uint32(1)
	for i := 0; i < totpDigits; i++ {
		mod *= 10
	}
	return fmt.Sprintf("%0*d", totpDigits, truncated%mod)
}

func NewRecoveryCodes() ([]string, [][sha256.Size]byte, error) {
	codes := make([]string, recoveryCodeCount)
	hashes := make([][sha256.Size]byte, recoveryCodeCount)
	for i := 0; i < recoveryCodeCount; i++ {
		var raw [recoveryCodeBytes]byte
		if _, err := rand.Read(raw[:]); err != nil {
			return nil, nil, err
		}
		codes[i] = strings.ToLower(fmt.Sprintf("%x", raw[:]))
		hashes[i] = HashSecret(codes[i])
	}
	return codes, hashes, nil
}

func NormalizeRecoveryCode(code string) string {
	return strings.ToLower(strings.TrimSpace(strings.ReplaceAll(code, "-", "")))
}
