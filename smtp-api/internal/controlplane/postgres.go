package controlplane

import (
	"context"
	"errors"
	"fmt"
	"net"
	"strconv"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
	"github.com/jackc/pgx/v5/pgxpool"
)

const (
	defaultDatabaseMaxConnections = int32(10)
	maximumDatabaseConnections    = int32(100)
	apiKeyPrefixLength            = 11
)

type PostgreSQLRepository struct {
	pool *pgxpool.Pool
}

func NewPostgreSQLRepository(pool *pgxpool.Pool) *PostgreSQLRepository {
	return &PostgreSQLRepository{pool: pool}
}

func PostgreSQLPoolConfigFromEnv(getenv func(string) string) (*pgxpool.Config, error) {
	databaseURL := strings.TrimSpace(getenv("DATABASE_URL"))
	if databaseURL == "" {
		return nil, errors.New("DATABASE_URL is required")
	}

	maxConnections := defaultDatabaseMaxConnections
	if raw := strings.TrimSpace(getenv("API_DATABASE_MAX_CONNECTIONS")); raw != "" {
		parsed, err := strconv.ParseInt(raw, 10, 32)
		if err != nil || parsed < 1 || parsed > int64(maximumDatabaseConnections) {
			return nil, fmt.Errorf("API_DATABASE_MAX_CONNECTIONS must be between 1 and %d", maximumDatabaseConnections)
		}
		maxConnections = int32(parsed)
	}

	config, err := pgxpool.ParseConfig(databaseURL)
	if err != nil {
		return nil, errors.New("DATABASE_URL is invalid")
	}
	config.MaxConns = maxConnections
	config.MinConns = 0
	config.ConnConfig.ConnectTimeout = 5 * time.Second

	environment := strings.ToLower(strings.TrimSpace(getenv("APP_ENVIRONMENT")))
	if (environment == "production" || environment == "prod") &&
		!isLocalDatabaseHost(config.ConnConfig.Host) &&
		(config.ConnConfig.TLSConfig == nil || hasUnencryptedFallback(config.ConnConfig.Fallbacks)) {
		return nil, errors.New("remote PostgreSQL connections require TLS in production")
	}
	return config, nil
}

func hasUnencryptedFallback(fallbacks []*pgconn.FallbackConfig) bool {
	for _, fallback := range fallbacks {
		if fallback.TLSConfig == nil {
			return true
		}
	}
	return false
}

func isLocalDatabaseHost(host string) bool {
	if host == "" || strings.HasPrefix(host, "/") {
		return true
	}
	if strings.EqualFold(host, "localhost") {
		return true
	}
	ip := net.ParseIP(host)
	return ip != nil && ip.IsLoopback()
}

func (repository *PostgreSQLRepository) Ping(ctx context.Context) error {
	return repository.pool.Ping(ctx)
}

func (repository *PostgreSQLRepository) Close() {
	repository.pool.Close()
}

func (repository *PostgreSQLRepository) Tenant(
	ctx context.Context,
	principal Principal,
) (Tenant, error) {
	if !principal.durable || !validUUID(principal.TenantID) {
		return Tenant{}, ErrUnauthorized
	}
	var tenant Tenant
	if principal.UserID == "" {
		err := repository.pool.QueryRow(ctx, `
			SELECT id::text, name, status, created_at
			FROM tenants
			WHERE id = $1 AND status = 'active'
		`, principal.TenantID).Scan(
			&tenant.ID, &tenant.Name, &tenant.Status, &tenant.CreatedAt,
		)
		if errors.Is(err, pgx.ErrNoRows) {
			return Tenant{}, ErrNotFound
		}
		if err != nil {
			return Tenant{}, fmt.Errorf("resolve authenticated tenant: %w", err)
		}
		return tenant, nil
	}
	err := repository.pool.QueryRow(ctx, `
		SELECT tenant.id::text, tenant.name, tenant.status, tenant.created_at
		FROM tenants AS tenant
		JOIN users AS account
		  ON account.id = $2 AND account.status = 'active'
		JOIN tenant_memberships AS membership
		  ON membership.tenant_id = tenant.id AND membership.user_id = account.id
		WHERE tenant.id = $1 AND tenant.status = 'active'
	`, principal.TenantID, principal.UserID).Scan(
		&tenant.ID, &tenant.Name, &tenant.Status, &tenant.CreatedAt,
	)
	if errors.Is(err, pgx.ErrNoRows) {
		return Tenant{}, ErrNotFound
	}
	if err != nil {
		return Tenant{}, fmt.Errorf("resolve authenticated tenant: %w", err)
	}
	return tenant, nil
}
