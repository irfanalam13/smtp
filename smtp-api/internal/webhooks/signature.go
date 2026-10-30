package webhooks

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"errors"
	"strconv"
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
