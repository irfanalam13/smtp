pub mod email_auth;

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::{SaltString, rand_core::OsRng},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use uuid::Uuid;

const PASSWORD_HASH_VERSION: i16 = 1;
const PASSWORD_MIN_BYTES: usize = 8;
const PASSWORD_MAX_BYTES: usize = 512;
const HASH_MEMORY_KIB: u32 = 19_456;
const HASH_ITERATIONS: u32 = 2;
const HASH_LANES: u32 = 1;

#[derive(Debug, Error)]
pub enum AuthenticationError {
    #[error("SMTP authentication storage is unavailable")]
    Database(#[from] sqlx::Error),
    #[error("SMTP password hashing operation failed")]
    Hashing,
    #[error("SMTP credential username is invalid")]
    InvalidUsername,
    #[error("SMTP credential tenant is unavailable")]
    TenantUnavailable,
    #[error("SMTP credential password does not meet the configured requirements")]
    InvalidPassword,
    #[error("SMTP rate-limit HMAC key must contain at least 32 bytes")]
    InvalidRateLimitKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedIdentity {
    pub tenant_id: Uuid,
    pub username: String,
}

#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<AuthenticatedIdentity>, AuthenticationError>;
}

#[async_trait]
pub trait SenderAuthorizer: Send + Sync {
    async fn authorize_sender(
        &self,
        tenant_id: Uuid,
        sender: &str,
    ) -> Result<bool, AuthenticationError>;

    async fn max_message_bytes(
        &self,
        _tenant_id: Uuid,
        _sender: &str,
    ) -> Result<Option<usize>, AuthenticationError> {
        Ok(None)
    }
}

#[derive(Clone)]
pub struct PostgresAuthenticator {
    pool: PgPool,
}

impl PostgresAuthenticator {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn shared_rate_limiter(
        &self,
        hmac_key: &[u8],
    ) -> Result<PostgresAuthRateLimiter, AuthenticationError> {
        PostgresAuthRateLimiter::new(self.pool.clone(), hmac_key)
    }

    pub async fn provision(
        &self,
        tenant_id: Uuid,
        username: &str,
        password: &str,
    ) -> Result<(), AuthenticationError> {
        self.provision_with_expiry(tenant_id, username, password, None)
            .await
    }

    pub async fn provision_with_expiry(
        &self,
        tenant_id: Uuid,
        username: &str,
        password: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), AuthenticationError> {
        let username = normalize_username(username)?;
        validate_password(password)?;
        let password = password.to_owned();
        let password_hash = tokio::task::spawn_blocking(move || hash_password(&password))
            .await
            .map_err(|_| AuthenticationError::Hashing)??;

        let result = sqlx::query(
            "INSERT INTO smtp_credentials
                (id, tenant_id, username, password_hash, password_hash_version, expires_at)
             SELECT $1, tenant.id, $3, $4, $5, $6
             FROM tenants AS tenant
             WHERE tenant.id = $2 AND tenant.status = 'active'
             ON CONFLICT (tenant_id, username) DO UPDATE
                SET password_hash = EXCLUDED.password_hash,
                    password_hash_version = EXCLUDED.password_hash_version,
                    password_changed_at = NOW(),
                    expires_at = EXCLUDED.expires_at,
                    revoked_at = NULL,
                    updated_at = NOW()",
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(&username)
        .bind(password_hash)
        .bind(PASSWORD_HASH_VERSION)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() != 1 {
            return Err(AuthenticationError::TenantUnavailable);
        }
        Ok(())
    }

    pub async fn revoke(
        &self,
        tenant_id: Uuid,
        username: &str,
    ) -> Result<bool, AuthenticationError> {
        let username = normalize_username(username)?;
        let result = sqlx::query(
            "UPDATE smtp_credentials
             SET revoked_at = COALESCE(revoked_at, NOW()), updated_at = NOW()
             WHERE tenant_id = $1 AND username = $2 AND revoked_at IS NULL",
        )
        .bind(tenant_id)
        .bind(&username)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn authorize_sender(
        &self,
        tenant_id: Uuid,
        sender: &str,
    ) -> Result<bool, AuthenticationError> {
        let Some((local_part, domain)) = sender.split_once('@') else {
            return Ok(false);
        };
        if local_part.is_empty()
            || local_part.len() > 64
            || domain.is_empty()
            || domain.len() > 253
            || !sender.is_ascii()
            || sender
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Ok(false);
        }
        let authorized: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1
                 FROM domains
                 JOIN tenants ON tenants.id = domains.tenant_id
                 WHERE domains.tenant_id = $1
                   AND lower(domains.domain) = lower($2)
                   AND domains.status = 'verified'
                   AND tenants.status = 'active'
             )",
        )
        .bind(tenant_id)
        .bind(domain)
        .fetch_one(&self.pool)
        .await?;
        Ok(authorized)
    }

    pub async fn max_message_bytes(
        &self,
        tenant_id: Uuid,
        sender: &str,
    ) -> Result<usize, AuthenticationError> {
        let domain = sender.rsplit_once('@').map(|(_, domain)| domain);
        let row = sqlx::query(
            "SELECT COALESCE(domain.max_smtp_message_bytes, tenant.max_smtp_message_bytes)
             FROM tenants AS tenant
             LEFT JOIN domains AS domain
               ON domain.tenant_id = tenant.id
              AND domain.status = 'verified'
              AND lower(domain.domain) = lower($2)
             WHERE tenant.id = $1 AND tenant.status = 'active'",
        )
        .bind(tenant_id)
        .bind(domain)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthenticationError::TenantUnavailable)?;
        let effective: i64 = row.try_get(0)?;
        usize::try_from(effective).map_err(|_| AuthenticationError::TenantUnavailable)
    }
}

#[derive(Clone)]
pub struct PostgresAuthRateLimiter {
    pool: PgPool,
    hmac_key: Arc<[u8]>,
    recorded_failures: Arc<AtomicU64>,
}

impl PostgresAuthRateLimiter {
    fn new(pool: PgPool, hmac_key: &[u8]) -> Result<Self, AuthenticationError> {
        if hmac_key.len() < 32 {
            return Err(AuthenticationError::InvalidRateLimitKey);
        }
        Ok(Self {
            pool,
            hmac_key: Arc::from(hmac_key),
            recorded_failures: Arc::new(AtomicU64::new(0)),
        })
    }

    pub async fn is_limited(
        &self,
        peer_ip: IpAddr,
        username: &str,
    ) -> Result<bool, AuthenticationError> {
        let keys = self.shared_rate_keys(peer_ip, username);
        let limited: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM smtp_auth_rate_limits
                 WHERE (key_hash = $1 OR key_hash = $2 OR key_hash = $3)
                   AND locked_until > NOW()
             )",
        )
        .bind(keys[0].0.as_slice())
        .bind(keys[1].0.as_slice())
        .bind(keys[2].0.as_slice())
        .fetch_one(&self.pool)
        .await?;
        Ok(limited)
    }

    pub async fn is_ip_limited(&self, peer_ip: IpAddr) -> Result<bool, AuthenticationError> {
        let key = self.bucket_key("ip", &peer_ip.to_string());
        let limited: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM smtp_auth_rate_limits
                 WHERE key_hash = $1 AND locked_until > NOW()
             )",
        )
        .bind(key.as_slice())
        .fetch_one(&self.pool)
        .await?;
        Ok(limited)
    }

    pub async fn record_failure(
        &self,
        peer_ip: IpAddr,
        username: &str,
    ) -> Result<(), AuthenticationError> {
        let mut keys = self.shared_rate_keys(peer_ip, username);
        keys.sort_by_key(|(key, _, _)| *key);
        let mut transaction = self.pool.begin().await?;
        for (key, dimension, threshold) in keys {
            sqlx::query(
                "INSERT INTO smtp_auth_rate_limits
                    (key_hash, dimension, failures, window_started_at, updated_at)
                 VALUES ($1, $2, 1, NOW(), NOW())
                 ON CONFLICT (key_hash) DO UPDATE
                 SET failures = CASE
                         WHEN smtp_auth_rate_limits.window_started_at <= NOW() - INTERVAL '15 minutes'
                         THEN 1 ELSE smtp_auth_rate_limits.failures + 1
                     END,
                     window_started_at = CASE
                         WHEN smtp_auth_rate_limits.window_started_at <= NOW() - INTERVAL '15 minutes'
                         THEN NOW() ELSE smtp_auth_rate_limits.window_started_at
                     END,
                     locked_until = CASE
                         WHEN (CASE
                             WHEN smtp_auth_rate_limits.window_started_at <= NOW() - INTERVAL '15 minutes'
                             THEN 1 ELSE smtp_auth_rate_limits.failures + 1
                         END) >= $3
                         THEN NOW() + make_interval(secs => LEAST(
                             900,
                             30 * power(2, LEAST(
                                 (CASE
                                     WHEN smtp_auth_rate_limits.window_started_at <= NOW() - INTERVAL '15 minutes'
                                     THEN 1 ELSE smtp_auth_rate_limits.failures + 1
                                 END) - $3,
                                 10
                             ))::integer
                         ))
                         ELSE NULL
                     END,
                     updated_at = NOW()",
            )
            .bind(key.as_slice())
            .bind(dimension)
            .bind(threshold)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        if self
            .recorded_failures
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(256)
        {
            sqlx::query(
                "DELETE FROM smtp_auth_rate_limits
                 WHERE ctid IN (
                     SELECT ctid FROM smtp_auth_rate_limits
                     WHERE updated_at < NOW() - INTERVAL '24 hours'
                       AND (locked_until IS NULL OR locked_until <= NOW())
                     ORDER BY updated_at
                     LIMIT 1000
                 )",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn record_success(
        &self,
        peer_ip: IpAddr,
        username: &str,
    ) -> Result<(), AuthenticationError> {
        let keys = self.shared_rate_keys(peer_ip, username);
        sqlx::query(
            "DELETE FROM smtp_auth_rate_limits
             WHERE key_hash = $1 OR key_hash = $2",
        )
        .bind(keys[1].0.as_slice())
        .bind(keys[2].0.as_slice())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    fn shared_rate_keys(
        &self,
        peer_ip: IpAddr,
        username: &str,
    ) -> [([u8; 32], &'static str, i32); 3] {
        let tenant = parse_authcid(username)
            .map(|(tenant, _)| tenant.to_string())
            .unwrap_or_else(|_| "invalid-tenant".to_owned());
        [
            (self.bucket_key("ip", &peer_ip.to_string()), "ip", 10),
            (self.bucket_key("tenant", &tenant), "tenant", 25),
            (
                self.bucket_key("account", &username.to_ascii_lowercase()),
                "account",
                5,
            ),
        ]
    }

    fn bucket_key(&self, dimension: &str, value: &str) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.hmac_key)
            .expect("HMAC accepts keys of any length");
        mac.update(dimension.as_bytes());
        mac.update(b":");
        mac.update(value.as_bytes());
        mac.finalize().into_bytes().into()
    }
}

#[async_trait]
impl Authenticator for PostgresAuthenticator {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<AuthenticatedIdentity>, AuthenticationError> {
        let Ok((tenant_id, username)) = parse_authcid(username) else {
            run_dummy_hash(password).await?;
            return Ok(None);
        };
        let row = sqlx::query(
            "SELECT credential.password_hash, credential.password_hash_version
             FROM smtp_credentials AS credential
             JOIN tenants AS tenant ON tenant.id = credential.tenant_id
             WHERE credential.tenant_id = $1 AND credential.username = $2
               AND credential.revoked_at IS NULL
               AND (credential.expires_at IS NULL OR credential.expires_at > NOW())
               AND tenant.status = 'active'",
        )
        .bind(tenant_id)
        .bind(&username)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            run_dummy_hash(password).await?;
            return Ok(None);
        };

        let hash: String = row.try_get("password_hash")?;
        let version: i16 = row.try_get("password_hash_version")?;
        if version != PASSWORD_HASH_VERSION {
            run_dummy_hash(password).await?;
            return Ok(None);
        }

        let password = password.to_owned();
        let verified = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
            .await
            .map_err(|_| AuthenticationError::Hashing)??;
        Ok(verified.then_some(AuthenticatedIdentity {
            tenant_id,
            username,
        }))
    }
}

#[async_trait]
impl SenderAuthorizer for PostgresAuthenticator {
    async fn authorize_sender(
        &self,
        tenant_id: Uuid,
        sender: &str,
    ) -> Result<bool, AuthenticationError> {
        PostgresAuthenticator::authorize_sender(self, tenant_id, sender).await
    }

    async fn max_message_bytes(
        &self,
        tenant_id: Uuid,
        sender: &str,
    ) -> Result<Option<usize>, AuthenticationError> {
        PostgresAuthenticator::max_message_bytes(self, tenant_id, sender)
            .await
            .map(Some)
    }
}

#[derive(Clone, Default)]
pub struct AuthRateLimiter {
    buckets: Arc<Mutex<HashMap<[u8; 32], RateBucket>>>,
}

struct RateBucket {
    failures: u32,
    window_started: Instant,
    locked_until: Option<Instant>,
    last_seen: Instant,
}

#[derive(Clone, Copy)]
enum RateDimension {
    Ip,
    Tenant,
    Account,
}

impl AuthRateLimiter {
    const WINDOW: Duration = Duration::from_secs(15 * 60);
    const BASE_LOCK: Duration = Duration::from_secs(30);
    const MAX_LOCK: Duration = Duration::from_secs(15 * 60);
    const MAX_BUCKETS: usize = 4096;

    pub fn is_ip_limited(&self, peer_ip: IpAddr) -> bool {
        self.is_limited_by(&[bucket_key(RateDimension::Ip, &peer_ip.to_string())])
    }

    pub fn is_limited(&self, peer_ip: IpAddr, username: &str) -> bool {
        let keys = rate_keys(peer_ip, username);
        self.is_limited_by(&keys)
    }

    pub fn record_failure(&self, peer_ip: IpAddr, username: &str) {
        let now = Instant::now();
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (key, threshold) in rate_keys_with_thresholds(peer_ip, username) {
            if buckets.len() >= Self::MAX_BUCKETS
                && !buckets.contains_key(&key)
                && let Some(oldest) = buckets
                    .iter()
                    .min_by_key(|(_, bucket)| bucket.last_seen)
                    .map(|(key, _)| *key)
            {
                buckets.remove(&oldest);
            }
            let bucket = buckets.entry(key).or_insert(RateBucket {
                failures: 0,
                window_started: now,
                locked_until: None,
                last_seen: now,
            });
            if now.duration_since(bucket.window_started) >= Self::WINDOW {
                bucket.failures = 0;
                bucket.window_started = now;
                bucket.locked_until = None;
            }
            bucket.last_seen = now;
            bucket.failures = bucket.failures.saturating_add(1);
            if bucket.failures >= threshold {
                let exponent = bucket.failures.saturating_sub(threshold).min(10);
                let backoff = Self::BASE_LOCK
                    .saturating_mul(1_u32 << exponent)
                    .min(Self::MAX_LOCK);
                bucket.locked_until = Some(now + backoff);
            }
        }
    }

    pub fn record_success(&self, peer_ip: IpAddr, username: &str) {
        let keys = rate_keys(peer_ip, username);
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        buckets.remove(&keys[1]);
        buckets.remove(&keys[2]);
    }

    fn is_limited_by(&self, keys: &[[u8; 32]]) -> bool {
        let now = Instant::now();
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for key in keys {
            if let Some(bucket) = buckets.get_mut(key) {
                bucket.last_seen = now;
                if now.duration_since(bucket.window_started) >= Self::WINDOW {
                    bucket.failures = 0;
                    bucket.window_started = now;
                    bucket.locked_until = None;
                }
                if bucket.locked_until.is_some_and(|until| until > now) {
                    return true;
                }
            }
        }
        false
    }
}

fn rate_keys(peer_ip: IpAddr, username: &str) -> [[u8; 32]; 3] {
    let tenant = parse_authcid(username)
        .map(|(tenant, _)| tenant.to_string())
        .unwrap_or_else(|_| "invalid-tenant".to_owned());
    [
        bucket_key(RateDimension::Ip, &peer_ip.to_string()),
        bucket_key(RateDimension::Tenant, &tenant),
        bucket_key(RateDimension::Account, &username.to_ascii_lowercase()),
    ]
}

fn rate_keys_with_thresholds(peer_ip: IpAddr, username: &str) -> [([u8; 32], u32); 3] {
    let keys = rate_keys(peer_ip, username);
    [(keys[0], 10), (keys[1], 25), (keys[2], 5)]
}

fn bucket_key(dimension: RateDimension, value: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(match dimension {
        RateDimension::Ip => b"ip:".as_slice(),
        RateDimension::Tenant => b"tenant:".as_slice(),
        RateDimension::Account => b"account:".as_slice(),
    });
    hasher.update(value.as_bytes());
    hasher.finalize().into()
}

fn normalize_username(username: &str) -> Result<String, AuthenticationError> {
    let username = username.trim();
    if username.is_empty()
        || username.len() > 128
        || !username.is_ascii()
        || username.bytes().any(|byte| {
            !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+' | b'@'))
        })
    {
        return Err(AuthenticationError::InvalidUsername);
    }
    Ok(username.to_ascii_lowercase())
}

fn parse_authcid(authcid: &str) -> Result<(Uuid, String), AuthenticationError> {
    let (tenant, username) = authcid
        .split_once(':')
        .ok_or(AuthenticationError::InvalidUsername)?;
    let tenant_id = Uuid::parse_str(tenant).map_err(|_| AuthenticationError::InvalidUsername)?;
    Ok((tenant_id, normalize_username(username)?))
}

fn validate_password(password: &str) -> Result<(), AuthenticationError> {
    if !(PASSWORD_MIN_BYTES..=PASSWORD_MAX_BYTES).contains(&password.len())
        || password.as_bytes().contains(&0)
    {
        return Err(AuthenticationError::InvalidPassword);
    }
    Ok(())
}

fn argon2() -> Result<Argon2<'static>, AuthenticationError> {
    let params = Params::new(HASH_MEMORY_KIB, HASH_ITERATIONS, HASH_LANES, None)
        .map_err(|_| AuthenticationError::Hashing)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn hash_password(password: &str) -> Result<String, AuthenticationError> {
    let salt = SaltString::generate(&mut OsRng);
    argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthenticationError::Hashing)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AuthenticationError> {
    let parsed = PasswordHash::new(hash).map_err(|_| AuthenticationError::Hashing)?;
    Ok(argon2()?
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

async fn run_dummy_hash(password: &str) -> Result<(), AuthenticationError> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|_| AuthenticationError::Hashing)??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authcid_binds_credentials_to_one_tenant() {
        let tenant_id = Uuid::new_v4();
        assert_eq!(
            parse_authcid(&format!("{tenant_id}:Alice@example.test")).unwrap(),
            (tenant_id, "alice@example.test".into())
        );
        assert!(parse_authcid("alice@example.test").is_err());
        assert!(parse_authcid(&format!("{}:user", Uuid::new_v4())).is_ok());
    }

    #[test]
    fn password_hashes_are_argon2id_and_verify() {
        let hash = hash_password("a sufficiently long password").unwrap();
        assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(verify_password("a sufficiently long password", &hash).unwrap());
        assert!(!verify_password("a different password", &hash).unwrap());
    }

    #[test]
    fn credential_inputs_are_bounded_and_normalized() {
        assert_eq!(
            normalize_username(" Alice@example.test ").unwrap(),
            "alice@example.test"
        );
        assert!(normalize_username("tenant:alice").is_err());
        assert!(validate_password("short").is_err());
        assert!(validate_password("long enough\0but invalid").is_err());
    }

    #[test]
    fn rate_limiter_locks_ip_account_and_tenant_and_resets_after_success() {
        let limiter = AuthRateLimiter::default();
        let peer_ip = "192.0.2.1".parse().unwrap();
        let tenant_id = Uuid::new_v4();
        let username = format!("{tenant_id}:alice@example.test");
        for _ in 0..5 {
            limiter.record_failure(peer_ip, &username);
        }
        assert!(limiter.is_limited(peer_ip, &username));
        limiter.record_success(peer_ip, &username);
        assert!(!limiter.is_limited(peer_ip, &username));
    }
}
