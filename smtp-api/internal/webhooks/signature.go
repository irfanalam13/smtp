package webhooks

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"errors"
	"strconv"
	"strings"
	"time"
)

var errInvalidSigningInput = errors.New("invalid webhook signing input")

type SignatureHeaders struct {
	ID        string
	Timestamp string
	Signature string
}

func Sign(secret []byte, messageID string, timestamp time.Time, payload []byte) (SignatureHeaders, error) {
	if len(secret) < 32 || !validMessageID(messageID) || timestamp.Unix() < 0 {
		return SignatureHeaders{}, errInvalidSigningInput
	}

	unixTimestamp := strconv.FormatInt(timestamp.Unix(), 10)
	signedContent := make([]byte, 0, len(messageID)+len(unixTimestamp)+len(payload)+2)
	signedContent = append(signedContent, messageID...)
	signedContent = append(signedContent, '.')
	signedContent = append(signedContent, unixTimestamp...)
	signedContent = append(signedContent, '.')
	signedContent = append(signedContent, payload...)

	mac := hmac.New(sha256.New, secret)
	_, _ = mac.Write(signedContent)

	return SignatureHeaders{
		ID:        messageID,
		Timestamp: unixTimestamp,
		Signature: "v1," + base64.StdEncoding.EncodeToString(mac.Sum(nil)),
	}, nil
}

func Verify(secret []byte, headers SignatureHeaders, payload []byte, now time.Time, tolerance time.Duration) error {
	if len(secret) < 32 || !validMessageID(headers.ID) || tolerance < 0 {
		return errInvalidSigningInput
	}
	timestamp, err := strconv.ParseInt(headers.Timestamp, 10, 64)
	if err != nil || timestamp < 0 {
		return errInvalidSigningInput
	}
	signedAt := time.Unix(timestamp, 0)
	if tolerance > 0 && absDuration(now.Sub(signedAt)) > tolerance {
		return errors.New("webhook signature timestamp outside tolerance")
	}
	parts := strings.Split(headers.Signature, ",")
	if len(parts) != 2 || parts[0] != "v1" {
		return errors.New("unsupported webhook signature")
	}
	provided, err := base64.StdEncoding.DecodeString(parts[1])
	if err != nil {
		return errors.New("invalid webhook signature encoding")
	}
	expectedHeaders, err := Sign(secret, headers.ID, signedAt, payload)
	if err != nil || !hmac.Equal(provided, mustDecodeSignature(expectedHeaders.Signature)) {
		return errors.New("invalid webhook signature")
	}
	return nil
}

func mustDecodeSignature(value string) []byte {
	encoded := strings.TrimPrefix(value, "v1,")
	decoded, _ := base64.StdEncoding.DecodeString(encoded)
	return decoded
}

func absDuration(value time.Duration) time.Duration {
	if value < 0 {
		return -value
	}
	return value
}

func validMessageID(value string) bool {
	if len(value) == 0 || len(value) > 128 {
		return false
	}
	for _, char := range value {
		if !(char >= 'a' && char <= 'z' ||
			char >= 'A' && char <= 'Z' ||
			char >= '0' && char <= '9' ||
			char == '_' || char == '-') {
			return false
		}
	}
	return true
}
