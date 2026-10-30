package controlplane

import "sort"

func (s *Store) Mailboxes(tenantID string, limit int) []Mailbox {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := []Mailbox{}
	for _, mailbox := range s.mailboxes {
		if mailbox.TenantID == tenantID {
			out = append(out, mailbox)
		}
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].CreatedAt.Equal(out[j].CreatedAt) {
			return out[i].ID < out[j].ID
		}
		return out[i].CreatedAt.Before(out[j].CreatedAt)
	})
	if len(out) > limit {
		out = out[:limit]
	}
	return out
}

func (s *Store) Messages(tenantID, mailboxID string) ([]MessageSummary, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	mailbox, ok := s.mailboxes[mailboxID]
	if !ok || mailbox.TenantID != tenantID {
		return nil, ErrNotFound
	}
	return append([]MessageSummary(nil), s.messages[mailboxID]...), nil
}

func (s *Store) Message(tenantID, mailboxID, messageID string) (MessageSummary, error) {
	items, err := s.Messages(tenantID, mailboxID)
	if err != nil {
		return MessageSummary{}, err
	}
	for _, item := range items {
		if item.ID == messageID {
			return item, nil
		}
	}
	return MessageSummary{}, ErrNotFound
}
