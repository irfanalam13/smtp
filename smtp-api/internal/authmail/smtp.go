package authmail

import (
	"context"
	"crypto/tls"
	"errors"
	"fmt"
	"io"
	"net"
	"net/mail"
	"net/smtp"
	"net/textproto"
	"net/url"
	"strconv"
	"strings"
	"time"
)

type Message struct {
	To      string
	Subject string
	Body    string
}

type Sender interface {
	Send(context.Context, Message) error
}

type SMTPConfig struct {
	Address  string
	Username string
	Password string
	From     string
}

type SMTP struct {
	address  string
	host     string
	port     int
	username string
	password string
	from     *mail.Address
}

func NewSMTPFromEnv(getenv func(string) string) (*SMTP, error) {
	config := SMTPConfig{
		Address:  strings.TrimSpace(getenv("AUTH_SMTP_ADDR")),
		Username: strings.TrimSpace(getenv("AUTH_SMTP_USERNAME")),
		Password: getenv("AUTH_SMTP_PASSWORD"),
		From:     strings.TrimSpace(getenv("AUTH_EMAIL_FROM")),
	}
	if config.Address == "" && config.Username == "" && config.Password == "" && config.From == "" {
		return nil, nil
	}
	return NewSMTP(config)
}

func NewSMTP(config SMTPConfig) (*SMTP, error) {
	if config.Address == "" || config.From == "" ||
		(config.Username == "") != (config.Password == "") {
		return nil, errors.New("AUTH_SMTP_ADDR and AUTH_EMAIL_FROM are required; SMTP username/password must be set together")
	}
	host, portText, err := net.SplitHostPort(config.Address)
	if err != nil || host == "" {
		return nil, errors.New("AUTH_SMTP_ADDR must be a hostname and port")
	}
	port, err := strconv.Atoi(portText)
	if err != nil || port < 1 || port > 65535 {
		return nil, errors.New("AUTH_SMTP_ADDR has an invalid port")
	}
	from, err := mail.ParseAddress(config.From)
	if err != nil || from.Address == "" || strings.ContainsAny(config.From, "\r\n") {
		return nil, errors.New("AUTH_EMAIL_FROM must be a valid mailbox address")
	}
	if parsed, parseErr := url.Parse("smtp://" + config.Address); parseErr != nil || parsed.Host != config.Address {
		return nil, errors.New("AUTH_SMTP_ADDR is invalid")
	}
	return &SMTP{
		address: config.Address, host: host, port: port,
		username: config.Username, password: config.Password, from: from,
	}, nil
}

func (client *SMTP) Send(ctx context.Context, message Message) error {
	if client == nil {
		return errors.New("authentication email sender is not configured")
	}
	to, err := parseMailbox(message.To)
	if err != nil {
		return errors.New("authentication message recipient is invalid")
	}
	if message.Subject == "" || strings.ContainsAny(message.Subject, "\r\n") ||
		message.Body == "" || strings.ContainsRune(message.Body, '\x00') {
		return errors.New("authentication message is invalid")
	}

	dialer := net.Dialer{Timeout: 10 * time.Second}
	connection, err := dialer.DialContext(ctx, "tcp", client.address)
	if err != nil {
		return fmt.Errorf("connect authentication SMTP relay: %w", err)
	}
	if deadline, ok := ctx.Deadline(); ok {
		_ = connection.SetDeadline(deadline)
	} else {
		_ = connection.SetDeadline(time.Now().Add(10 * time.Second))
	}
	if client.port == 465 {
		tlsConnection := tls.Client(connection, &tls.Config{
			MinVersion: tls.VersionTLS12,
			ServerName: client.host,
		})
		if err := tlsConnection.HandshakeContext(ctx); err != nil {
			_ = connection.Close()
			return fmt.Errorf("establish authentication SMTP TLS: %w", err)
		}
		connection = tlsConnection
	}

	smtpClient, err := smtp.NewClient(connection, client.host)
	if err != nil {
		_ = connection.Close()
		return fmt.Errorf("initialize authentication SMTP client: %w", err)
	}
	defer func() { _ = smtpClient.Close() }()

	if client.port != 465 {
		if supported, _ := smtpClient.Extension("STARTTLS"); !supported {
			return errors.New("authentication SMTP relay does not offer STARTTLS")
		}
		if err := smtpClient.StartTLS(&tls.Config{
			MinVersion: tls.VersionTLS12,
			ServerName: client.host,
		}); err != nil {
			return fmt.Errorf("establish authentication SMTP STARTTLS: %w", err)
		}
	}
	if client.username != "" {
		if err := smtpClient.Auth(smtp.PlainAuth("", client.username, client.password, client.host)); err != nil {
			return fmt.Errorf("authenticate to authentication SMTP relay: %w", err)
		}
	}
	if err := smtpClient.Mail(client.from.Address); err != nil {
		return fmt.Errorf("set authentication SMTP sender: %w", err)
	}
	if err := smtpClient.Rcpt(to.Address); err != nil {
		return fmt.Errorf("set authentication SMTP recipient: %w", err)
	}
	data, err := smtpClient.Data()
	if err != nil {
		return fmt.Errorf("begin authentication SMTP message: %w", err)
	}
	_, writeErr := io.WriteString(data, messageHeaders(client.from, to, message.Subject)+message.Body+"\r\n")
	closeErr := data.Close()
	if writeErr != nil {
		return fmt.Errorf("write authentication SMTP message: %w", writeErr)
	}
	if closeErr != nil {
		return fmt.Errorf("send authentication SMTP message: %w", closeErr)
	}
	if err := smtpClient.Quit(); err != nil {
		return fmt.Errorf("close authentication SMTP transaction: %w", err)
	}
	return nil
}

func parseMailbox(value string) (*mail.Address, error) {
	if strings.TrimSpace(value) != value || strings.ContainsAny(value, "\r\n") {
		return nil, errors.New("invalid mailbox")
	}
	address, err := mail.ParseAddress(value)
	if err != nil || address.Address == "" || address.Address != value {
		return nil, errors.New("invalid mailbox")
	}
	return address, nil
}

func messageHeaders(from, to *mail.Address, subject string) string {
	var headers textproto.MIMEHeader
	headers = make(textproto.MIMEHeader)
	headers.Set("From", from.String())
	headers.Set("To", to.String())
	headers.Set("Subject", subject)
	headers.Set("MIME-Version", "1.0")
	headers.Set("Content-Type", "text/plain; charset=utf-8")
	headers.Set("Content-Transfer-Encoding", "8bit")
	var output strings.Builder
	for _, name := range []string{
		"From", "To", "Subject", "MIME-Version",
		"Content-Type", "Content-Transfer-Encoding",
	} {
		fmt.Fprintf(&output, "%s: %s\r\n", name, headers.Get(name))
	}
	output.WriteString("\r\n")
	return output.String()
}
