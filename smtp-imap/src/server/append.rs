use super::*;

pub(super) async fn append_message<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
    original_line: &str,
) -> io::Result<()> {
    let Some(identity) = &connection.identity else {
        return write_tagged(stream, &command.tag, "NO", "authenticate first").await;
    };
    let Some((length, non_synchronizing)) =
        parse_literal_marker(original_line).map_err(io::Error::other)?
    else {
        return write_tagged(stream, &command.tag, "BAD", "APPEND requires a literal").await;
    };
    if non_synchronizing {
        return write_tagged(stream, &command.tag, "NO", "LITERAL+ is not supported").await;
    }
    let Some(mailbox_name) = command.arguments.first().and_then(Token::value) else {
        return write_tagged(stream, &command.tag, "BAD", "APPEND requires a mailbox").await;
    };
    let Some(marker_index) = command
        .arguments
        .iter()
        .position(|token| matches!(token, Token::Atom(value) if value.starts_with('{')))
    else {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "APPEND literal marker is missing",
        )
        .await;
    };
    if marker_index + 1 != command.arguments.len() {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "APPEND literal marker must be the final argument",
        )
        .await;
    }
    let (flags, internal_date) = match parse_append_options(&command.arguments[1..marker_index]) {
        Ok(options) => options,
        Err(()) => {
            return write_tagged(stream, &command.tag, "BAD", "invalid APPEND options").await;
        }
    };
    let Some(destination) = resolve_folder(&state.pool, identity, mailbox_name)
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
    stream.write_all(b"+ Ready for literal data\r\n").await?;
    stream.flush().await?;
    let mut raw = vec![0_u8; length];
    timeout(COMMAND_TIMEOUT, stream.read_exact(&mut raw))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "APPEND literal timed out"))??;
    let mut ending = [0_u8; 2];
    timeout(COMMAND_TIMEOUT, stream.read_exact(&mut ending))
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "APPEND literal terminator timed out",
            )
        })??;
    if ending != *b"\r\n" {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "APPEND literal must end with CRLF",
        )
        .await;
    }
    let appended = state
        .mailbox_store
        .append_for_user(
            identity.tenant_id,
            identity.user_id,
            destination.1,
            &raw,
            &flags,
            internal_date.unwrap_or_else(Utc::now),
        )
        .await;
    match appended {
        Ok(message) => {
            write_tagged(
                stream,
                &command.tag,
                "OK",
                &format!(
                    "[APPENDUID {} {}] APPEND completed",
                    destination.3, message.uid
                ),
            )
            .await
        }
        Err(error) => {
            tracing::warn!(%error, "IMAP APPEND failed");
            write_tagged(stream, &command.tag, "NO", "APPEND failed").await
        }
    }
}
