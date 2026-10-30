package webhooks

import (
	"testing"
	"time"
)

func TestSignUsesStandardWebhookHeadersAndSignsExactPayload(t *testing.T) {
	headers, err := Sign(
		[]byte("0123456789abcdef0123456789abcdef"),
		"evt_12345",
		time.Unix(1_700_000_000, 0),
		[]byte(`{"type":"delivery.sent"}`),
	)
	if err != nil {
		t.Fatal(err)
	}

	if headers.ID != "evt_12345" {
		t.Fatalf("unexpected webhook ID %q", headers.ID)
	}
	if headers.Timestamp != "1700000000" {
		t.Fatalf("unexpected timestamp %q", headers.Timestamp)
	}
	if headers.Signature != "v1,AeP5t1FoyWoGW7q64eKprp7BsrTXypWR6iMIUPoCSf8=" {
		t.Fatalf("unexpected signature %q", headers.Signature)
	}
}

func TestSignRejectsInvalidInputs(t *testing.T) {
	tests := []struct {
		name      string
		secret    []byte
		messageID string
		timestamp time.Time
	}{
		{
			name:      "short secret",
			secret:    []byte("short"),
			messageID: "evt_1",
			timestamp: time.Unix(1, 0),
		},
		{
			name:      "invalid message ID",
			secret:    []byte("0123456789abcdef0123456789abcdef"),
			messageID: "evt.1",
			timestamp: time.Unix(1, 0),
		},
		{
			name:      "pre-epoch timestamp",
			secret:    []byte("0123456789abcdef0123456789abcdef"),
			messageID: "evt_1",
			timestamp: time.Unix(-1, 0),
		},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if _, err := Sign(test.secret, test.messageID, test.timestamp, nil); err == nil {
				t.Fatal("expected invalid input to be rejected")
			}
		})
	}
}
