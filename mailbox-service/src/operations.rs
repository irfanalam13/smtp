use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{AppState, InternalPrincipal};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateMailboxRequest {
    domain_id: Uuid,
    local_part: String,
    display_name: String,
    kind: String,
    quota_bytes: i64,
    owner_user_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MailboxLifecycleRequest {
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FolderRequest {
    name: String,
    parent_id: Option<Uuid>,
    special_use: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameFolderRequest {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AliasRequest {
    domain_id: Uuid,
    local_part: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PermissionRequest {
    permission: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuotaRequest {
    quota_bytes: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelRequest {
    name: String,
    kind: String,
    color: Option<String>,
    query_definition: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetFolderRequest {
    target_folder_id: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlagsRequest {
    flags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AppendQuery {
    folder_id: Option<Uuid>,
}

pub(super) async fn dispatch(
    state: &AppState,
    principal: &InternalPrincipal,
    method: &str,
    path: &str,
    query: &str,
    body: &[u8],
    idempotency_key: Option<&str>,
) -> Option<Response> {
    let response = match (method, path) {
        ("POST", "/v1/mailboxes") if has_scope(principal, "mailbox:manage") => {
            create_mailbox(state, principal, body).await
        }
        ("PATCH", path) if mailbox_id(path, "").is_some() => {
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                update_mailbox_status(state, principal, mailbox_id(path, "").unwrap(), body).await
            }
        }
        ("GET", path) if path.ends_with("/aliases") => {
            let Some(mailbox_id) = mailbox_id(path, "/aliases") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                list_aliases(state, principal, mailbox_id).await
            }
        }
        ("GET", path) if path.ends_with("/permissions") => {
            let Some(mailbox_id) = mailbox_id(path, "/permissions") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                list_permissions(state, principal, mailbox_id).await
            }
        }
        ("POST", path) if path.ends_with("/messages") => {
            let Some(mailbox_id) = mailbox_id(path, "/messages") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                append_message(state, principal, mailbox_id, query, body, idempotency_key).await
            }
        }
        ("POST", path) if path.ends_with("/copy") || path.ends_with("/move") => {
            let suffix = if path.ends_with("/copy") {
                "/copy"
            } else {
                "/move"
            };
            let Some((mailbox_id, message_id)) =
                message_resource_ids(path.strip_suffix(suffix).unwrap_or_default())
            else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                copy_or_move_message(
                    state,
                    principal,
                    mailbox_id,
                    message_id,
                    body,
                    suffix == "/move",
                )
                .await
            }
        }
        ("PATCH", path) if path.ends_with("/flags") => {
            let Some((mailbox_id, message_id)) =
                message_resource_ids(path.strip_suffix("/flags").unwrap_or_default())
            else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                set_message_flags(state, principal, mailbox_id, message_id, body).await
            }
        }
        ("DELETE", path) if path.contains("/messages/") && !path.contains("/labels/") => {
            let Some((mailbox_id, message_id)) = message_resource_ids(path) else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                mark_message_deleted(state, principal, mailbox_id, message_id).await
            }
        }
        ("POST", path) if path.ends_with("/expunge") => {
            let Some((mailbox_id, folder_id)) =
                folder_resource_ids(path.strip_suffix("/expunge").unwrap_or_default())
            else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                expunge_folder(state, principal, mailbox_id, folder_id).await
            }
        }
        ("GET", path) if path.ends_with("/folders") => {
            let Some(mailbox_id) = mailbox_id(path, "/folders") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:read") {
                forbidden()
            } else {
                list_folders(state, principal, mailbox_id).await
            }
        }
        ("POST", path) if path.ends_with("/folders") => {
            let Some(mailbox_id) = mailbox_id(path, "/folders") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                create_folder(state, principal, mailbox_id, body).await
            }
        }
        ("PATCH", path) if path.contains("/folders/") => {
            let Some((mailbox_id, folder_id)) = resource_ids(path, "/folders/") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                rename_folder(state, principal, mailbox_id, folder_id, body).await
            }
        }
        ("DELETE", path) if path.contains("/folders/") => {
            let Some((mailbox_id, folder_id)) = resource_ids(path, "/folders/") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                delete_folder(state, principal, mailbox_id, folder_id).await
            }
        }
        ("POST", path) if path.ends_with("/aliases") => {
            let Some(mailbox_id) = mailbox_id(path, "/aliases") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                create_alias(state, principal, mailbox_id, body).await
            }
        }
        ("DELETE", path) if path.contains("/aliases/") => {
            let Some((mailbox_id, alias_id)) = resource_ids(path, "/aliases/") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                delete_alias(state, principal, mailbox_id, alias_id).await
            }
        }
        ("PUT", path) if path.contains("/permissions/") => {
            let Some((mailbox_id, user_id)) = resource_ids(path, "/permissions/") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                set_permission(state, principal, mailbox_id, user_id, body).await
            }
        }
        ("DELETE", path) if path.contains("/permissions/") => {
            let Some((mailbox_id, user_id)) = resource_ids(path, "/permissions/") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                revoke_permission(state, principal, mailbox_id, user_id).await
            }
        }
        ("GET", path) if path.ends_with("/quota") => {
            let Some(mailbox_id) = mailbox_id(path, "/quota") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:read") {
                forbidden()
            } else {
                get_quota(state, principal, mailbox_id).await
            }
        }
        ("PUT", path) if path.ends_with("/quota") => {
            let Some(mailbox_id) = mailbox_id(path, "/quota") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                set_quota(state, principal, mailbox_id, body).await
            }
        }
        ("GET", path) if path.ends_with("/labels") => {
            let Some(mailbox_id) = mailbox_id(path, "/labels") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:read") {
                forbidden()
            } else {
                list_labels(state, principal, mailbox_id).await
            }
        }
        ("POST", path) if path.ends_with("/labels") => {
            let Some(mailbox_id) = mailbox_id(path, "/labels") else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:manage") {
                forbidden()
            } else {
                create_label(state, principal, mailbox_id, body).await
            }
        }
        ("PUT", path) if path.contains("/labels/") => {
            let Some((mailbox_id, ids)) = message_label_ids(path) else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                assign_label(state, principal, mailbox_id, ids.0, ids.1).await
            }
        }
        ("DELETE", path) if path.contains("/labels/") => {
            let Some((mailbox_id, ids)) = message_label_ids(path) else {
                return Some(not_found());
            };
            if !has_scope(principal, "mailbox:write") {
                forbidden()
            } else {
                remove_label(state, principal, mailbox_id, ids.0, ids.1).await
            }
        }
        _ => return None,
    };
    Some(response)
}

fn has_scope(principal: &InternalPrincipal, scope: &str) -> bool {
    principal.scopes.iter().any(|value| value == scope)
}

pub(super) fn mailbox_id(path: &str, suffix: &str) -> Option<Uuid> {
    path.strip_prefix("/v1/mailboxes/")?
        .strip_suffix(suffix)?
        .parse()
        .ok()
}

fn resource_ids(path: &str, segment: &str) -> Option<(Uuid, Uuid)> {
    let (mailbox_id, resource_id) = path
        .strip_prefix("/v1/mailboxes/")?
        .split_once(segment.trim_start_matches("/v1/mailboxes"))?;
    Some((mailbox_id.parse().ok()?, resource_id.parse().ok()?))
}

fn message_label_ids(path: &str) -> Option<(Uuid, (Uuid, Uuid))> {
    let path = path.strip_prefix("/v1/mailboxes/")?;
    let (mailbox_id, rest) = path.split_once("/messages/")?;
    let (message_id, label_id) = rest.split_once("/labels/")?;
    Some((
        mailbox_id.parse().ok()?,
        (message_id.parse().ok()?, label_id.parse().ok()?),
    ))
}

fn message_resource_ids(path: &str) -> Option<(Uuid, Uuid)> {
    let path = path.strip_prefix("/v1/mailboxes/")?;
    let (mailbox_id, message_id) = path.split_once("/messages/")?;
    Some((mailbox_id.parse().ok()?, message_id.parse().ok()?))
}

fn folder_resource_ids(path: &str) -> Option<(Uuid, Uuid)> {
    let path = path.strip_prefix("/v1/mailboxes/")?;
    let (mailbox_id, folder_id) = path.split_once("/folders/")?;
    Some((mailbox_id.parse().ok()?, folder_id.parse().ok()?))
}

pub(super) async fn authorize(
    transaction: &mut Transaction<'_, Postgres>,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    permission: &str,
) -> Result<bool, sqlx::Error> {
    let tenant_id = Uuid::parse_str(&principal.tenant_id).expect("verified UUID");
    let user_id = Uuid::parse_str(&principal.user_id).expect("verified UUID");
    let membership = sqlx::query(
        "SELECT membership.role, mailbox.owner_user_id
         FROM mailboxes AS mailbox
         JOIN tenants AS tenant
           ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
         JOIN users AS account
           ON account.id = $3 AND account.status = 'active'
         JOIN tenant_memberships AS membership
           ON membership.tenant_id = mailbox.tenant_id
          AND membership.user_id = account.id
         WHERE mailbox.id = $1 AND mailbox.tenant_id = $2
           AND (mailbox.status = 'active'
                OR ($4 = 'lifecycle' AND mailbox.status IN ('suspended', 'deleting')))
           AND mailbox.deleted_at IS NULL
         FOR SHARE OF mailbox, tenant, account, membership",
    )
    .bind(mailbox_id)
    .bind(tenant_id)
    .bind(user_id)
    .bind(permission)
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(membership) = membership else {
        return Ok(false);
    };
    let role: String = membership.try_get("role")?;
    let owner: Option<Uuid> = membership.try_get("owner_user_id")?;
    if owner == Some(user_id) || (permission == "manage" && role != "member") {
        return Ok(true);
    }
    let allowed = match permission {
        "read" => vec!["read", "read_write", "manage"],
        "write" => vec!["read_write", "manage"],
        "lifecycle" => vec!["manage"],
        _ => vec!["manage"],
    };
    Ok(sqlx::query(
        "SELECT permission
         FROM mailbox_permissions
         WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3
           AND permission = ANY($4)
         FOR SHARE",
    )
    .bind(tenant_id)
    .bind(mailbox_id)
    .bind(user_id)
    .bind(allowed)
    .fetch_optional(&mut **transaction)
    .await?
    .is_some())
}

pub(super) async fn get_mailbox(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin mailbox read"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "read").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize mailbox read"),
    }
    let row = match sqlx::query(
        "SELECT mailbox.id, mailbox.tenant_id, mailbox.domain_id,
                mailbox.local_part, domain.domain, mailbox.display_name,
                mailbox.mailbox_kind, mailbox.owner_user_id, mailbox.status,
                mailbox.used_bytes, mailbox.quota_bytes, mailbox.created_at
         FROM mailboxes AS mailbox
         JOIN domains AS domain
           ON domain.id = mailbox.domain_id AND domain.tenant_id = mailbox.tenant_id
         WHERE mailbox.id = $1",
    )
    .bind(mailbox_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(row) => row,
        Err(error) => return database_error(error, "read mailbox"),
    };
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit mailbox read");
    }
    json_response(
        StatusCode::OK,
        json!({
            "id": row.get::<Uuid, _>("id"),
            "tenant_id": row.get::<Uuid, _>("tenant_id"),
            "domain_id": row.get::<Uuid, _>("domain_id"),
            "address": format!(
                "{}@{}",
                row.get::<String, _>("local_part"),
                row.get::<String, _>("domain")
            ),
            "display_name": row.get::<String, _>("display_name"),
            "kind": row.get::<String, _>("mailbox_kind"),
            "owner_user_id": row.get::<Option<Uuid>, _>("owner_user_id"),
            "status": row.get::<String, _>("status"),
            "used_bytes": row.get::<i64, _>("used_bytes"),
            "quota_bytes": row.get::<i64, _>("quota_bytes"),
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
        }),
    )
}

async fn update_mailbox_status(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    body: &[u8],
) -> Response {
    let request: MailboxLifecycleRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if !matches!(request.status.as_str(), "active" | "suspended" | "deleting") {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin mailbox lifecycle update"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "lifecycle").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize mailbox lifecycle update"),
    }
    let current_status: String =
        match sqlx::query_scalar("SELECT status FROM mailboxes WHERE id = $1 FOR UPDATE")
            .bind(mailbox_id)
            .fetch_one(&mut *transaction)
            .await
        {
            Ok(status) => status,
            Err(error) => return database_error(error, "read mailbox lifecycle"),
        };
    let valid_transition = current_status == request.status
        || matches!(
            (current_status.as_str(), request.status.as_str()),
            ("active", "suspended" | "deleting") | ("suspended", "active" | "deleting")
        );
    if !valid_transition {
        return conflict();
    }
    if current_status != request.status {
        if let Err(error) = sqlx::query(
            "UPDATE mailboxes
             SET status = $2,
                 deleted_at = CASE WHEN $2 = 'deleting'
                                   THEN COALESCE(deleted_at, NOW()) ELSE deleted_at END
             WHERE id = $1",
        )
        .bind(mailbox_id)
        .bind(&request.status)
        .execute(&mut *transaction)
        .await
        {
            return database_error(error, "update mailbox lifecycle");
        }
        if let Err(error) = audit(
            &mut transaction,
            principal,
            "mailbox.lifecycle.updated",
            "mailbox",
            mailbox_id,
            json!({"previous_status": current_status, "status": request.status}),
        )
        .await
        {
            return database_error(error, "audit mailbox lifecycle");
        }
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit mailbox lifecycle update");
    }
    json_response(
        StatusCode::OK,
        json!({"id": mailbox_id, "status": request.status}),
    )
}

async fn list_aliases(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin mailbox alias listing"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize mailbox alias listing"),
    }
    let rows = match sqlx::query(
        "SELECT alias.id, alias.local_part, alias.status, domain.domain, alias.created_at
         FROM mailbox_aliases AS alias
         JOIN domains AS domain
           ON domain.id = alias.domain_id AND domain.tenant_id = alias.tenant_id
         WHERE alias.tenant_id = $1 AND alias.mailbox_id = $2
         ORDER BY lower(domain.domain), lower(alias.local_part), alias.id",
    )
    .bind(Uuid::parse_str(&principal.tenant_id).expect("verified UUID"))
    .bind(mailbox_id)
    .fetch_all(&mut *transaction)
    .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error, "list mailbox aliases"),
    };
    let items = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<Uuid, _>("id"),
                "address": format!(
                    "{}@{}",
                    row.get::<String, _>("local_part"),
                    row.get::<String, _>("domain")
                ),
                "status": row.get::<String, _>("status"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            })
        })
        .collect::<Vec<_>>();
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit mailbox alias listing");
    }
    json_response(StatusCode::OK, json!({"items": items, "next_page": null}))
}

async fn list_permissions(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin mailbox permission listing"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize mailbox permission listing"),
    }
    let rows = match sqlx::query(
        "SELECT permission.user_id, permission.permission, permission.created_at
         FROM mailbox_permissions AS permission
         WHERE permission.tenant_id = $1 AND permission.mailbox_id = $2
         ORDER BY permission.created_at, permission.user_id",
    )
    .bind(Uuid::parse_str(&principal.tenant_id).expect("verified UUID"))
    .bind(mailbox_id)
    .fetch_all(&mut *transaction)
    .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error, "list mailbox permissions"),
    };
    let items = rows
        .iter()
        .map(|row| {
            json!({
                "user_id": row.get::<Uuid, _>("user_id"),
                "permission": row.get::<String, _>("permission"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            })
        })
        .collect::<Vec<_>>();
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit mailbox permission listing");
    }
    json_response(StatusCode::OK, json!({"items": items, "next_page": null}))
}

async fn create_mailbox(state: &AppState, principal: &InternalPrincipal, body: &[u8]) -> Response {
    let request: CreateMailboxRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if !matches!(request.kind.as_str(), "user" | "shared") || request.quota_bytes <= 0 {
        return bad_request();
    }
    let tenant_id = Uuid::parse_str(&principal.tenant_id).expect("verified UUID");
    let user_id = Uuid::parse_str(&principal.user_id).expect("verified UUID");
    let owner_user_id = match (request.kind.as_str(), request.owner_user_id) {
        ("user", Some(owner)) => Some(owner),
        ("user", None) => Some(user_id),
        (_, owner) => owner,
    };
    let mailbox_kind = request.kind.clone();
    let request = smtp_core::CreateMailbox {
        tenant_id,
        domain_id: request.domain_id,
        local_part: request.local_part,
        display_name: request.display_name,
        owner_user_id,
        kind: if request.kind == "shared" {
            smtp_core::MailboxKind::Shared
        } else {
            smtp_core::MailboxKind::User
        },
        quota_bytes: request.quota_bytes,
    };
    match state
        .mailbox_store
        .create_mailbox_for_user(&request, user_id)
        .await
    {
        Ok(mailbox) => json_response(
            StatusCode::CREATED,
            json!({
                "id": mailbox.id,
                "tenant_id": mailbox.tenant_id,
                "domain_id": mailbox.domain_id,
                "address_local_part": mailbox.local_part,
                "display_name": mailbox.display_name,
                "owner_user_id": mailbox.owner_user_id,
                "kind": mailbox_kind,
                "status": "active",
                "quota_bytes": mailbox.quota_bytes,
                "used_bytes": mailbox.used_bytes,
            }),
        ),
        Err(error) => {
            tracing::warn!(%error, "mailbox provisioning rejected");
            if matches!(
                &error,
                smtp_core::MailboxStoreError::Database(database_error)
                    if is_unique_violation(database_error)
            ) {
                conflict()
            } else if matches!(
                error,
                smtp_core::MailboxStoreError::Validation(_)
                    | smtp_core::MailboxStoreError::InvalidMailboxQuota
                    | smtp_core::MailboxStoreError::DisplayNameTooLong
            ) {
                bad_request()
            } else if matches!(error, smtp_core::MailboxStoreError::MailboxNotFound) {
                not_found()
            } else {
                unavailable()
            }
        }
    }
}

async fn list_folders(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin folder listing"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "read").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize folder listing"),
    }
    let rows = match sqlx::query(
        "SELECT id, parent_id, name, special_use, uid_validity, uid_next
         FROM mailbox_folders WHERE mailbox_id = $1 ORDER BY parent_id NULLS FIRST, name, id",
    )
    .bind(mailbox_id)
    .fetch_all(&mut *transaction)
    .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error, "list mailbox folders"),
    };
    let folders = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<Uuid, _>("id"),
                "parent_id": row.get::<Option<Uuid>, _>("parent_id"),
                "name": row.get::<String, _>("name"),
                "special_use": row.get::<Option<String>, _>("special_use"),
                "uid_validity": row.get::<i64, _>("uid_validity"),
                "uid_next": row.get::<i64, _>("uid_next"),
            })
        })
        .collect::<Vec<_>>();
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit folder listing");
    }
    json_response(StatusCode::OK, json!({"items": folders, "next_page": null}))
}

async fn create_folder(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    body: &[u8],
) -> Response {
    let request: FolderRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if !valid_name(&request.name)
        || request.special_use.as_deref().is_some_and(|value| {
            !matches!(
                value,
                "\\Sent" | "\\Drafts" | "\\Trash" | "\\Archive" | "\\Junk"
            )
        })
    {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin folder creation"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize folder creation"),
    }
    let id = Uuid::new_v4();
    let uid_validity: i64 = match sqlx::query_scalar("SELECT nextval('mailbox_uid_validity_seq')")
        .fetch_one(&mut *transaction)
        .await
    {
        Ok(value) => value,
        Err(error) => return database_error(error, "allocate folder UID validity"),
    };
    let row = match sqlx::query(
        "INSERT INTO mailbox_folders
             (id, mailbox_id, parent_id, name, special_use, uid_validity)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, parent_id, name, special_use, uid_validity, uid_next",
    )
    .bind(id)
    .bind(mailbox_id)
    .bind(request.parent_id)
    .bind(request.name)
    .bind(request.special_use)
    .bind(uid_validity)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(row) => row,
        Err(error) if is_unique_violation(&error) => return conflict(),
        Err(error) => return database_error(error, "create mailbox folder"),
    };
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.folder.created",
        "mailbox_folder",
        id,
        json!({"mailbox_id": mailbox_id}),
    )
    .await
    {
        return database_error(error, "audit folder creation");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit folder creation");
    }
    json_response(StatusCode::CREATED, folder_json(&row))
}

async fn rename_folder(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    folder_id: Uuid,
    body: &[u8],
) -> Response {
    let request: RenameFolderRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if !valid_name(&request.name) {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin folder rename"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize folder rename"),
    }
    let row = match sqlx::query(
        "UPDATE mailbox_folders
         SET name = $3, updated_at = NOW()
         WHERE mailbox_id = $1 AND id = $2 AND name <> 'INBOX'
         RETURNING id, parent_id, name, special_use, uid_validity, uid_next",
    )
    .bind(mailbox_id)
    .bind(folder_id)
    .bind(request.name)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return not_found(),
        Err(error) if is_unique_violation(&error) => return conflict(),
        Err(error) => return database_error(error, "rename mailbox folder"),
    };
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.folder.renamed",
        "mailbox_folder",
        folder_id,
        json!({"mailbox_id": mailbox_id}),
    )
    .await
    {
        return database_error(error, "audit folder rename");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit folder rename");
    }
    json_response(StatusCode::OK, folder_json(&row))
}

async fn delete_folder(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    folder_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin folder deletion"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize folder deletion"),
    }
    let folder = match sqlx::query(
        "SELECT special_use, name, parent_id FROM mailbox_folders
         WHERE mailbox_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(mailbox_id)
    .bind(folder_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return not_found(),
        Err(error) => return database_error(error, "find mailbox folder"),
    };
    let special_use: Option<String> = folder.get("special_use");
    let name: String = folder.get("name");
    let parent_id: Option<Uuid> = folder.get("parent_id");
    if special_use.is_some() || (name == "INBOX" && parent_id.is_none()) {
        return conflict();
    }
    let occupied: bool = match sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM mailbox_messages WHERE folder_id = $1
             UNION ALL
             SELECT 1 FROM mailbox_folders WHERE parent_id = $1
         )",
    )
    .bind(folder_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(error) => return database_error(error, "check folder contents"),
    };
    if occupied {
        return conflict();
    }
    if let Err(error) = sqlx::query("DELETE FROM mailbox_folders WHERE mailbox_id = $1 AND id = $2")
        .bind(mailbox_id)
        .bind(folder_id)
        .execute(&mut *transaction)
        .await
    {
        return database_error(error, "delete mailbox folder");
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.folder.deleted",
        "mailbox_folder",
        folder_id,
        json!({"mailbox_id": mailbox_id}),
    )
    .await
    {
        return database_error(error, "audit folder deletion");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit folder deletion");
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn create_alias(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    body: &[u8],
) -> Response {
    let request: AliasRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if !valid_local_part(&request.local_part) {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin alias creation"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize alias creation"),
    }
    let tenant_id = Uuid::parse_str(&principal.tenant_id).expect("verified UUID");
    let domain_verified: bool = match sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM domains
             WHERE id = $1 AND tenant_id = $2 AND status = 'verified'
         )",
    )
    .bind(request.domain_id)
    .bind(tenant_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(error) => return database_error(error, "check alias domain"),
    };
    if !domain_verified {
        return bad_request();
    }
    let alias_id = Uuid::new_v4();
    if let Err(error) = sqlx::query(
        "INSERT INTO mailbox_aliases (id, tenant_id, mailbox_id, domain_id, local_part)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(alias_id)
    .bind(tenant_id)
    .bind(mailbox_id)
    .bind(request.domain_id)
    .bind(&request.local_part)
    .execute(&mut *transaction)
    .await
    {
        if is_unique_violation(&error) {
            return conflict();
        }
        return database_error(error, "create mailbox alias");
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.alias.created",
        "mailbox_alias",
        alias_id,
        json!({"mailbox_id": mailbox_id, "domain_id": request.domain_id}),
    )
    .await
    {
        return database_error(error, "audit alias creation");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit alias creation");
    }
    json_response(StatusCode::CREATED, json!({"id": alias_id}))
}

async fn delete_alias(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    alias_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin alias deletion"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize alias deletion"),
    }
    let result = match sqlx::query(
        "DELETE FROM mailbox_aliases
         WHERE tenant_id = $1 AND mailbox_id = $2 AND id = $3",
    )
    .bind(Uuid::parse_str(&principal.tenant_id).expect("verified UUID"))
    .bind(mailbox_id)
    .bind(alias_id)
    .execute(&mut *transaction)
    .await
    {
        Ok(result) => result,
        Err(error) => return database_error(error, "delete mailbox alias"),
    };
    if result.rows_affected() == 0 {
        return not_found();
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.alias.deleted",
        "mailbox_alias",
        alias_id,
        json!({"mailbox_id": mailbox_id}),
    )
    .await
    {
        return database_error(error, "audit alias deletion");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit alias deletion");
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn set_permission(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    target_user_id: Uuid,
    body: &[u8],
) -> Response {
    let request: PermissionRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if !matches!(
        request.permission.as_str(),
        "read" | "read_write" | "manage"
    ) {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin permission update"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize permission update"),
    }
    let tenant_id = Uuid::parse_str(&principal.tenant_id).expect("verified UUID");
    let member_exists: bool = match sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM tenant_memberships AS membership
             JOIN users AS account ON account.id = membership.user_id
             WHERE membership.tenant_id = $1 AND membership.user_id = $2
               AND account.status = 'active'
         )",
    )
    .bind(tenant_id)
    .bind(target_user_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(error) => return database_error(error, "check mailbox permission member"),
    };
    if !member_exists {
        return not_found();
    }
    if let Err(error) = sqlx::query(
        "INSERT INTO mailbox_permissions (tenant_id, mailbox_id, user_id, permission)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (mailbox_id, user_id)
         DO UPDATE SET permission = EXCLUDED.permission, tenant_id = EXCLUDED.tenant_id",
    )
    .bind(tenant_id)
    .bind(mailbox_id)
    .bind(target_user_id)
    .bind(&request.permission)
    .execute(&mut *transaction)
    .await
    {
        return database_error(error, "set mailbox permission");
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.permission.updated",
        "mailbox",
        mailbox_id,
        json!({"user_id": target_user_id, "permission": request.permission}),
    )
    .await
    {
        return database_error(error, "audit permission update");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit permission update");
    }
    json_response(
        StatusCode::OK,
        json!({"mailbox_id": mailbox_id, "user_id": target_user_id}),
    )
}

async fn revoke_permission(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    target_user_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin permission revocation"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize permission revocation"),
    }
    let result = match sqlx::query(
        "DELETE FROM mailbox_permissions
         WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3",
    )
    .bind(Uuid::parse_str(&principal.tenant_id).expect("verified UUID"))
    .bind(mailbox_id)
    .bind(target_user_id)
    .execute(&mut *transaction)
    .await
    {
        Ok(result) => result,
        Err(error) => return database_error(error, "revoke mailbox permission"),
    };
    if result.rows_affected() == 0 {
        return not_found();
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.permission.revoked",
        "mailbox",
        mailbox_id,
        json!({"user_id": target_user_id}),
    )
    .await
    {
        return database_error(error, "audit permission revocation");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit permission revocation");
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn append_message(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    raw_query: &str,
    body: &[u8],
    idempotency_key: Option<&str>,
) -> Response {
    let Some(idempotency_key) = idempotency_key else {
        return bad_request();
    };
    let query: AppendQuery = match serde_urlencoded::from_str(raw_query) {
        Ok(query) => query,
        Err(_) => return bad_request(),
    };
    if body.is_empty() {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin message append"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize message append"),
    }
    let folder_id = match query.folder_id {
        Some(folder_id) => match sqlx::query_scalar(
            "SELECT id FROM mailbox_folders WHERE mailbox_id = $1 AND id = $2",
        )
        .bind(mailbox_id)
        .bind(folder_id)
        .fetch_optional(&mut *transaction)
        .await
        {
            Ok(Some(id)) => id,
            Ok(None) => return not_found(),
            Err(error) => return database_error(error, "find append folder"),
        },
        None => match sqlx::query_scalar(
            "SELECT id FROM mailbox_folders
             WHERE mailbox_id = $1 AND (special_use = '\\Inbox' OR name = 'INBOX')
             ORDER BY (special_use = '\\Inbox') DESC NULLS LAST, id LIMIT 1",
        )
        .bind(mailbox_id)
        .fetch_optional(&mut *transaction)
        .await
        {
            Ok(Some(id)) => id,
            Ok(None) => return unavailable(),
            Err(error) => return database_error(error, "find inbox folder"),
        },
    };
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit message append authorization");
    }
    match state
        .mailbox_store
        .append_idempotent_for_user(
            (
                Uuid::parse_str(&principal.tenant_id).expect("verified UUID"),
                Uuid::parse_str(&principal.user_id).expect("verified UUID"),
            ),
            folder_id,
            body,
            &[],
            chrono::Utc::now(),
            idempotency_key,
        )
        .await
    {
        Ok(message) => json_response(
            StatusCode::CREATED,
            json!({
                "id": message.id,
                "mailbox_id": mailbox_id,
                "folder_id": message.folder_id,
                "uid": message.uid,
            }),
        ),
        Err(error) => mailbox_store_error(error),
    }
}

async fn copy_or_move_message(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    message_id: Uuid,
    body: &[u8],
    move_message: bool,
) -> Response {
    let request: TargetFolderRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin mailbox message transfer"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize mailbox message transfer"),
    }
    let source = match sqlx::query(
        "SELECT id, folder_id, uid, raw_size
         FROM mailbox_messages
         WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL
         FOR UPDATE",
    )
    .bind(mailbox_id)
    .bind(message_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return not_found(),
        Err(error) => return database_error(error, "find source mailbox message"),
    };
    let source_folder_id: Uuid = source.get("folder_id");
    let source_uid: i64 = source.get("uid");
    let raw_size: i64 = source.get("raw_size");
    if move_message && source_folder_id == request.target_folder_id {
        if let Err(error) = transaction.commit().await {
            return database_error(error, "commit no-op message move");
        }
        return json_response(
            StatusCode::OK,
            json!({"id": message_id, "folder_id": source_folder_id, "uid": source_uid}),
        );
    }
    let target_uid = match sqlx::query_scalar::<_, i64>(
        "UPDATE mailbox_folders
         SET uid_next = uid_next + 1, updated_at = NOW()
         WHERE mailbox_id = $1 AND id = $2 AND uid_next < 9223372036854775807
         RETURNING uid_next - 1",
    )
    .bind(mailbox_id)
    .bind(request.target_folder_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(uid)) => uid,
        Ok(None) => return not_found(),
        Err(error) => return database_error(error, "allocate target folder UID"),
    };
    let (result_id, result) = if move_message {
        let result = sqlx::query(
            "UPDATE mailbox_messages
             SET folder_id = $3, uid = $4
             WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(request.target_folder_id)
        .bind(target_uid)
        .execute(&mut *transaction)
        .await;
        (message_id, result)
    } else {
        let charged = match sqlx::query_scalar::<_, Uuid>(
            "UPDATE mailboxes
             SET used_bytes = used_bytes + $1
             WHERE id = $2 AND status = 'active' AND deleted_at IS NULL
               AND $1 <= quota_bytes AND used_bytes <= quota_bytes - $1
             RETURNING id",
        )
        .bind(raw_size)
        .bind(mailbox_id)
        .fetch_optional(&mut *transaction)
        .await
        {
            Ok(Some(id)) => id,
            Ok(None) => return insufficient_storage(),
            Err(error) => return database_error(error, "charge copied message quota"),
        };
        let _ = charged;
        let new_id = Uuid::new_v4();
        let result = sqlx::query(
            "INSERT INTO mailbox_messages
                (id, mailbox_id, folder_id, uid, raw_message, raw_size,
                 message_schema_version, parsed_metadata, flags, internal_date,
                 deleted_at, sender_address, recipients, recipients_search_text,
                 subject, body_text, attachment_names, attachment_names_search_text,
                 header_message_id, in_reply_to, references_ids, conversation_id,
                 content_sha256, sent_at, custom_properties)
             SELECT $3, mailbox_id, $4, $5, raw_message, raw_size,
                    message_schema_version, parsed_metadata, flags, internal_date,
                    NULL, sender_address, recipients, recipients_search_text,
                    subject, body_text, attachment_names, attachment_names_search_text,
                    header_message_id, in_reply_to, references_ids, conversation_id,
                    content_sha256, sent_at, custom_properties
             FROM mailbox_messages
             WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(new_id)
        .bind(request.target_folder_id)
        .bind(target_uid)
        .execute(&mut *transaction)
        .await;
        (new_id, result)
    };
    match result {
        Ok(result) if result.rows_affected() == 1 => {}
        Ok(_) => return not_found(),
        Err(error) => return database_error(error, "persist mailbox message transfer"),
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        if move_message {
            "mailbox.message.moved"
        } else {
            "mailbox.message.copied"
        },
        "mailbox_message",
        result_id,
        json!({
            "mailbox_id": mailbox_id,
            "source_message_id": message_id,
            "target_folder_id": request.target_folder_id,
        }),
    )
    .await
    {
        return database_error(error, "audit mailbox message transfer");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit mailbox message transfer");
    }
    json_response(
        StatusCode::OK,
        json!({"id": result_id, "folder_id": request.target_folder_id, "uid": target_uid}),
    )
}

async fn set_message_flags(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    message_id: Uuid,
    body: &[u8],
) -> Response {
    let request: FlagsRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if request.flags.len() > 5
        || request.flags.iter().any(|flag| {
            !matches!(
                flag.as_str(),
                "\\Seen" | "\\Answered" | "\\Flagged" | "\\Deleted" | "\\Draft"
            )
        })
    {
        return bad_request();
    }
    let mut flags = request.flags;
    flags.sort();
    flags.dedup();
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin message flag update"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize message flag update"),
    }
    let result = match sqlx::query(
        "UPDATE mailbox_messages SET flags = $3
         WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(mailbox_id)
    .bind(message_id)
    .bind(&flags)
    .execute(&mut *transaction)
    .await
    {
        Ok(result) => result,
        Err(error) => return database_error(error, "update mailbox message flags"),
    };
    if result.rows_affected() == 0 {
        return not_found();
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.message.flags.updated",
        "mailbox_message",
        message_id,
        json!({"mailbox_id": mailbox_id, "flags": flags}),
    )
    .await
    {
        return database_error(error, "audit message flag update");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit message flag update");
    }
    json_response(StatusCode::OK, json!({"id": message_id, "flags": flags}))
}

async fn mark_message_deleted(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    message_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin message deletion"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize message deletion"),
    }
    let result = match sqlx::query(
        "UPDATE mailbox_messages
         SET deleted_at = COALESCE(deleted_at, NOW()),
             flags = CASE WHEN '\\Deleted' = ANY(flags)
                          THEN flags ELSE array_append(flags, '\\Deleted') END
         WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(mailbox_id)
    .bind(message_id)
    .execute(&mut *transaction)
    .await
    {
        Ok(result) => result,
        Err(error) => return database_error(error, "mark mailbox message deleted"),
    };
    if result.rows_affected() == 0 {
        return not_found();
    }
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.message.deleted",
        "mailbox_message",
        message_id,
        json!({"mailbox_id": mailbox_id}),
    )
    .await
    {
        return database_error(error, "audit message deletion");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit message deletion");
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn expunge_folder(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    folder_id: Uuid,
) -> Response {
    let belongs: bool = match sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM mailbox_folders WHERE id = $1 AND mailbox_id = $2
         )",
    )
    .bind(folder_id)
    .bind(mailbox_id)
    .fetch_one(&state.pool)
    .await
    {
        Ok(value) => value,
        Err(error) => return database_error(error, "find expunge folder"),
    };
    if !belongs {
        return not_found();
    }
    match state
        .mailbox_store
        .expunge_for_user(
            Uuid::parse_str(&principal.tenant_id).expect("verified UUID"),
            Uuid::parse_str(&principal.user_id).expect("verified UUID"),
            folder_id,
        )
        .await
    {
        Ok(count) => json_response(StatusCode::OK, json!({"expunged": count})),
        Err(error) => mailbox_store_error(error),
    }
}

async fn get_quota(state: &AppState, principal: &InternalPrincipal, mailbox_id: Uuid) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin quota read"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "read").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize quota read"),
    }
    let row = match sqlx::query("SELECT used_bytes, quota_bytes FROM mailboxes WHERE id = $1")
        .bind(mailbox_id)
        .fetch_one(&mut *transaction)
        .await
    {
        Ok(row) => row,
        Err(error) => return database_error(error, "read mailbox quota"),
    };
    let used: i64 = row.get("used_bytes");
    let quota: i64 = row.get("quota_bytes");
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit quota read");
    }
    json_response(
        StatusCode::OK,
        json!({
            "mailbox_id": mailbox_id,
            "used_bytes": used,
            "quota_bytes": quota,
            "usage_percent": ((used as f64 / quota as f64) * 100.0).round() as u8,
        }),
    )
}

async fn set_quota(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    body: &[u8],
) -> Response {
    let request: QuotaRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if request.quota_bytes <= 0 {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin quota update"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize quota update"),
    }
    let row = match sqlx::query(
        "UPDATE mailboxes SET quota_bytes = $2
         WHERE id = $1 AND used_bytes <= $2
         RETURNING used_bytes, quota_bytes",
    )
    .bind(mailbox_id)
    .bind(request.quota_bytes)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return conflict(),
        Err(error) => return database_error(error, "update mailbox quota"),
    };
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.quota.updated",
        "mailbox",
        mailbox_id,
        json!({"quota_bytes": request.quota_bytes}),
    )
    .await
    {
        return database_error(error, "audit quota update");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit quota update");
    }
    json_response(
        StatusCode::OK,
        json!({
            "mailbox_id": mailbox_id,
            "used_bytes": row.get::<i64, _>("used_bytes"),
            "quota_bytes": row.get::<i64, _>("quota_bytes"),
        }),
    )
}

async fn list_labels(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin label listing"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "read").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize label listing"),
    }
    let rows = match sqlx::query(
        "SELECT id, name, label_kind, color, query_definition
         FROM mailbox_labels WHERE mailbox_id = $1 ORDER BY lower(name), id",
    )
    .bind(mailbox_id)
    .fetch_all(&mut *transaction)
    .await
    {
        Ok(rows) => rows,
        Err(error) => return database_error(error, "list mailbox labels"),
    };
    let labels = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<Uuid, _>("id"),
                "name": row.get::<String, _>("name"),
                "kind": row.get::<String, _>("label_kind"),
                "color": row.get::<Option<String>, _>("color"),
                "query_definition": row.get::<Option<Value>, _>("query_definition"),
            })
        })
        .collect::<Vec<_>>();
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit label listing");
    }
    json_response(StatusCode::OK, json!({"items": labels, "next_page": null}))
}

async fn create_label(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    body: &[u8],
) -> Response {
    let request: LabelRequest = match parse_body(body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if request.name.trim().is_empty()
        || request.name.len() > 64
        || !matches!(
            request.kind.as_str(),
            "label" | "category" | "tag" | "smart"
        )
        || request
            .color
            .as_deref()
            .is_some_and(|color| !valid_color(color))
        || (request.kind == "smart") != request.query_definition.is_some()
        || request
            .query_definition
            .as_ref()
            .is_some_and(|value| value.to_string().len() > 8192)
    {
        return bad_request();
    }
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin label creation"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "manage").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize label creation"),
    }
    let id = Uuid::new_v4();
    let row = match sqlx::query(
        "INSERT INTO mailbox_labels
            (id, mailbox_id, name, label_kind, color, query_definition)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, name, label_kind, color, query_definition",
    )
    .bind(id)
    .bind(mailbox_id)
    .bind(request.name.trim())
    .bind(&request.kind)
    .bind(&request.color)
    .bind(&request.query_definition)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(row) => row,
        Err(error) if is_unique_violation(&error) => return conflict(),
        Err(error) => return database_error(error, "create mailbox label"),
    };
    if let Err(error) = audit(
        &mut transaction,
        principal,
        "mailbox.label.created",
        "mailbox_label",
        id,
        json!({"mailbox_id": mailbox_id}),
    )
    .await
    {
        return database_error(error, "audit label creation");
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit label creation");
    }
    json_response(StatusCode::CREATED, label_json(&row))
}

async fn assign_label(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    message_id: Uuid,
    label_id: Uuid,
) -> Response {
    label_assignment(state, principal, mailbox_id, message_id, label_id, true).await
}

async fn remove_label(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    message_id: Uuid,
    label_id: Uuid,
) -> Response {
    label_assignment(state, principal, mailbox_id, message_id, label_id, false).await
}

async fn label_assignment(
    state: &AppState,
    principal: &InternalPrincipal,
    mailbox_id: Uuid,
    message_id: Uuid,
    label_id: Uuid,
    assign: bool,
) -> Response {
    let mut transaction = match state.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return database_error(error, "begin label assignment"),
    };
    match authorize(&mut transaction, principal, mailbox_id, "write").await {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(error) => return database_error(error, "authorize label assignment"),
    }
    let result = if assign {
        sqlx::query(
            "INSERT INTO mailbox_message_label_assignments (mailbox_id, message_id, label_id)
             SELECT $1, $2, $3
             WHERE EXISTS (
                 SELECT 1 FROM mailbox_messages
                 WHERE mailbox_id = $1 AND id = $2 AND deleted_at IS NULL
             )
             ON CONFLICT DO NOTHING",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(label_id)
        .execute(&mut *transaction)
        .await
    } else {
        sqlx::query(
            "DELETE FROM mailbox_message_label_assignments
             WHERE mailbox_id = $1 AND message_id = $2 AND label_id = $3",
        )
        .bind(mailbox_id)
        .bind(message_id)
        .bind(label_id)
        .execute(&mut *transaction)
        .await
    };
    match result {
        Ok(result) if assign && result.rows_affected() == 0 => return not_found(),
        Ok(_) => {}
        Err(error) => {
            if is_constraint_violation(&error) {
                return conflict();
            }
            return database_error(error, "change mailbox message label");
        }
    }
    if let Err(error) = transaction.commit().await {
        return database_error(error, "commit label assignment");
    }
    if assign {
        StatusCode::CREATED.into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    }
}

async fn audit(
    transaction: &mut Transaction<'_, Postgres>,
    principal: &InternalPrincipal,
    action: &str,
    target_type: &str,
    target_id: Uuid,
    metadata: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events
            (tenant_id, actor_user_id, action, target_type, target_id, metadata)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::parse_str(&principal.tenant_id).expect("verified UUID"))
    .bind(Uuid::parse_str(&principal.user_id).expect("verified UUID"))
    .bind(action)
    .bind(target_type)
    .bind(target_id.to_string())
    .bind(metadata)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn folder_json(row: &sqlx::postgres::PgRow) -> Value {
    json!({
        "id": row.get::<Uuid, _>("id"),
        "parent_id": row.get::<Option<Uuid>, _>("parent_id"),
        "name": row.get::<String, _>("name"),
        "special_use": row.get::<Option<String>, _>("special_use"),
        "uid_validity": row.get::<i64, _>("uid_validity"),
        "uid_next": row.get::<i64, _>("uid_next"),
    })
}

fn label_json(row: &sqlx::postgres::PgRow) -> Value {
    json!({
        "id": row.get::<Uuid, _>("id"),
        "name": row.get::<String, _>("name"),
        "kind": row.get::<String, _>("label_kind"),
        "color": row.get::<Option<String>, _>("color"),
        "query_definition": row.get::<Option<Value>, _>("query_definition"),
    })
}

#[allow(clippy::result_large_err)]
fn parse_body<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T, Response> {
    serde_json::from_slice(body).map_err(|_| bad_request())
}

fn valid_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 255 && !value.chars().any(char::is_control)
}

fn valid_local_part(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.contains('@')
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .as_deref()
        == Some("23505")
}

fn is_constraint_violation(error: &sqlx::Error) -> bool {
    matches!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23503" | "23514")
    )
}

fn mailbox_store_error(error: smtp_core::MailboxStoreError) -> Response {
    tracing::warn!(%error, "mailbox storage operation rejected");
    match error {
        smtp_core::MailboxStoreError::QuotaExceeded => insufficient_storage(),
        smtp_core::MailboxStoreError::MailboxUnavailable => conflict(),
        smtp_core::MailboxStoreError::IdempotencyConflict => conflict(),
        smtp_core::MailboxStoreError::MailboxNotFound
        | smtp_core::MailboxStoreError::FolderNotFound => not_found(),
        smtp_core::MailboxStoreError::Validation(smtp_core::MailboxError::MessageTooLarge) => {
            error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "message_too_large",
                "message exceeds the configured mailbox size limit",
            )
        }
        smtp_core::MailboxStoreError::Validation(_)
        | smtp_core::MailboxStoreError::InvalidMailboxQuota
        | smtp_core::MailboxStoreError::DisplayNameTooLong
        | smtp_core::MailboxStoreError::InvalidWorkflowValue => bad_request(),
        smtp_core::MailboxStoreError::Database(ref database_error)
            if is_unique_violation(database_error) =>
        {
            conflict()
        }
        smtp_core::MailboxStoreError::Database(_)
        | smtp_core::MailboxStoreError::UidExhausted
        | smtp_core::MailboxStoreError::InvalidUidValidity
        | smtp_core::MailboxStoreError::InvalidQuotaUsage
        | smtp_core::MailboxStoreError::InvalidMailboxKind
        | smtp_core::MailboxStoreError::InvalidMailboxStatus
        | smtp_core::MailboxStoreError::VersionConflict
        | smtp_core::MailboxStoreError::InvalidWorkflowState
        | smtp_core::MailboxStoreError::MessageMetadata
        | smtp_core::MailboxStoreError::MessageParsing(_) => unavailable(),
    }
}

fn insufficient_storage() -> Response {
    error_response(
        StatusCode::INSUFFICIENT_STORAGE,
        "mailbox_quota_exceeded",
        "mailbox quota would be exceeded",
    )
}

fn database_error(error: sqlx::Error, operation: &'static str) -> Response {
    tracing::error!(%error, operation, "mailbox database operation failed");
    unavailable()
}

fn json_response(status: StatusCode, value: Value) -> Response {
    let body = match serde_json::to_vec(&value) {
        Ok(body) => body,
        Err(error) => {
            tracing::error!(%error, "encode mailbox operation response");
            return unavailable();
        }
    };
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )],
        Body::from(body),
    )
        .into_response()
}

fn error_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    json_response(status, json!({"error": {"code": code, "message": message}}))
}

fn bad_request() -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "request body or path is invalid",
    )
}

fn forbidden() -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        "forbidden",
        "the authenticated user cannot perform this operation",
    )
}

fn not_found() -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "not_found",
        "mailbox resource was not found",
    )
}

fn conflict() -> Response {
    error_response(
        StatusCode::CONFLICT,
        "conflict",
        "mailbox resource conflicts with existing data",
    )
}

fn unavailable() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "service_unavailable",
        "mailbox service is temporarily unavailable",
    )
}

#[cfg(test)]
mod tests {
    use super::{mailbox_id, valid_local_part, valid_name};
    use uuid::Uuid;

    #[test]
    fn mailbox_path_parser_accepts_only_a_single_identifier_segment() {
        let id = Uuid::new_v4();
        assert_eq!(mailbox_id(&format!("/v1/mailboxes/{id}"), ""), Some(id));
        assert_eq!(
            mailbox_id(&format!("/v1/mailboxes/{id}/messages"), ""),
            None
        );
    }

    #[test]
    fn mailbox_api_names_are_bounded_and_control_free() {
        assert!(valid_name("Billing"));
        assert!(!valid_name(""));
        assert!(!valid_name("bad\r\nname"));
        assert!(valid_local_part("support"));
        assert!(!valid_local_part("support@example.test"));
        assert!(!valid_local_part(" bad "));
    }
}
