use super::*;

pub(super) async fn status<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    let Some(identity) = &connection.identity else {
        return write_tagged(stream, &command.tag, "NO", "authenticate first").await;
    };
    let Some(name) = command.arguments.first().and_then(Token::value) else {
        return write_tagged(stream, &command.tag, "BAD", "STATUS requires a mailbox").await;
    };
    let Some(folder) = resolve_folder(&state.pool, identity, name)
        .await
        .map_err(io::Error::other)?
    else {
        return write_tagged(stream, &command.tag, "NO", "mailbox unavailable").await;
    };
    let counts = sqlx::query(
        "SELECT COUNT(*)::BIGINT AS messages,
                COUNT(*) FILTER (WHERE '\\Seen' = ANY(flags))::BIGINT AS seen,
                COUNT(*) FILTER (WHERE NOT ('\\Seen' = ANY(flags)))::BIGINT AS unseen
         FROM mailbox_messages WHERE mailbox_id = $1 AND folder_id = $2 AND deleted_at IS NULL",
    )
    .bind(folder.0)
    .bind(folder.1)
    .fetch_one(&state.pool)
    .await
    .map_err(io::Error::other)?;
    let messages: i64 = counts.try_get("messages").map_err(io::Error::other)?;
    let seen: i64 = counts.try_get("seen").map_err(io::Error::other)?;
    let unseen: i64 = counts.try_get("unseen").map_err(io::Error::other)?;
    let items = command
        .arguments
        .iter()
        .skip(1)
        .filter_map(Token::value)
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>();
    if items.is_empty()
        || items.iter().any(|item| {
            !matches!(
                item.as_str(),
                "MESSAGES" | "RECENT" | "UIDNEXT" | "UIDVALIDITY" | "UNSEEN"
            )
        })
    {
        return write_tagged(stream, &command.tag, "BAD", "unsupported STATUS item").await;
    }
    let values = items
        .iter()
        .map(|item| match item.as_str() {
            "MESSAGES" => format!("MESSAGES {messages}"),
            "RECENT" => "RECENT 0".into(),
            "UIDNEXT" => format!("UIDNEXT {}", folder.4),
            "UIDVALIDITY" => format!("UIDVALIDITY {}", folder.3),
            "UNSEEN" => format!("UNSEEN {}", unseen + 1),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>()
        .join(" ");
    stream
        .write_all(format!("* STATUS {} ({values})\r\n", quote(name)).as_bytes())
        .await?;
    let _ = seen;
    write_tagged(stream, &command.tag, "OK", "STATUS completed").await
}

pub(super) async fn select_mailbox<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
    read_only: bool,
) -> io::Result<()> {
    let Some(identity) = &connection.identity else {
        return write_tagged(stream, &command.tag, "NO", "authenticate first").await;
    };
    let Some(name) = command.arguments.first().and_then(Token::value) else {
        return write_tagged(stream, &command.tag, "BAD", "SELECT requires a mailbox").await;
    };
    let Some(folder) = resolve_folder(&state.pool, identity, name)
        .await
        .map_err(io::Error::other)?
    else {
        return write_tagged(stream, &command.tag, "NO", "mailbox unavailable").await;
    };
    let writable = if folder.5 {
        true
    } else {
        sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM mailbox_permissions
                 WHERE tenant_id = $1 AND mailbox_id = $2 AND user_id = $3
                   AND permission IN ('read_write', 'manage')
             )",
        )
        .bind(identity.tenant_id)
        .bind(folder.0)
        .bind(identity.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(io::Error::other)?
    };
    if !read_only && !writable {
        return write_tagged(stream, &command.tag, "NO", "mailbox is read-only").await;
    }
    let messages = load_messages(&state.pool, folder.0, folder.1)
        .await
        .map_err(io::Error::other)?;
    let unseen = messages
        .iter()
        .position(|message| !message.flags.iter().any(|flag| flag == "\\Seen"))
        .map_or(0, |index| index + 1);
    let mut flags = vec!["\\Seen", "\\Answered", "\\Flagged", "\\Deleted", "\\Draft"];
    if writable && !read_only {
        stream
            .write_all(format!("* FLAGS ({})\r\n", flags.join(" ")).as_bytes())
            .await?;
    } else {
        flags.retain(|flag| *flag != "\\Recent");
        stream
            .write_all(format!("* FLAGS ({})\r\n", flags.join(" ")).as_bytes())
            .await?;
    }
    stream.write_all(format!(
        "* {} EXISTS\r\n* 0 RECENT\r\n* OK [UIDVALIDITY {}] UIDs valid\r\n* OK [UIDNEXT {}] Predicted next UID\r\n",
        messages.len(), folder.3, folder.4
    ).as_bytes()).await?;
    if unseen > 0 {
        stream
            .write_all(format!("* OK [UNSEEN {unseen}] First unseen message\r\n").as_bytes())
            .await?;
    }
    let mode = if read_only || !writable {
        "READ-ONLY"
    } else {
        "READ-WRITE"
    };
    write_tagged(
        stream,
        &command.tag,
        "OK",
        &format!("[{mode}] SELECT completed"),
    )
    .await?;
    connection.selected = Some(Selected {
        mailbox_id: folder.0,
        folder_id: folder.1,
        read_only: read_only || !writable,
        snapshot: messages.into_iter().map(|message| message.id).collect(),
        change_cursor: sqlx::query_scalar(
            "SELECT COALESCE(MAX(token), 0) FROM mailbox_change_log WHERE mailbox_id = $1",
        )
        .bind(folder.0)
        .fetch_one(&state.pool)
        .await
        .map_err(io::Error::other)?,
    });
    Ok(())
}

pub(super) async fn close_mailbox<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    if connection.selected.is_none() {
        return write_tagged(stream, &command.tag, "BAD", "no mailbox selected").await;
    }
    if let Some(selected) = connection
        .selected
        .clone()
        .filter(|selected| !selected.read_only)
    {
        let Some(identity) = &connection.identity else {
            return write_tagged(stream, &command.tag, "NO", "authenticate first").await;
        };
        state
            .mailbox_store
            .expunge_for_user(identity.tenant_id, identity.user_id, selected.folder_id)
            .await
            .map_err(io::Error::other)?;
    }
    connection.selected = None;
    write_tagged(stream, &command.tag, "OK", "CLOSE completed").await
}

pub(super) async fn unselect_mailbox<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    command: &CommandLine,
) -> io::Result<()> {
    if connection.selected.take().is_none() {
        return write_tagged(stream, &command.tag, "BAD", "no mailbox selected").await;
    }
    write_tagged(stream, &command.tag, "OK", "UNSELECT completed").await
}

pub(super) async fn load_messages(
    pool: &PgPool,
    mailbox_id: Uuid,
    folder_id: Uuid,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, uid, flags, raw_message, internal_date
         FROM mailbox_messages
         WHERE mailbox_id = $1 AND folder_id = $2 AND deleted_at IS NULL
         ORDER BY uid",
    )
    .bind(mailbox_id)
    .bind(folder_id)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(MessageRow {
                id: row.try_get("id")?,
                uid: row.try_get("uid")?,
                flags: row.try_get("flags")?,
                raw_message: row.try_get("raw_message")?,
                internal_date: row.try_get("internal_date")?,
            })
        })
        .collect()
}

pub(super) async fn notify_selected<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
) -> io::Result<()> {
    let Some(selected) = connection.selected.as_mut() else {
        return Ok(());
    };
    let changes = sqlx::query(
        "SELECT token, message_id, change_kind, payload
         FROM mailbox_change_log
         WHERE mailbox_id = $1 AND token > $2
         ORDER BY token LIMIT 500",
    )
    .bind(selected.mailbox_id)
    .bind(selected.change_cursor)
    .fetch_all(&state.pool)
    .await
    .map_err(io::Error::other)?;
    for change in &changes {
        let token: i64 = change.try_get("token").map_err(io::Error::other)?;
        selected.change_cursor = token;
        let kind: String = change.try_get("change_kind").map_err(io::Error::other)?;
        if kind != "upsert" {
            continue;
        }
        let payload: serde_json::Value = change.try_get("payload").map_err(io::Error::other)?;
        if payload
            .get("folder_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            != Some(selected.folder_id)
        {
            continue;
        }
        let message_id: Uuid = change.try_get("message_id").map_err(io::Error::other)?;
        let Some((index, uid, flags)) = sqlx::query(
            "SELECT message.flags, message.uid,
                    (SELECT COUNT(*) + 1
                     FROM mailbox_messages AS prior
                     WHERE prior.mailbox_id = message.mailbox_id
                       AND prior.folder_id = message.folder_id
                       AND prior.deleted_at IS NULL AND prior.uid < message.uid
                    )::BIGINT AS sequence
             FROM mailbox_messages AS message
             WHERE message.mailbox_id = $1 AND message.folder_id = $2
               AND message.id = $3 AND message.deleted_at IS NULL",
        )
        .bind(selected.mailbox_id)
        .bind(selected.folder_id)
        .bind(message_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(io::Error::other)?
        .map(|row| {
            Ok::<_, sqlx::Error>((
                row.try_get::<i64, _>("sequence")?,
                row.try_get::<i64, _>("uid")?,
                row.try_get::<Vec<String>, _>("flags")?,
            ))
        })
        .transpose()
        .map_err(io::Error::other)?
        else {
            continue;
        };
        stream
            .write_all(
                format!(
                    "* {index} FETCH (UID {uid} FLAGS ({}))\r\n",
                    flags.join(" ")
                )
                .as_bytes(),
            )
            .await?;
    }
    let messages = load_messages(&state.pool, selected.mailbox_id, selected.folder_id)
        .await
        .map_err(io::Error::other)?;
    let current = messages
        .iter()
        .map(|message| message.id)
        .collect::<Vec<_>>();
    for old_position in (0..selected.snapshot.len()).rev() {
        if !current.contains(&selected.snapshot[old_position]) {
            stream
                .write_all(format!("* {} EXPUNGE\r\n", old_position + 1).as_bytes())
                .await?;
        }
    }
    if current.len() != selected.snapshot.len() {
        stream
            .write_all(format!("* {} EXISTS\r\n", current.len()).as_bytes())
            .await?;
    }
    selected.snapshot = current;
    Ok(())
}

pub(super) async fn selected_context<'a, S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut BufStream<S>,
    connection: &'a Connection,
    state: &AppState,
    tag: &str,
    write: bool,
) -> io::Result<Option<(&'a Identity, &'a Selected)>> {
    let (Some(identity), Some(selected)) = (&connection.identity, &connection.selected) else {
        write_tagged(stream, tag, "NO", "select a mailbox first").await?;
        return Ok(None);
    };
    if selected.read_only && write {
        write_tagged(stream, tag, "NO", "mailbox is read-only").await?;
        return Ok(None);
    }
    let allowed: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1
             FROM mailboxes AS mailbox
             JOIN tenants AS tenant
               ON tenant.id = mailbox.tenant_id AND tenant.status = 'active'
             JOIN users AS account ON account.id = $2 AND account.status = 'active'
             JOIN tenant_memberships AS membership
               ON membership.tenant_id = mailbox.tenant_id AND membership.user_id = account.id
             WHERE mailbox.id = $3 AND mailbox.tenant_id = $1 AND mailbox.status = 'active'
               AND mailbox.deleted_at IS NULL
               AND (mailbox.owner_user_id = account.id OR EXISTS (
                    SELECT 1 FROM mailbox_permissions AS permission
                    WHERE permission.tenant_id = mailbox.tenant_id
                      AND permission.mailbox_id = mailbox.id
                      AND permission.user_id = account.id
                      AND permission.permission IN (
                          'read', 'read_write', 'manage'
                      )
                      AND (NOT $4 OR permission.permission IN ('read_write', 'manage'))
               ))
         )",
    )
    .bind(identity.tenant_id)
    .bind(identity.user_id)
    .bind(selected.mailbox_id)
    .bind(write)
    .fetch_one(&state.pool)
    .await
    .map_err(io::Error::other)?;
    if !allowed {
        write_tagged(stream, tag, "NO", "mailbox access revoked").await?;
        return Ok(None);
    }
    Ok(Some((identity, selected)))
}
