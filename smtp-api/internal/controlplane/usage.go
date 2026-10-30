package controlplane

import (
	"time"
)

type Usage struct {
	WindowStarted   time.Time `json:"window_started"`
	WindowSeconds   int       `json:"window_seconds"`
	MessagesUsed    int       `json:"messages_used"`
	MessageLimit    int       `json:"message_limit"`
	MaxMessageBytes int       `json:"max_message_bytes"`
}

func (s *Store) Usage(p Principal) Usage {
	s.mu.Lock()
	defer s.mu.Unlock()
	usage := s.usage[p.TenantID]
	now := time.Now().UTC()
	if usage.WindowStarted.IsZero() || now.Sub(usage.WindowStarted) >= time.Hour {
		usage = Usage{WindowStarted: now, WindowSeconds: 3600, MessageLimit: 1000, MaxMessageBytes: 10 * 1024 * 1024}
		s.usage[p.TenantID] = usage
	}
	return usage
}

func (s *Store) ConsumeMessage(p Principal) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	usage := s.usage[p.TenantID]
	now := time.Now().UTC()
	if usage.WindowStarted.IsZero() || now.Sub(usage.WindowStarted) >= time.Hour {
		usage = Usage{WindowStarted: now, WindowSeconds: 3600, MessageLimit: 1000, MaxMessageBytes: 10 * 1024 * 1024}
	}
	if usage.MessagesUsed >= usage.MessageLimit {
		return ErrRateLimited
	}
	usage.MessagesUsed++
	s.usage[p.TenantID] = usage
	return nil
}
