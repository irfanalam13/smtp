use super::*;

pub(super) async fn copy_or_move<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
    move_messages: bool,
) -> io::Result<()> {
    copy_or_move_args(
        stream,
        connection,
        state,
        command,
        &command.arguments,
        false,
        move_messages,
    )
    .await
}

pub(super) async fn copy_or_move_args<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
    arguments: &[Token],
    uid_mode: bool,
    move_messages: bool,
) -> io::Result<()> {
    let Some((identity, source)) =
        selected_context(stream, connection, state, &command.tag, move_messages).await?
    else {
        return Ok(());
    };
    if arguments.len() != 2 {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "COPY/MOVE requires sequence and destination",
        )
        .await;
    }
    let set = arguments[0].value().unwrap_or_default();
    let destination_name = arguments[1].value().unwrap_or_default();
    let Some(destination) = resolve_folder(&state.pool, identity, destination_name)
        .await
        .map_err(io::Error::other)?
    else {
        return write_tagged(
            stream,
            &command.tag,
            "NO",
            "destination mailbox unavailable",
        )
        .await;
    };
    let can_write: bool = destination.5
        || sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM mailbox_permissions
         WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3
           AND permission IN ('read_write', 'manage'))",
        )
        .bind(identity.tenant_id)
        .bind(destination.0)
        .bind(identity.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(io::Error::other)?;
    if !can_write {
        return write_tagged(
            stream,
            &command.tag,
            "NO",
            "destination mailbox is read-only",
        )
        .await;
    }
    let source_messages = load_messages(&state.pool, source.mailbox_id, source.folder_id)
        .await
        .map_err(io::Error::other)?;
    let selected_indexes =
        match sequence_members(set, source_messages.len(), uid_mode, &source_messages) {
            Ok(indexes) => indexes,
            Err(_) => {
                return write_tagged(stream, &command.tag, "BAD", "invalid message sequence set")
                    .await;
            }
        };
    if selected_indexes.is_empty() {
        return write_tagged(stream, &command.tag, "OK", "COPY/MOVE completed").await;
    }
    let destination_folder = destination.1;
    let mut transaction = state.pool.begin().await.map_err(io::Error::other)?;
    let (first_mailbox, second_mailbox) = if source.mailbox_id <= destination.0 {
        (source.mailbox_id, destination.0)
    } else {
        (destination.0, source.mailbox_id)
    };
    sqlx::query("SELECT id FROM mailboxes WHERE id = $1 FOR UPDATE")
        .bind(first_mailbox)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(io::Error::other)?;
    if first_mailbox != second_mailbox {
        sqlx::query("SELECT id FROM mailboxes WHERE id = $1 FOR UPDATE")
            .bind(second_mailbox)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(io::Error::other)?;
    }
    let mut destination_uids = Vec::new();
    let mut source_uids = Vec::new();
    for one_based in selected_indexes {
        let message = &source_messages[one_based - 1];
        source_uids.push(message.uid);
        let uid: i64 = sqlx::query_scalar(
            "UPDATE mailbox_folders
             SET uid_next = uid_next + 1, updated_at = NOW()
             WHERE id = $1 AND mailbox_id = $2 AND uid_next < 9223372036854775807
             RETURNING uid_next - 1",
        )
        .bind(destination_folder)
        .bind(destination.0)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::other("destination UID allocation failed"))?;
        if move_messages && source.mailbox_id == destination.0 {
            sqlx::query(
                "UPDATE mailbox_messages SET folder_id = $1, uid = $2
                 WHERE mailbox_id = $3 AND id = $4",
            )
            .bind(destination_folder)
            .bind(uid)
            .bind(source.mailbox_id)
            .bind(message.id)
            .execute(&mut *transaction)
            .await
            .map_err(io::Error::other)?;
        } else {
            let reserved = sqlx::query(
                "UPDATE mailboxes SET used_bytes = used_bytes + $1
                 WHERE id = $2 AND status = 'active'
                   AND used_bytes <= quota_bytes - $1",
            )
            .bind(message.raw_message.len() as i64)
            .bind(destination.0)
            .execute(&mut *transaction)
            .await
            .map_err(io::Error::other)?;
            if reserved.rows_affected() != 1 {
                transaction.rollback().await.map_err(io::Error::other)?;
                return write_tagged(
                    stream,
                    &command.tag,
                    "NO",
                    "destination mailbox quota exceeded",
                )
                .await;
            }
            sqlx::query(
                "INSERT INTO mailbox_messages
                 (id, mailbox_id, folder_id, uid, raw_message, raw_size, flags,
                  internal_date, message_schema_version, parsed_metadata)
                 SELECT gen_random_uuid(), $1, $2, $3, raw_message, raw_size, flags,
                        internal_date, message_schema_version, parsed_metadata
                 FROM mailbox_messages WHERE mailbox_id = $4 AND id = $5",
            )
            .bind(destination.0)
            .bind(destination_folder)
            .bind(uid)
            .bind(source.mailbox_id)
            .bind(message.id)
            .execute(&mut *transaction)
            .await
            .map_err(io::Error::other)?;
            if move_messages {
                sqlx::query(
                    "UPDATE mailboxes SET used_bytes = GREATEST(used_bytes - $1, 0)
                     WHERE id = $2",
                )
                .bind(message.raw_message.len() as i64)
                .bind(source.mailbox_id)
                .execute(&mut *transaction)
                .await
                .map_err(io::Error::other)?;
                sqlx::query("DELETE FROM mailbox_messages WHERE mailbox_id = $1 AND id = $2")
                    .bind(source.mailbox_id)
                    .bind(message.id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(io::Error::other)?;
            }
        }
        destination_uids.push(uid);
    }
    transaction.commit().await.map_err(io::Error::other)?;
    if move_messages {
        notify_selected(stream, connection, state).await?;
    }
    let response = format!(
        "[COPYUID {} {} {}] {} completed",
        destination.4,
        compact_sequence_set(&source_uids),
        compact_sequence_set(&destination_uids),
        if move_messages { "MOVE" } else { "COPY" }
    );
    write_tagged(stream, &command.tag, "OK", &response).await
}

pub(super) fn compact_sequence_set(values: &[i64]) -> String {
    values
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
