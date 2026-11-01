use std::{env, net::SocketAddr, time::Duration};

use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use smtp_core::MailboxStore;
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

const MAX_MESSAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = MAX_MESSAGE_BYTES + 16 * 1024;
const MAX_TOKEN_AGE_SECONDS: i64 = 30;
type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct AppState {
    pool: PgPool,
    mailbox_store: MailboxStore,
    signing_key: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InternalPrincipal {
    tenant_id: String,
    user_id: String,
    role: String,
    scopes: Vec<String>,
    issued_at: i64,
    method: String,
    uri: String,
    body_sha256: String,
    #[serde(default)]
    idempotency_key: Option<String>,
}

mod inbound_projection;
mod operations;
mod retention;

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url =
        env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required for mailbox service")?;
    let signing_key = env::var("MAILBOX_SERVICE_HMAC_KEY")
        .map_err(|_| "MAILBOX_SERVICE_HMAC_KEY is required for mailbox service")?;
    let signing_key = hex::decode(signing_key)
        .map_err(|_| "MAILBOX_SERVICE_HMAC_KEY must be 64 hexadecimal characters")?;
    if signing_key.len() < 32 {
        return Err("MAILBOX_SERVICE_HMAC_KEY must contain at least 32 bytes".into());
    }

    let pool = PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await?;
    let address: SocketAddr = env::var("MAILBOX_SERVICE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8081".into())
        .parse()?;
    if !address.ip().is_loopback() {
        return Err("mailbox service only supports loopback binding without configured TLS".into());
    }
    let mailbox_store = MailboxStore::from_pool(pool.clone(), MAX_MESSAGE_BYTES);
    let state = AppState {
        pool,
        mailbox_store,
        signing_key,
    };
    tokio::spawn(inbound_projection::run(state.pool.clone()));
    tokio::spawn(retention::run(state.pool.clone()));
    let app = Router::new()
        .route("/health/live", any(|| async { StatusCode::NO_CONTENT }))
        .route("/health/ready", any(ready))
        .fallback(any(handle_request))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "mailbox service listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn ready(State(state): State<AppState>) -> StatusCode {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(error) => {
            tracing::error!(%error, "mailbox service readiness check failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to listen for shutdown signal");
    }
}

async fn handle_request(State(state): State<AppState>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_REQUEST_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return error_response(StatusCode::PAYLOAD_TOO_LARGE, "request_too_large"),
    };
    let Some(token) = parts
        .headers
        .get("x-mailbox-principal")
        .and_then(|value| value.to_str().ok())
    else {
        return error_response(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    let principal = match verify_principal(
        &state.signing_key,
        token,
        parts.method.as_str(),
        &parts.uri.to_string(),
        &bytes,
    ) {
        Some(principal) => principal,
        None => return error_response(StatusCode::UNAUTHORIZED, "unauthorized"),
    };
    let path = parts.uri.path();
    let query = parts.uri.query().unwrap_or_default();
    if let Some(response) = operations::dispatch(
        &state,
        &principal,
        parts.method.as_str(),
        path,
        query,
        &bytes,
        principal.idempotency_key.as_deref(),
    )
    .await
    {
        return response;
    }
    match (parts.method.as_str(), path) {
        ("GET", "/v1/mailboxes") if has_scope(&principal, "mailbox:read") => {
            list_mailboxes(&state.pool, &principal, query).await
        }
        ("GET", path)
            if has_scope(&principal, "mailbox:read")
                && operations::mailbox_id(path, "").is_some() =>
        {
            operations::get_mailbox(
                &state,
                &principal,
                operations::mailbox_id(path, "").unwrap(),
            )
            .await
        }
        ("GET", path)
            if has_scope(&principal, "mailbox:read")
                && path.ends_with("/messages")
                && mailbox_path_id(path, "/messages").is_some() =>
        {
            list_messages(
                &state.pool,
                &principal,
                mailbox_path_id(path, "/messages").unwrap(),
                query,
            )
            .await
        }
        ("GET", path)
            if has_scope(&principal, "mailbox:read")
                && path.ends_with("/content")
                && message_path_ids(path, "/content").is_some() =>
        {
            let (mailbox_id, message_id) = message_path_ids(path, "/content").unwrap();
            read_message_content(
                &state.pool,
                &principal,
                mailbox_id,
                message_id,
                "content",
                parts
                    .headers
                    .get(header::RANGE)
                    .and_then(|value| value.to_str().ok()),
                parts
                    .headers
                    .get(header::IF_RANGE)
                    .and_then(|value| value.to_str().ok()),
            )
            .await
        }
        ("GET", path)
            if has_scope(&principal, "mailbox:read")
                && path.ends_with("/headers")
                && message_path_ids(path, "/headers").is_some() =>
        {
            let (mailbox_id, message_id) = message_path_ids(path, "/headers").unwrap();
            read_message_content(
                &state.pool,
                &principal,
                mailbox_id,
                message_id,
                "headers",
                parts
                    .headers
                    .get(header::RANGE)
                    .and_then(|value| value.to_str().ok()),
                parts
                    .headers
                    .get(header::IF_RANGE)
                    .and_then(|value| value.to_str().ok()),
            )
            .await
        }
        ("GET", path)
            if has_scope(&principal, "mailbox:read")
                && path.ends_with("/body")
                && message_path_ids(path, "/body").is_some() =>
        {
            let (mailbox_id, message_id) = message_path_ids(path, "/body").unwrap();
            read_message_content(
                &state.pool,
                &principal,
                mailbox_id,
                message_id,
                "body",
                parts
                    .headers
                    .get(header::RANGE)
                    .and_then(|value| value.to_str().ok()),
                parts
                    .headers
                    .get(header::IF_RANGE)
                    .and_then(|value| value.to_str().ok()),
            )
            .await
        }
        ("GET", path)
            if has_scope(&principal, "mailbox:read")
                && path.matches('/').count() == 5
                && path.contains("/messages/") =>
        {
            let Some((mailbox_id, message_id)) = message_path_ids(path, "") else {
                return error_response(StatusCode::NOT_FOUND, "not_found");
            };
            get_message(&state.pool, &principal, mailbox_id, message_id).await
        }
        _ => error_response(StatusCode::NOT_FOUND, "not_found"),
    }
}

fn verify_principal(
    key: &[u8],
    token: &str,
    method: &str,
    uri: &str,
    body: &[u8],
) -> Option<InternalPrincipal> {
    let (payload, signature) = token.split_once('.')?;
    let payload_bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let mut mac = HmacSha256::new_from_slice(key).ok()?;
    mac.update(&payload_bytes);
    mac.verify_slice(&signature).ok()?;
    let principal: InternalPrincipal = serde_json::from_slice(&payload_bytes).ok()?;
    let now = Utc::now().timestamp();
    let age = now.checked_sub(principal.issued_at)?;
    let body_hash = hex::encode(Sha256::digest(body));
    if !(-5..=MAX_TOKEN_AGE_SECONDS).contains(&age)
        || principal.method != method
        || principal.uri != uri
        || principal.body_sha256 != body_hash
        || Uuid::parse_str(&principal.tenant_id).is_err()
        || Uuid::parse_str(&principal.user_id).is_err()
    {
        return None;
    }
    Some(principal)
}

fn has_scope(principal: &InternalPrincipal, scope: &str) -> bool {
    principal.scopes.iter().any(|value| value == scope)
}

fn mailbox_path_id<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    let mailbox_id = path.strip_prefix("/v1/mailboxes/")?.strip_suffix(suffix)?;
    valid_uuid(mailbox_id).then_some(mailbox_id)
}

fn message_path_ids<'a>(path: &'a str, suffix: &str) -> Option<(&'a str, &'a str)> {
    let path = path.strip_suffix(suffix)?;
    let (mailbox_id, message_id) = path
        .strip_prefix("/v1/mailboxes/")?
        .split_once("/messages/")?;
    (valid_uuid(mailbox_id) && valid_uuid(message_id)).then_some((mailbox_id, message_id))
}

fn valid_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok()
}

#[derive(Deserialize)]
struct PageQuery {
    limit: Option<i64>,
    page_token: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct MailboxPageCursor {
    tenant_id: Uuid,
    created_at: DateTime<Utc>,
    id: Uuid,
}

#[derive(Deserialize, Serialize)]
struct MessagePageCursor {
    mailbox_id: Uuid,
    internal_date: DateTime<Utc>,
    id: Uuid,
}

#[allow(clippy::result_large_err)]
fn page_query(query: &str) -> Result<PageQuery, Response> {
    let query: PageQuery = serde_urlencoded::from_str(query)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid_query"))?;
    if query.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
        return Err(error_response(StatusCode::BAD_REQUEST, "invalid_query"));
    }
    Ok(query)
}

#[allow(clippy::result_large_err)]
fn decode_cursor<T: for<'de> Deserialize<'de>>(value: Option<&str>) -> Result<Option<T>, Response> {
    value
        .map(|value| {
            let bytes = URL_SAFE_NO_PAD
                .decode(value)
                .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid_query"))?;
            serde_json::from_slice(&bytes)
                .map_err(|_| error_response(StatusCode::BAD_REQUEST, "invalid_query"))
        })
        .transpose()
}

#[allow(clippy::result_large_err)]
fn encode_cursor<T: Serialize>(value: &T) -> Result<String, Response> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal_error"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

async fn list_mailboxes(pool: &PgPool, principal: &InternalPrincipal, query: &str) -> Response {
    let query = match page_query(query) {
        Ok(query) => query,
        Err(response) => return response,
    };
    let cursor = match decode_cursor::<MailboxPageCursor>(query.page_token.as_deref()) {
        Ok(cursor) => cursor,
        Err(response) => return response,
    };
    if cursor
        .as_ref()
        .is_some_and(|cursor| Uuid::parse_str(&principal.tenant_id).ok() != Some(cursor.tenant_id))
    {
        return error_response(StatusCode::BAD_REQUEST, "invalid_query");
    }
    let limit = query.limit.unwrap_or(50);
    let rows = match sqlx::query(
        "SELECT mailbox.id, mailbox.tenant_id,
                mailbox.local_part || '@' || domain.domain AS address,
                mailbox.status, mailbox.display_name, mailbox.mailbox_kind,
                mailbox.quota_bytes, mailbox.used_bytes, mailbox.created_at
         FROM mailboxes AS mailbox
         JOIN domains AS domain
           ON domain.id = mailbox.domain_id AND domain.tenant_id = mailbox.tenant_id
         JOIN tenants AS tenant ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
         JOIN users AS account ON account.id = $2 AND account.status = 'active'
         JOIN tenant_memberships AS membership
           ON membership.tenant_id = mailbox.tenant_id AND membership.user_id = account.id
         WHERE mailbox.tenant_id = $1 AND mailbox.status = 'active'
           AND mailbox.deleted_at IS NULL
           AND (mailbox.owner_user_id = account.id
                OR membership.role IN ('owner', 'admin')
                OR EXISTS (
               SELECT 1 FROM mailbox_permissions AS permission
               WHERE permission.tenant_id = mailbox.tenant_id
                 AND permission.mailbox_id = mailbox.id
                 AND permission.user_id = account.id
                 AND permission.permission IN ('read', 'read_write', 'manage')
           ))
           AND ($3::TIMESTAMPTZ IS NULL OR (mailbox.created_at, mailbox.id) > ($3, $4))
         ORDER BY mailbox.created_at, mailbox.id LIMIT $5",
    )
    .bind(Uuid::parse_str(&principal.tenant_id).unwrap())
    .bind(Uuid::parse_str(&principal.user_id).unwrap())
    .bind(cursor.as_ref().map(|cursor| cursor.created_at))
    .bind(cursor.as_ref().map(|cursor| cursor.id))
    .bind(limit + 1)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!(%error, "list mailboxes");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    };
    let has_more = rows.len() > limit as usize;
    let items = rows
        .iter()
        .take(limit as usize)
        .map(|row| {
            serde_json::json!({
                "id": row.get::<Uuid, _>("id"),
                "tenant_id": row.get::<Uuid, _>("tenant_id"),
                "address": row.get::<String, _>("address"),
                "status": row.get::<String, _>("status"),
                "display_name": row.get::<String, _>("display_name"),
                "kind": row.get::<String, _>("mailbox_kind"),
                "quota_bytes": row.get::<i64, _>("quota_bytes"),
                "used_bytes": row.get::<i64, _>("used_bytes"),
                "created_at": row.get::<DateTime<Utc>, _>("created_at"),
            })
        })
        .collect::<Vec<_>>();
    let next_page = if has_more {
        let row = &rows[limit as usize - 1];
        match encode_cursor(&MailboxPageCursor {
            tenant_id: Uuid::parse_str(&principal.tenant_id).unwrap(),
            created_at: row.get("created_at"),
            id: row.get("id"),
        }) {
            Ok(cursor) => Some(cursor),
            Err(response) => return response,
        }
    } else {
        None
    };
    json_response(
        StatusCode::OK,
        serde_json::json!({"items": items, "next_page": next_page}),
    )
}

async fn list_messages(
    pool: &PgPool,
    principal: &InternalPrincipal,
    mailbox_id: &str,
    query: &str,
) -> Response {
    let Ok(mailbox_id) = Uuid::parse_str(mailbox_id) else {
        return error_response(StatusCode::NOT_FOUND, "not_found");
    };
    let page_query = match page_query(query) {
        Ok(query) => query,
        Err(response) => return response,
    };
    let cursor = match decode_cursor::<MessagePageCursor>(page_query.page_token.as_deref()) {
        Ok(cursor) => cursor,
        Err(response) => return response,
    };
    if cursor
        .as_ref()
        .is_some_and(|cursor| cursor.mailbox_id != mailbox_id)
    {
        return error_response(StatusCode::BAD_REQUEST, "invalid_query");
    }
    let limit = page_query.limit.unwrap_or(50);
    match mailbox_allowed(pool, principal, mailbox_id, false).await {
        Ok(true) => {}
        Ok(false) => return error_response(StatusCode::NOT_FOUND, "not_found"),
        Err(error) => {
            tracing::error!(%error, "authorize mailbox message listing");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    }
    let rows = match sqlx::query(
        "SELECT id, mailbox_id, folder_id, uid, sender_address, recipients,
                subject, raw_size, internal_date, flags, parsed_metadata
         FROM mailbox_messages
         WHERE mailbox_id = $1 AND deleted_at IS NULL
           AND ($3::TIMESTAMPTZ IS NULL OR (internal_date, id) < ($3, $4))
         ORDER BY internal_date DESC, id DESC LIMIT $2",
    )
    .bind(mailbox_id)
    .bind(limit + 1)
    .bind(cursor.as_ref().map(|cursor| cursor.internal_date))
    .bind(cursor.as_ref().map(|cursor| cursor.id))
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!(%error, "list mailbox messages");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    };
    let has_more = rows.len() > limit as usize;
    let items = rows
        .iter()
        .take(limit as usize)
        .map(message_summary)
        .collect::<Vec<_>>();
    let next_page = if has_more {
        let row = &rows[limit as usize - 1];
        match encode_cursor(&MessagePageCursor {
            mailbox_id,
            internal_date: row.get("internal_date"),
            id: row.get("id"),
        }) {
            Ok(cursor) => Some(cursor),
            Err(response) => return response,
        }
    } else {
        None
    };
    json_response(
        StatusCode::OK,
        serde_json::json!({"items": items, "next_page": next_page}),
    )
}

async fn get_message(
    pool: &PgPool,
    principal: &InternalPrincipal,
    mailbox_id: &str,
    message_id: &str,
) -> Response {
    let (Ok(mailbox_id), Ok(message_id)) =
        (Uuid::parse_str(mailbox_id), Uuid::parse_str(message_id))
    else {
        return error_response(StatusCode::NOT_FOUND, "not_found");
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            tracing::error!(%error, "begin mailbox message read");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    };
    match operations::authorize(&mut transaction, principal, mailbox_id, "read").await {
        Ok(true) => {}
        Ok(false) => return error_response(StatusCode::NOT_FOUND, "not_found"),
        Err(error) => {
            tracing::error!(%error, "authorize mailbox message read");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    }
    let row = match message_row(&mut transaction, mailbox_id, message_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "not_found"),
        Err(error) => {
            tracing::error!(%error, "get mailbox message");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    };
    if let Err(error) = transaction.commit().await {
        tracing::error!(%error, "commit mailbox message read");
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
    }
    json_response(StatusCode::OK, message_summary(&row))
}

fn message_summary(row: &sqlx::postgres::PgRow) -> serde_json::Value {
    serde_json::json!({
        "id": row.get::<Uuid, _>("id"),
        "mailbox_id": row.get::<Uuid, _>("mailbox_id"),
        "folder_id": row.get::<Uuid, _>("folder_id"),
        "uid": row.get::<i64, _>("uid"),
        "sender": row.get::<Option<String>, _>("sender_address").unwrap_or_default(),
        "recipients": row.get::<Vec<String>, _>("recipients"),
        "subject": row.get::<Option<String>, _>("subject").unwrap_or_default(),
        "size": row.get::<i64, _>("raw_size"),
        "received_at": row.get::<DateTime<Utc>, _>("internal_date"),
        "flags": row.get::<Vec<String>, _>("flags"),
        "metadata": row.get::<serde_json::Value, _>("parsed_metadata"),
    })
}

async fn message_row(
    transaction: &mut Transaction<'_, Postgres>,
    mailbox_id: Uuid,
    message_id: Uuid,
) -> Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    sqlx::query(
        "SELECT id, mailbox_id, folder_id, uid, sender_address, recipients,
                subject, raw_size, internal_date, flags, parsed_metadata, raw_message
         FROM mailbox_messages
         WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(mailbox_id)
    .bind(message_id)
    .fetch_optional(&mut **transaction)
    .await
}

async fn mailbox_allowed(
    pool: &PgPool,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    write: bool,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM mailboxes AS mailbox
             JOIN tenants AS tenant
               ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
             JOIN users AS account ON account.id = $3 AND account.status = 'active'
             JOIN tenant_memberships AS membership
               ON membership.tenant_id = mailbox.tenant_id AND membership.user_id = account.id
             WHERE mailbox.id = $1 AND mailbox.tenant_id = $2
               AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
               AND (mailbox.owner_user_id = account.id
                    OR membership.role IN ('owner', 'admin')
                    OR EXISTS (
                   SELECT 1 FROM mailbox_permissions AS permission
                   WHERE permission.tenant_id = mailbox.tenant_id
                     AND permission.mailbox_id = mailbox.id
                     AND permission.user_id = account.id
                     AND permission.permission = ANY($4)
               ))
         )",
    )
    .bind(mailbox_id)
    .bind(Uuid::parse_str(&principal.tenant_id).unwrap())
    .bind(Uuid::parse_str(&principal.user_id).unwrap())
    .bind(if write {
        vec!["read_write", "manage"]
    } else {
        vec!["read", "read_write", "manage"]
    })
    .fetch_one(pool)
    .await
}

async fn read_message_content(
    pool: &PgPool,
    principal: &InternalPrincipal,
    mailbox_id: &str,
    message_id: &str,
    part: &str,
    range_header: Option<&str>,
    if_range: Option<&str>,
) -> Response {
    let (Ok(mailbox_id), Ok(message_id)) =
        (Uuid::parse_str(mailbox_id), Uuid::parse_str(message_id))
    else {
        return error_response(StatusCode::NOT_FOUND, "not_found");
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            tracing::error!(%error, "begin mailbox content read");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    };
    match operations::authorize(&mut transaction, principal, mailbox_id, "read").await {
        Ok(true) => {}
        Ok(false) => return error_response(StatusCode::NOT_FOUND, "not_found"),
        Err(error) => {
            tracing::error!(%error, "authorize mailbox content read");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    }
    let row = match message_row(&mut transaction, mailbox_id, message_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "not_found"),
        Err(error) => {
            tracing::error!(%error, "load mailbox message content");
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
        }
    };
    let raw: Vec<u8> = row.get("raw_message");
    let body_start = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| (position + 4, position))
        .or_else(|| {
            raw.windows(2)
                .position(|window| window == b"\n\n")
                .map(|position| (position + 2, position))
        });
    let Some((body_start, header_end)) = body_start else {
        return error_response(StatusCode::UNPROCESSABLE_ENTITY, "message_format_invalid");
    };
    let content = match part {
        "headers" => &raw[..header_end],
        "body" => &raw[body_start..],
        _ => raw.as_slice(),
    };
    let representation_len = content.len();
    let etag = format!("\"{}\"", hex::encode(Sha256::digest(content)));
    let apply_range = range_header.is_some() && if_range.is_none_or(|value| value == etag);
    let (content, partial, content_range) = if apply_range {
        match parse_byte_range(range_header.unwrap_or_default(), content.len()) {
            Some((start, end)) => (&content[start..=end], true, Some((start, end))),
            None => {
                let mut response =
                    error_response(StatusCode::RANGE_NOT_SATISFIABLE, "invalid_range");
                response.headers_mut().insert(
                    header::CONTENT_RANGE,
                    HeaderValue::from_str(&format!("bytes */{representation_len}"))
                        .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
                );
                return response;
            }
        }
    } else {
        (content, false, None)
    };
    let action = format!("mailbox.message.{part}.read");
    if let Err(error) = sqlx::query(
        "INSERT INTO audit_events (tenant_id, actor_user_id, action, target_type, target_id, metadata)
         VALUES ($1, $2, $3, 'mailbox_message', $4, $5)",
    )
    .bind(Uuid::parse_str(&principal.tenant_id).unwrap())
    .bind(Uuid::parse_str(&principal.user_id).unwrap())
    .bind(&action)
    .bind(message_id.to_string())
    .bind(serde_json::json!({"mailbox_id": mailbox_id, "content_bytes": content.len()}))
    .execute(&mut *transaction)
    .await
    {
        tracing::error!(%error, "persist mailbox content access audit");
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
    }
    if let Err(error) = transaction.commit().await {
        tracing::error!(%error, "commit mailbox content access audit");
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable");
    }
    let mut response = Response::new(Body::from(content.to_vec()));
    *response.status_mut() = if partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(if part == "content" {
            "message/rfc822"
        } else {
            "text/plain; charset=utf-8"
        }),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&content.len().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response.headers_mut().insert(
        header::ETAG,
        HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("\"\"")),
    );
    if let Some((start, end)) = content_range {
        response.headers_mut().insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{representation_len}"))
                .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
        );
    }
    response
}

fn parse_byte_range(value: &str, content_len: usize) -> Option<(usize, usize)> {
    if content_len == 0 {
        return None;
    }
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix_len = end.parse::<usize>().ok()?.min(content_len);
        if suffix_len == 0 {
            return None;
        }
        return Some((content_len - suffix_len, content_len - 1));
    }
    let start = start.parse::<usize>().ok()?;
    let end = if end.is_empty() {
        content_len - 1
    } else {
        end.parse::<usize>().ok()?.min(content_len - 1)
    };
    (start <= end && start < content_len).then_some((start, end))
}

fn json_response(status: StatusCode, value: serde_json::Value) -> Response {
    let body = serde_json::to_vec(&value).unwrap_or_else(|error| {
        tracing::error!(%error, "encode mailbox API response");
        b"{\"error\":{\"code\":\"internal_error\",\"message\":\"internal error\"}}".to_vec()
    });
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

fn error_response(status: StatusCode, code: &'static str) -> Response {
    let message = match code {
        "unauthorized" => "authentication required",
        "not_found" => "mailbox or message not found",
        "invalid_query" => "query parameters are invalid",
        "request_too_large" => "request exceeds the configured size limit",
        "message_format_invalid" => "stored message has no header/body boundary",
        _ => "requested data is temporarily unavailable",
    };
    (
        status,
        axum::Json(ErrorBody {
            error: ErrorDetail { code, message },
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use serde_json::Value;
    use sqlx::postgres::PgPoolOptions;

    fn signed_principal(key: &[u8], principal: &InternalPrincipal) -> String {
        let payload = serde_json::to_vec(principal).expect("serialize principal");
        let mut mac = HmacSha256::new_from_slice(key).expect("initialize HMAC");
        mac.update(&payload);
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        )
    }

    #[test]
    fn principal_signature_binds_the_append_idempotency_key() {
        let key = [42; 32];
        let principal = InternalPrincipal {
            tenant_id: "a1234567-1234-1234-1234-123456789abc".into(),
            user_id: "b1234567-1234-1234-1234-123456789abc".into(),
            role: "owner".into(),
            scopes: vec!["mailbox:write".into()],
            issued_at: Utc::now().timestamp(),
            method: "POST".into(),
            uri: "/v1/mailboxes/123/messages".into(),
            body_sha256: hex::encode(Sha256::digest(b"message bytes")),
            idempotency_key: Some("append-001".into()),
        };
        let token = signed_principal(&key, &principal);

        let verified = verify_principal(
            &key,
            &token,
            "POST",
            "/v1/mailboxes/123/messages",
            b"message bytes",
        )
        .expect("verify signed request");
        assert_eq!(verified.idempotency_key.as_deref(), Some("append-001"));
        assert!(
            verify_principal(
                &key,
                &token,
                "POST",
                "/v1/mailboxes/123/messages",
                b"different message bytes",
            )
            .is_none()
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn signed_request(
        state: &AppState,
        tenant_id: Uuid,
        user_id: Uuid,
        method: &str,
        uri: &str,
        body: &[u8],
        scopes: &[&str],
        idempotency_key: Option<&str>,
    ) -> Request<Body> {
        let principal = InternalPrincipal {
            tenant_id: tenant_id.to_string(),
            user_id: user_id.to_string(),
            role: "owner".into(),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
            issued_at: Utc::now().timestamp(),
            method: method.into(),
            uri: uri.into(),
            body_sha256: hex::encode(Sha256::digest(body)),
            idempotency_key: idempotency_key.map(str::to_string),
        };
        Request::builder()
            .method(method)
            .uri(uri)
            .header(
                "X-Mailbox-Principal",
                signed_principal(&state.signing_key, &principal),
            )
            .body(Body::from(body.to_vec()))
            .expect("build signed test request")
    }

    #[tokio::test]
    #[ignore = "requires TEST_DATABASE_URL and migrations"]
    async fn mailbox_http_api_provisions_and_appends_idempotently() {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .expect("set TEST_DATABASE_URL to an isolated disposable PostgreSQL database");
        assert!(
            database_url.contains("test"),
            "TEST_DATABASE_URL must identify an isolated test database"
        );
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&database_url)
            .await
            .expect("connect isolated test database");
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let domain_id = Uuid::new_v4();
        sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, 'mailbox service integration')")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("insert tenant");
        sqlx::query("INSERT INTO users (id, external_subject) VALUES ($1, $2)")
            .bind(user_id)
            .bind(format!("mailbox-service-{user_id}"))
            .execute(&pool)
            .await
            .expect("insert user");
        sqlx::query(
            "INSERT INTO tenant_memberships (tenant_id, user_id, role)
             VALUES ($1, $2, 'owner')",
        )
        .bind(tenant_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("insert owner membership");
        sqlx::query(
            "INSERT INTO domains (id, tenant_id, domain, status)
             VALUES ($1, $2, $3, 'verified')",
        )
        .bind(domain_id)
        .bind(tenant_id)
        .bind(format!("{}.mailbox-test.invalid", domain_id.simple()))
        .execute(&pool)
        .await
        .expect("insert verified domain");

        let key = vec![42; 32];
        let state = AppState {
            mailbox_store: MailboxStore::from_pool(pool.clone(), MAX_MESSAGE_BYTES),
            pool: pool.clone(),
            signing_key: key,
        };
        let create_body = serde_json::to_vec(&serde_json::json!({
            "domain_id": domain_id,
            "local_part": format!("box-{}", tenant_id.simple()),
            "display_name": "Integration mailbox",
            "kind": "user",
            "quota_bytes": 4096,
        }))
        .expect("encode mailbox request");
        let response = handle_request(
            State(state.clone()),
            signed_request(
                &state,
                tenant_id,
                user_id,
                "POST",
                "/v1/mailboxes",
                &create_body,
                &["mailbox:manage"],
                None,
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let created: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 4096)
                .await
                .expect("read provision response"),
        )
        .expect("decode mailbox response");
        let mailbox_id = created["id"].as_str().expect("mailbox ID");
        let folders_response = handle_request(
            State(state.clone()),
            signed_request(
                &state,
                tenant_id,
                user_id,
                "GET",
                &format!("/v1/mailboxes/{mailbox_id}/folders"),
                &[],
                &["mailbox:read"],
                None,
            ),
        )
        .await;
        assert_eq!(folders_response.status(), StatusCode::OK);
        let folders: Value = serde_json::from_slice(
            &axum::body::to_bytes(folders_response.into_body(), 4096)
                .await
                .expect("read folders response"),
        )
        .expect("decode folders");
        assert_eq!(folders["items"].as_array().expect("folder list").len(), 6);
        let inbox_id = folders["items"]
            .as_array()
            .expect("folder list")
            .iter()
            .find(|folder| folder["name"] == "INBOX")
            .and_then(|folder| folder["id"].as_str())
            .expect("INBOX folder ID")
            .to_string();
        let child_body = serde_json::to_vec(&serde_json::json!({
            "name": "Receipts",
            "parent_id": inbox_id,
        }))
        .expect("encode child folder");
        let child_response = handle_request(
            State(state.clone()),
            signed_request(
                &state,
                tenant_id,
                user_id,
                "POST",
                &format!("/v1/mailboxes/{mailbox_id}/folders"),
                &child_body,
                &["mailbox:write"],
                None,
            ),
        )
        .await;
        assert_eq!(child_response.status(), StatusCode::CREATED);

        let raw_message = b"Subject: idempotent API test\r\n\r\nmessage body";
        let append_uri = format!("/v1/mailboxes/{mailbox_id}/messages");
        let append = |body: &[u8]| {
            handle_request(
                State(state.clone()),
                signed_request(
                    &state,
                    tenant_id,
                    user_id,
                    "POST",
                    &append_uri,
                    body,
                    &["mailbox:write"],
                    Some("mailbox-api-append-001"),
                ),
            )
        };
        let first = append(raw_message).await;
        assert_eq!(first.status(), StatusCode::CREATED);
        let first: Value = serde_json::from_slice(
            &axum::body::to_bytes(first.into_body(), 4096)
                .await
                .expect("read append response"),
        )
        .expect("decode append result");
        let replay = append(raw_message).await;
        assert_eq!(replay.status(), StatusCode::CREATED);
        let replay: Value = serde_json::from_slice(
            &axum::body::to_bytes(replay.into_body(), 4096)
                .await
                .expect("read append replay"),
        )
        .expect("decode append replay result");
        assert_eq!(first["id"], replay["id"]);
        assert_eq!(first["uid"], replay["uid"]);
        assert_eq!(append(b"changed body").await.status(), StatusCode::CONFLICT);
        let request_sha256 = [0_u8; 32];
        sqlx::query(
            "INSERT INTO mailbox_append_idempotency
             (tenant_id, mailbox_id, user_id, idempotency_key, request_sha256, folder_id,
              result_folder_id, message_id, uid, result_flags, expires_at)
             VALUES ($1, $2, $3, 'expired-cleanup-test', $4, $5, $5, $6, 1, '{}',
                     NOW() - INTERVAL '1 second')",
        )
        .bind(tenant_id)
        .bind(Uuid::parse_str(mailbox_id).expect("mailbox UUID"))
        .bind(user_id)
        .bind(request_sha256.as_slice())
        .bind(Uuid::parse_str(&inbox_id).expect("INBOX UUID"))
        .bind(Uuid::parse_str(first["id"].as_str().expect("message ID")).expect("message UUID"))
        .execute(&pool)
        .await
        .expect("insert expired append idempotency fixture");
        assert_eq!(
            retention::prune_expired_batch(&pool)
                .await
                .expect("prune expired idempotency records"),
            1
        );
        assert_eq!(
            retention::prune_expired_messages_batch(&pool)
                .await
                .expect("prune expired retained messages"),
            0
        );

        let content_uri = format!(
            "/v1/mailboxes/{mailbox_id}/messages/{}/content",
            first["id"].as_str().expect("message ID")
        );
        let content = handle_request(
            State(state.clone()),
            signed_request(
                &state,
                tenant_id,
                user_id,
                "GET",
                &content_uri,
                &[],
                &["mailbox:read"],
                None,
            ),
        )
        .await;
        assert_eq!(content.status(), StatusCode::OK);
        assert_eq!(
            axum::body::to_bytes(content.into_body(), 4096)
                .await
                .expect("read message content")
                .as_ref(),
            raw_message
        );
        let inbound_fixtures = [
            (
                Uuid::new_v4(),
                "stored",
                "inbound.message.accepted",
                b"Subject: projected inbox\r\n\r\ninbox body".as_slice(),
                "INBOX",
            ),
            (
                Uuid::new_v4(),
                "quarantined",
                "inbound.message.quarantined",
                b"Subject: projected quarantine\r\n\r\nquarantine body".as_slice(),
                "Junk",
            ),
        ];
        let inbound_bytes = inbound_fixtures
            .iter()
            .map(|(_, _, _, raw, _)| raw.len() as i64)
            .sum::<i64>();
        sqlx::query("UPDATE mailboxes SET used_bytes = used_bytes + $2 WHERE id = $1")
            .bind(Uuid::parse_str(mailbox_id).expect("mailbox UUID"))
            .bind(inbound_bytes)
            .execute(&pool)
            .await
            .expect("reserve inbound fixture quota");
        for (message_id, status, event_type, raw, _) in &inbound_fixtures {
            let mut idempotency_key = [0; 32];
            idempotency_key[..16].copy_from_slice(message_id.as_bytes());
            sqlx::query(
                "INSERT INTO inbound_messages
                 (id, tenant_id, mailbox_id, envelope_sender, raw_message, raw_size,
                  idempotency_key, status)
                 VALUES ($1, $2, $3, 'sender@example.test', $4, $5, $6, $7)",
            )
            .bind(message_id)
            .bind(tenant_id)
            .bind(Uuid::parse_str(mailbox_id).expect("mailbox UUID"))
            .bind(*raw)
            .bind(raw.len() as i64)
            .bind(idempotency_key.as_slice())
            .bind(status)
            .execute(&pool)
            .await
            .expect("insert inbound projection fixture");
            sqlx::query(
                "INSERT INTO mailbox_event_outbox
                 (tenant_id, mailbox_id, event_type, idempotency_key, payload)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(tenant_id)
            .bind(Uuid::parse_str(mailbox_id).expect("mailbox UUID"))
            .bind(event_type)
            .bind(message_id.to_string())
            .bind(serde_json::json!({"message_id": message_id}))
            .execute(&pool)
            .await
            .expect("insert inbound projection event");
        }
        assert!(
            inbound_projection::project_next_for_tenant(&pool, Some(tenant_id))
                .await
                .expect("project inbound event")
        );
        assert!(
            inbound_projection::project_next_for_tenant(&pool, Some(tenant_id))
                .await
                .expect("project second inbound event")
        );
        assert!(
            !inbound_projection::project_next_for_tenant(&pool, Some(tenant_id))
                .await
                .expect("confirm inbound events drained")
        );
        for (message_id, _, _, _, expected_folder) in &inbound_fixtures {
            let folder_name: String = sqlx::query_scalar(
                "SELECT folder.name
                 FROM mailbox_messages AS message
                 JOIN mailbox_folders AS folder
                   ON folder.mailbox_id = message.mailbox_id AND folder.id = message.folder_id
                 WHERE message.id = $1",
            )
            .bind(message_id)
            .fetch_one(&pool)
            .await
            .expect("read projected message folder");
            assert_eq!(&folder_name, expected_folder);
        }
        let projected_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mailbox_event_outbox
             WHERE tenant_id = $1
               AND event_type IN ('inbound.message.accepted', 'inbound.message.quarantined')
               AND published_at IS NOT NULL AND attempts = 0",
        )
        .bind(tenant_id)
        .fetch_one(&pool)
        .await
        .expect("count acknowledged inbound events");
        assert_eq!(projected_count, 2);
        let missing_message_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mailbox_event_outbox
             (tenant_id, mailbox_id, event_type, idempotency_key, payload)
             VALUES ($1, $2, 'inbound.message.accepted', $3, $4)",
        )
        .bind(tenant_id)
        .bind(Uuid::parse_str(mailbox_id).expect("mailbox UUID"))
        .bind(missing_message_id.to_string())
        .bind(serde_json::json!({"message_id": missing_message_id}))
        .execute(&pool)
        .await
        .expect("insert failing inbound event");
        assert!(
            inbound_projection::project_next_for_tenant(&pool, Some(tenant_id))
                .await
                .is_err()
        );
        let retry_state = sqlx::query(
            "SELECT attempts, published_at, last_error, available_at > NOW() AS delayed
             FROM mailbox_event_outbox
             WHERE tenant_id = $1 AND idempotency_key = $2",
        )
        .bind(tenant_id)
        .bind(missing_message_id.to_string())
        .fetch_one(&pool)
        .await
        .expect("read failed event retry state");
        assert_eq!(retry_state.get::<i32, _>("attempts"), 1);
        assert!(
            retry_state
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("published_at")
                .is_none()
        );
        assert!(retry_state.get::<Option<String>, _>("last_error").is_some());
        assert!(retry_state.get::<bool, _>("delayed"));
        let quota_uri = format!("/v1/mailboxes/{mailbox_id}/quota");
        let quota_response = handle_request(
            State(state.clone()),
            signed_request(
                &state,
                tenant_id,
                user_id,
                "GET",
                &quota_uri,
                &[],
                &["mailbox:read"],
                None,
            ),
        )
        .await;
        let quota: Value = serde_json::from_slice(
            &axum::body::to_bytes(quota_response.into_body(), 4096)
                .await
                .expect("read quota response"),
        )
        .expect("decode quota");
        assert_eq!(
            quota["used_bytes"],
            raw_message.len() as i64 + inbound_bytes
        );

        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_events
             WHERE tenant_id = $1 AND action = 'mailbox.message.content.read'",
        )
        .bind(tenant_id)
        .fetch_one(&pool)
        .await
        .expect("count content access audit");
        assert_eq!(audit_count, 1);
        sqlx::query(
            "DELETE FROM inbound_recipients
             WHERE message_id IN (SELECT id FROM inbound_messages WHERE tenant_id = $1)",
        )
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("remove inbound recipient fixtures");
        sqlx::query("DELETE FROM inbound_messages WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("remove inbound message fixtures");
        sqlx::query("DELETE FROM mailboxes WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("remove mailbox fixture");
        sqlx::query("DELETE FROM domains WHERE id = $1")
            .bind(domain_id)
            .execute(&pool)
            .await
            .expect("remove domain fixture");
        sqlx::query("DELETE FROM tenant_memberships WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("remove membership fixture");
        sqlx::query("DELETE FROM audit_events WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("remove audit fixtures");
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("remove user fixture");
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await
            .expect("remove tenant fixture");
    }
}
