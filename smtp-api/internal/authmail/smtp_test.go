package authmail

import "testing"

func TestNewSMTPConfiguration(t *testing.T) {
	tests := []struct {
		name    string
		config  SMTPConfig
		wantErr bool
	}{
		{
			name: "valid authenticated STARTTLS relay",
			config: SMTPConfig{
				Address: "smtp.example.test:587", Username: "mailer",
				Password: "secret", From: "SMTP Platform <auth@example.test>",
			},
		},
		{
			name: "valid implicit TLS relay",
			config: SMTPConfig{
				Address: "smtp.example.test:465", From: "auth@example.test",
			},
		},
		{
			name:    "missing sender",
			config:  SMTPConfig{Address: "smtp.example.test:587"},
			wantErr: true,
		},
		{
			name: "incomplete credentials",
			config: SMTPConfig{
				Address: "smtp.example.test:587", Username: "mailer",
				From: "auth@example.test",
			},
			wantErr: true,
		},
		{
			name:    "missing TLS port",
			config:  SMTPConfig{Address: "smtp.example.test", From: "auth@example.test"},
			wantErr: true,
		},
		{
			name:    "header injection",
			config:  SMTPConfig{Address: "smtp.example.test:587", From: "auth@example.test\r\nBcc: other@example.test"},
			wantErr: true,
		},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			_, err := NewSMTP(test.config)
			if (err != nil) != test.wantErr {
				t.Fatalf("NewSMTP() error = %v, want error %t", err, test.wantErr)
			}
		})
	}
}

func TestNewSMTPFromEnv(t *testing.T) {
	values := map[string]string{}
	mailer, err := NewSMTPFromEnv(func(key string) string { return values[key] })
	if err != nil || mailer != nil {
		t.Fatalf("empty SMTP settings = (%v, %v), want disabled", mailer, err)
	}
	values["AUTH_SMTP_ADDR"] = "smtp.example.test:587"
	if _, err := NewSMTPFromEnv(func(key string) string { return values[key] }); err == nil {
		t.Fatal("partial SMTP settings were accepted")
	}
}
