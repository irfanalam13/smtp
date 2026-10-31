use super::*;

pub(super) async fn mutate_folder<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    let Some(identity) = &connection.identity else {
        return write_tagged(stream, &command.tag, "NO", "authenticate first").await;
    };
    let expected = if command.name == "RENAME" { 2 } else { 1 };
    if command.arguments.len() != expected {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "invalid folder command arguments",
        )
        .await;
    }
    let old_name = command.arguments[0].value().unwrap_or_default();
    if command.name == "CREATE" {
        if old_name.is_empty() || old_name.len() > 255 || old_name.chars().any(char::is_control) {
            return write_tagged(stream, &command.tag, "BAD", "invalid folder name").await;
        }
        let (parent_name, path_name) = match old_name.rsplit_once('/') {
            Some((parent, leaf)) if !parent.is_empty() && !leaf.is_empty() => (parent, leaf),
            Some(_) => {
                return write_tagged(stream, &command.tag, "BAD", "invalid folder name").await;
            }
            None => ("INBOX", old_name),
        };
        let Some(parent) = resolve_folder(&state.pool, identity, parent_name)
            .await
            .map_err(io::Error::other)?
        else {
            return write_tagged(stream, &command.tag, "NO", "parent folder unavailable").await;
        };
        let can_manage: bool = parent.5
            || sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM mailbox_permissions
                 WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3
                   AND permission = 'manage')",
            )
            .bind(identity.tenant_id)
            .bind(parent.0)
            .bind(identity.user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(io::Error::other)?;
        if !can_manage {
            return write_tagged(
                stream,
                &command.tag,
                "NO",
                "mailbox management permission required",
            )
            .await;
        }
        let uid_validity: i64 = sqlx::query_scalar("SELECT nextval('mailbox_uid_validity_seq')")
            .fetch_one(&state.pool)
            .await
            .map_err(io::Error::other)?;
        let result = sqlx::query(
            "INSERT INTO mailbox_folders (id, mailbox_id, parent_id, name, uid_validity)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(parent.0)
        .bind(parent.1)
        .bind(path_name)
        .bind(uid_validity)
        .execute(&state.pool)
        .await;
        return match result {
            Ok(_) => write_tagged(stream, &command.tag, "OK", "CREATE completed").await,
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                write_tagged(stream, &command.tag, "NO", "folder already exists").await
            }
            Err(error) => Err(io::Error::other(error)),
        };
    }
    let Some(folder) = resolve_folder(&state.pool, identity, old_name)
        .await
        .map_err(io::Error::other)?
    else {
        return write_tagged(stream, &command.tag, "NO", "mailbox unavailable").await;
    };
    let can_manage: bool = folder.5
        || sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM mailbox_permissions
         WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3 AND permission = 'manage')",
        )
        .bind(identity.tenant_id)
        .bind(folder.0)
        .bind(identity.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(io::Error::other)?;
    if !can_manage {
        return write_tagged(
            stream,
            &command.tag,
            "NO",
            "mailbox management permission required",
        )
        .await;
    }
    match command.name.as_str() {
        "DELETE" => {
            let (parent_id, special_use): (Option<Uuid>, Option<String>) = sqlx::query_as(
                "SELECT parent_id, special_use FROM mailbox_folders WHERE mailbox_id = $1 AND id = $2",
            )
            .bind(folder.0)
            .bind(folder.1)
            .fetch_one(&state.pool).await.map_err(io::Error::other)?;
            if parent_id.is_none() || special_use.is_some() {
                return write_tagged(
                    stream,
                    &command.tag,
                    "NO",
                    "cannot delete INBOX or a special-use folder",
                )
                .await;
            }
            let mut transaction = state.pool.begin().await.map_err(io::Error::other)?;
            let removed_bytes: i64 = sqlx::query_scalar(
                "WITH RECURSIVE tree AS (
                     SELECT id FROM mailbox_folders WHERE mailbox_id = $1 AND id = $2
                     UNION ALL
                     SELECT child.id FROM mailbox_folders child JOIN tree ON child.parent_id = tree.id
                 )
                 SELECT COALESCE(SUM(message.raw_size), 0)::BIGINT
                 FROM mailbox_messages message JOIN tree ON tree.id = message.folder_id",
            )
            .bind(folder.0)
            .bind(folder.1)
            .fetch_one(&mut *transaction).await.map_err(io::Error::other)?;
            sqlx::query(
                "UPDATE mailboxes SET used_bytes = GREATEST(used_bytes - $1, 0) WHERE id = $2",
            )
            .bind(removed_bytes)
            .bind(folder.0)
            .execute(&mut *transaction)
            .await
            .map_err(io::Error::other)?;
            sqlx::query("DELETE FROM mailbox_folders WHERE mailbox_id = $1 AND id = $2")
                .bind(folder.0)
                .bind(folder.1)
                .execute(&mut *transaction)
                .await
                .map_err(io::Error::other)?;
            transaction.commit().await.map_err(io::Error::other)?;
            write_tagged(stream, &command.tag, "OK", "DELETE completed").await
        }
        "RENAME" => {
            let new_name = command.arguments[1].value().unwrap_or_default();
            if new_name.is_empty() || new_name.len() > 255 || new_name.chars().any(char::is_control)
            {
                return write_tagged(stream, &command.tag, "BAD", "invalid folder name").await;
            }
            if new_name.eq_ignore_ascii_case("INBOX") || folder.2.eq_ignore_ascii_case("INBOX") {
                return write_tagged(
                    stream,
                    &command.tag,
                    "NO",
                    "renaming INBOX is not supported",
                )
                .await;
            }
            let name = new_name.rsplit('/').next().unwrap_or(new_name);
            let result = sqlx::query(
                "UPDATE mailbox_folders SET name = $1, updated_at = NOW()
                 WHERE mailbox_id = $2 AND id = $3",
            )
            .bind(name)
            .bind(folder.0)
            .bind(folder.1)
            .execute(&state.pool)
            .await;
            match result {
                Ok(result) if result.rows_affected() == 1 => {
                    write_tagged(stream, &command.tag, "OK", "RENAME completed").await
                }
                Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                    write_tagged(stream, &command.tag, "NO", "folder already exists").await
                }
                Err(error) => Err(io::Error::other(error)),
                _ => write_tagged(stream, &command.tag, "NO", "mailbox unavailable").await,
            }
        }
        _ => write_tagged(stream, &command.tag, "BAD", "unsupported folder command").await,
    }
}

pub(super) async fn expunge<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    let Some((identity, selected)) =
        selected_context(stream, connection, state, &command.tag, true).await?
    else {
        return Ok(());
    };
    state
        .mailbox_store
        .expunge_for_user(identity.tenant_id, identity.user_id, selected.folder_id)
        .await
        .map_err(io::Error::other)?;
    notify_selected(stream, connection, state).await?;
    write_tagged(stream, &command.tag, "OK", "EXPUNGE completed").await
}

pub(super) async fn uid_expunge<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
    arguments: &[Token],
) -> io::Result<()> {
    let Some((_identity, selected)) =
        selected_context(stream, connection, state, &command.tag, true).await?
    else {
        return Ok(());
    };
    let Some(set) = arguments.first().and_then(Token::value) else {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "UID EXPUNGE requires a UID set",
        )
        .await;
    };
    let messages = load_messages(&state.pool, selected.mailbox_id, selected.folder_id)
        .await
        .map_err(io::Error::other)?;
    let wanted = match sequence_members(set, messages.len(), true, &messages) {
        Ok(wanted) => wanted,
        Err(_) => {
            return write_tagged(stream, &command.tag, "BAD", "invalid UID sequence set").await;
        }
    };
    let ids = wanted
        .iter()
        .map(|index| messages[*index - 1].id)
        .collect::<Vec<_>>();
    let mut transaction = state.pool.begin().await.map_err(io::Error::other)?;
    let removed = sqlx::query(
        "DELETE FROM mailbox_messages
         WHERE mailbox_id = $1 AND folder_id = $2 AND id = ANY($3)
           AND '\\Deleted' = ANY(flags)
         RETURNING raw_size",
    )
    .bind(selected.mailbox_id)
    .bind(selected.folder_id)
    .bind(&ids)
    .fetch_all(&mut *transaction)
    .await
    .map_err(io::Error::other)?;
    let removed_bytes = removed
        .iter()
        .map(|row| row.try_get::<i64, _>("raw_size"))
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?
        .into_iter()
        .sum::<i64>();
    sqlx::query("UPDATE mailboxes SET used_bytes = GREATEST(used_bytes - $1, 0) WHERE id = $2")
        .bind(removed_bytes)
        .bind(selected.mailbox_id)
        .execute(&mut *transaction)
        .await
        .map_err(io::Error::other)?;
    transaction.commit().await.map_err(io::Error::other)?;
    notify_selected(stream, connection, state).await?;
    write_tagged(stream, &command.tag, "OK", "UID EXPUNGE completed").await
}
