use super::*;

pub(super) async fn namespace<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut BufStream<S>,
    connection: &Connection,
    command: &CommandLine,
) -> io::Result<()> {
    if connection.identity.is_none() {
        return write_tagged(stream, &command.tag, "NO", "authenticate first").await;
    }
    stream
        .write_all(b"* NAMESPACE ((\"\" \"/\")) NIL NIL\r\n")
        .await?;
    write_tagged(stream, &command.tag, "OK", "NAMESPACE completed").await
}

pub(super) async fn list_mailboxes<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    let Some(identity) = &connection.identity else {
        return write_tagged(stream, &command.tag, "NO", "authenticate first").await;
    };
    if command.arguments.len() != 2 {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "LIST requires reference and pattern",
        )
        .await;
    }
    let pattern = command.arguments[1].value().unwrap_or_default();
    if pattern.len() > 255 {
        return write_tagged(stream, &command.tag, "BAD", "mailbox pattern too long").await;
    }
    let rows = match accessible_folders(&state.pool, identity).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!(%error, "IMAP LIST query failed");
            return write_tagged(stream, &command.tag, "NO", "mailbox service unavailable").await;
        }
    };
    for (name, special_use, _folder_id, _mailbox_id, _uid_validity, _uid_next) in rows {
        if mailbox_pattern_matches(pattern, &name) {
            let attributes = if special_use.as_deref() == Some("\\Noselect") {
                "(\\Noselect)"
            } else {
                "()"
            };
            stream
                .write_all(
                    format!("* {} {attributes} \"/\" {}\r\n", command.name, quote(&name))
                        .as_bytes(),
                )
                .await?;
        }
    }
    write_tagged(stream, &command.tag, "OK", "LIST completed").await
}

pub(super) fn mailbox_pattern_matches(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern == "%" {
        return !name.contains('/');
    }
    let prefix = pattern.split('*').next().unwrap_or_default();
    let suffix = pattern.split('*').nth(1).unwrap_or_default();
    name.starts_with(prefix) && name.ends_with(suffix)
}

pub(super) async fn accessible_folders(
    pool: &PgPool,
    identity: &Identity,
) -> Result<Vec<(String, Option<String>, Uuid, Uuid, i64, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        "WITH accessible AS (
             SELECT mailbox.id AS mailbox_id, mailbox.local_part || '@' || domain.domain AS address,
                    mailbox.owner_user_id = $2 AS is_owner
             FROM mailboxes AS mailbox
             JOIN domains AS domain
               ON domain.id = mailbox.domain_id AND domain.tenant_id = mailbox.tenant_id
             JOIN tenants AS tenant ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
             JOIN tenant_memberships AS membership
               ON membership.tenant_id = mailbox.tenant_id AND membership.user_id = $2
             WHERE mailbox.tenant_id = $1 AND mailbox.status = 'active'
               AND mailbox.deleted_at IS NULL
               AND (mailbox.owner_user_id = $2 OR EXISTS (
                    SELECT 1 FROM mailbox_permissions AS permission
                    WHERE permission.tenant_id = mailbox.tenant_id
                      AND permission.mailbox_id = mailbox.id AND permission.user_id = $2
                      AND permission.permission IN ('read', 'read_write', 'manage')
               ))
         ), folder_paths AS (
             SELECT folder.id, folder.mailbox_id, folder.parent_id, folder.name,
                    folder.special_use, folder.uid_validity, folder.uid_next,
                    folder.name::TEXT AS path
             FROM mailbox_folders AS folder
             WHERE folder.parent_id IS NULL
             UNION ALL
             SELECT child.id, child.mailbox_id, child.parent_id, child.name,
                    child.special_use, child.uid_validity, child.uid_next,
                    parent.path || '/' || child.name
             FROM mailbox_folders AS child
             JOIN folder_paths AS parent
               ON parent.mailbox_id = child.mailbox_id AND parent.id = child.parent_id
         )
         SELECT accessible.mailbox_id, accessible.address, accessible.is_owner,
                folder_paths.id AS folder_id, folder_paths.name, folder_paths.special_use,
                folder_paths.uid_validity, folder_paths.uid_next, folder_paths.path
         FROM accessible
         JOIN folder_paths ON folder_paths.mailbox_id = accessible.mailbox_id
         ORDER BY accessible.address, folder_paths.path",
    )
    .bind(identity.tenant_id)
    .bind(identity.user_id)
    .fetch_all(pool)
    .await?;
    let default_mailbox = default_mailbox_id(pool, identity).await?;
    rows.iter()
        .map(|row| {
            let mailbox_id: Uuid = row.try_get("mailbox_id")?;
            let address: String = row.try_get("address")?;
            let path: String = row.try_get("path")?;
            let is_owner: bool = row.try_get("is_owner")?;
            let rendered = if Some(mailbox_id) == default_mailbox && is_owner {
                if path == "INBOX" {
                    "INBOX".to_owned()
                } else {
                    format!("INBOX/{path}")
                }
            } else {
                format!("{address}/{path}")
            };
            Ok((
                rendered,
                row.try_get("special_use")?,
                row.try_get("folder_id")?,
                mailbox_id,
                row.try_get("uid_validity")?,
                row.try_get("uid_next")?,
            ))
        })
        .collect()
}

pub(super) async fn default_mailbox_id(
    pool: &PgPool,
    identity: &Identity,
) -> Result<Option<Uuid>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT mailbox.id, mailbox.local_part || '@' || domain.domain AS address
         FROM mailboxes AS mailbox
         JOIN domains AS domain ON domain.id = mailbox.domain_id AND domain.tenant_id = mailbox.tenant_id
         JOIN tenants AS tenant ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
         JOIN users AS account ON account.id = $2 AND account.status = 'active'
         JOIN tenant_memberships AS membership
           ON membership.tenant_id = mailbox.tenant_id AND membership.user_id = account.id
         WHERE mailbox.tenant_id = $1 AND mailbox.owner_user_id = $2
           AND mailbox.status = 'active' AND mailbox.deleted_at IS NULL
         ORDER BY mailbox.created_at, mailbox.id",
    )
    .bind(identity.tenant_id)
    .bind(identity.user_id)
    .fetch_all(pool)
    .await?;
    let by_name = rows
        .iter()
        .find(|row| {
            row.try_get::<String, _>("address")
                .is_ok_and(|value| value.eq_ignore_ascii_case(&identity.credential_username))
        })
        .or_else(|| rows.first());
    by_name.map(|row| row.try_get("id")).transpose()
}

pub(super) async fn resolve_folder(
    pool: &PgPool,
    identity: &Identity,
    name: &str,
) -> Result<Option<(Uuid, Uuid, String, i64, i64, bool)>, sqlx::Error> {
    if name.is_empty() || name.len() > 255 || name.chars().any(char::is_control) {
        return Ok(None);
    }
    let name = name.replace('\\', "/");
    let (mailbox_id, path, is_owner) = if name.eq_ignore_ascii_case("INBOX")
        || name.to_ascii_uppercase().starts_with("INBOX/")
    {
        let Some(mailbox_id) = default_mailbox_id(pool, identity).await? else {
            return Ok(None);
        };
        let path = if name.eq_ignore_ascii_case("INBOX") {
            "INBOX".to_owned()
        } else {
            name[6..].to_owned()
        };
        (mailbox_id, path, true)
    } else {
        let (address, path) = name
            .split_once('/')
            .map_or((name.as_str(), "INBOX"), |(address, path)| (address, path));
        let row = sqlx::query(
                "SELECT mailbox.id, mailbox.owner_user_id = $3 AS is_owner
             FROM mailboxes AS mailbox
             JOIN domains AS domain ON domain.id = mailbox.domain_id
                 JOIN tenants AS tenant ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
                 JOIN users AS account ON account.id = $3 AND account.status = 'active'
                 JOIN tenant_memberships AS membership
                   ON membership.tenant_id = mailbox.tenant_id AND membership.user_id = account.id
                 WHERE mailbox.tenant_id = $1 AND mailbox.status = 'active'
               AND mailbox.deleted_at IS NULL
               AND lower(mailbox.local_part || '@' || domain.domain) = lower($2)
               AND (mailbox.owner_user_id = $3 OR EXISTS (
                    SELECT 1 FROM mailbox_permissions AS permission
                    WHERE permission.mailbox_id = mailbox.id AND permission.user_id = $3
                      AND permission.permission IN ('read', 'read_write', 'manage')
               ))
             LIMIT 1",
            )
            .bind(identity.tenant_id)
            .bind(address)
            .bind(identity.user_id)
            .fetch_optional(pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        (
            row.try_get("id")?,
            path.to_owned(),
            row.try_get("is_owner")?,
        )
    };
    let folder = sqlx::query(
        "WITH RECURSIVE paths AS (
             SELECT id, mailbox_id, parent_id, name, special_use, uid_validity, uid_next,
                    name::TEXT AS path
             FROM mailbox_folders WHERE mailbox_id = $1 AND parent_id IS NULL
             UNION ALL
             SELECT child.id, child.mailbox_id, child.parent_id, child.name,
                    child.special_use, child.uid_validity, child.uid_next,
                    parent.path || '/' || child.name
             FROM mailbox_folders AS child
             JOIN paths AS parent ON parent.mailbox_id = child.mailbox_id AND parent.id = child.parent_id
         )
         SELECT id, name, uid_validity, uid_next
         FROM paths WHERE lower(path) = lower($2) LIMIT 1",
    )
    .bind(mailbox_id)
    .bind(&path)
    .fetch_optional(pool)
    .await?;
    folder
        .map(|row| {
            Ok((
                mailbox_id,
                row.try_get("id")?,
                row.try_get("name")?,
                row.try_get("uid_validity")?,
                row.try_get("uid_next")?,
                is_owner,
            ))
        })
        .transpose()
}

pub(super) fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
