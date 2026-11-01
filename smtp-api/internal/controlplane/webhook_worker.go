package controlplane

import (
	"bytes"
	"context"
	"errors"
	"log/slog"
	"net/http"
	"strconv"
	"time"

	"example.com/smtp-platform/smtp-api/internal/webhooks"
	"github.com/jackc/pgx/v5"
)

const webhookMaxAttempts = 10
const webhookLease = 2 * time.Minute

type webhookDelivery struct {
	ID             string
	SubscriptionID string
	Endpoint       string
	Secret         []byte
	Payload        []byte
	AttemptCount   int
}

func (repository *PostgreSQLRepository) claimWebhookDelivery(ctx context.Context) (webhookDelivery, error) {
	var item webhookDelivery
	tx, err := repository.pool.Begin(ctx)
	if err != nil {
		return item, err
	}
	defer func() { _ = tx.Rollback(ctx) }()
	err = tx.QueryRow(ctx, `
		SELECT delivery.id::text, subscription.id::text, subscription.endpoint,
		       subscription.secret, delivery.payload, delivery.attempt_count
		FROM webhook_deliveries AS delivery
		JOIN webhook_subscriptions AS subscription ON subscription.id = delivery.subscription_id
		WHERE (delivery.state IN ('pending', 'retry')
		       OR (delivery.state = 'delivering' AND delivery.lease_until < NOW()))
		  AND delivery.next_attempt_at <= NOW()
		  AND subscription.active = TRUE
		ORDER BY delivery.next_attempt_at, delivery.created_at
		FOR UPDATE OF delivery SKIP LOCKED
		LIMIT 1
	`).Scan(&item.ID, &item.SubscriptionID, &item.Endpoint, &item.Secret, &item.Payload, &item.AttemptCount)
	if errors.Is(err, pgx.ErrNoRows) {
		return webhookDelivery{}, nil
	}
	if err != nil {
		return item, err
	}
	if _, err := tx.Exec(ctx, `
		UPDATE webhook_deliveries
		SET state = 'delivering', attempt_count = attempt_count + 1,
		    lease_until = NOW() + $2::interval
		WHERE id = $1
	`, item.ID, strconv.FormatInt(int64(webhookLease/time.Second), 10)+" seconds"); err != nil {
		return item, err
	}
	if err := tx.Commit(ctx); err != nil {
		return item, err
	}
	return item, nil
}

func (repository *PostgreSQLRepository) finishWebhookDelivery(ctx context.Context, item webhookDelivery, deliveryErr error) error {
	if deliveryErr == nil {
		_, err := repository.pool.Exec(ctx, `
			UPDATE webhook_deliveries
			SET state = 'delivered', delivered_at = NOW(),
			    lease_until = NULL, last_error = NULL
			WHERE id = $1 AND state = 'delivering'
		`, item.ID)
		return err
	}
	nextState := "retry"
	if item.AttemptCount+1 >= webhookMaxAttempts {
		nextState = "dead_letter"
	}
	delay := time.Duration(1<<min(item.AttemptCount, 8)) * time.Minute
	_, err := repository.pool.Exec(ctx, `
		UPDATE webhook_deliveries
		SET state = $2, next_attempt_at = NOW() + $3::interval,
		    lease_until = NULL, last_error = $4
		WHERE id = $1 AND state = 'delivering'
	`, item.ID, nextState, strconv.FormatInt(int64(delay/time.Second), 10)+" seconds", deliveryErr.Error())
	return err
}

func (repository *PostgreSQLRepository) RunWebhookDelivery(ctx context.Context) {
	client := &http.Client{Timeout: 15 * time.Second}
	for {
		item, err := repository.claimWebhookDelivery(ctx)
		if err != nil {
			slog.Error("claim webhook delivery failed", "error", err)
		} else if item.ID != "" {
			headers, signErr := webhooks.Sign(item.Secret, item.ID, time.Now().UTC(), item.Payload)
			if signErr == nil {
				request, requestErr := http.NewRequestWithContext(ctx, http.MethodPost, item.Endpoint, bytes.NewReader(item.Payload))
				if requestErr == nil {
					request.Header.Set("Content-Type", "application/json")
					request.Header.Set("Webhook-ID", headers.ID)
					request.Header.Set("Webhook-Timestamp", headers.Timestamp)
					request.Header.Set("Webhook-Signature", headers.Signature)
					response, doErr := client.Do(request)
					if doErr == nil {
						if response.StatusCode < 200 || response.StatusCode >= 300 {
							doErr = errors.New("webhook endpoint returned HTTP " + strconv.Itoa(response.StatusCode))
						}
						_ = response.Body.Close()
					}
					signErr = doErr
				} else {
					signErr = requestErr
				}
			}
			if err := repository.finishWebhookDelivery(ctx, item, signErr); err != nil {
				slog.Error("finish webhook delivery failed", "error", err, "delivery_id", item.ID)
			}
		}
		select {
		case <-ctx.Done():
			return
		case <-time.After(500 * time.Millisecond):
		}
	}
}
