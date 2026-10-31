use super::*;

pub(super) async fn store_flags_args<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
    arguments: &[Token],
    uid_mode: bool,
) -> io::Result<()> {
    let Some((identity, selected)) =
        selected_context(stream, connection, state, &command.tag, true).await?
    else {
        return Ok(());
    };
    if arguments.len() < 3 {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "STORE requires sequence, operation and flags",
        )
        .await;
    }
    let sequence = arguments[0].value().unwrap_or_default();
    let operation = arguments[1]
        .value()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let parsed_flags = parse_flag_list(&arguments[2..]);
    let Ok(parsed_flags) = parsed_flags else {
        return write_tagged(stream, &command.tag, "BAD", "invalid flag list").await;
    };
    if !matches!(
        operation.as_str(),
        "FLAGS" | "+FLAGS" | "-FLAGS" | "FLAGS.SILENT" | "+FLAGS.SILENT" | "-FLAGS.SILENT"
    ) {
        return write_tagged(stream, &command.tag, "BAD", "unsupported STORE operation").await;
    }
    let messages = load_messages(&state.pool, selected.mailbox_id, selected.folder_id)
        .await
        .map_err(io::Error::other)?;
    let wanted = match sequence_members(sequence, messages.len(), uid_mode, &messages) {
        Ok(wanted) => wanted,
        Err(_) => {
            return write_tagged(stream, &command.tag, "BAD", "invalid message sequence set").await;
        }
    };
    for (index, message) in messages.iter().enumerate() {
        if !wanted.contains(&(index + 1)) {
            continue;
        }
        let mut flags = message
            .flags
            .iter()
            .filter_map(|flag| parse_flag(flag))
            .collect::<Vec<_>>();
        match operation.trim_end_matches(".SILENT") {
            "FLAGS" => flags = parsed_flags.clone(),
            "+FLAGS" => flags.extend(parsed_flags.iter().cloned()),
            "-FLAGS" => flags.retain(|flag| !parsed_flags.contains(flag)),
            _ => unreachable!(),
        }
        let flags = deduplicate_flags(flags);
        state
            .mailbox_store
            .set_message_flags(
                identity.tenant_id,
                identity.user_id,
                selected.mailbox_id,
                message.id,
                &flags,
            )
            .await
            .map_err(io::Error::other)?;
        if !operation.ends_with(".SILENT") {
            stream
                .write_all(
                    format!(
                        "* {} FETCH (FLAGS ({}))\r\n",
                        index + 1,
                        flags
                            .iter()
                            .map(|flag| flag.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                    .as_bytes(),
                )
                .await?;
        }
    }
    write_tagged(stream, &command.tag, "OK", "STORE completed").await
}

pub(super) fn parse_flag_list(tokens: &[Token]) -> Result<Vec<MailboxFlag>, ()> {
    let mut flags = Vec::new();
    let mut in_list = false;
    for token in tokens {
        match token {
            Token::OpenParen => {
                if in_list {
                    return Err(());
                }
                in_list = true;
            }
            Token::CloseParen => in_list = false,
            Token::Atom(flag) | Token::Quoted(flag) => {
                if let Some(flag) = parse_flag(flag) {
                    flags.push(flag);
                } else {
                    return Err(());
                }
                if flags.len() > MAX_FLAGS {
                    return Err(());
                }
            }
        }
    }
    if tokens.iter().any(|token| matches!(token, Token::OpenParen)) && in_list {
        return Err(());
    }
    Ok(deduplicate_flags(flags))
}

pub(super) fn parse_append_options(
    tokens: &[Token],
) -> Result<(Vec<MailboxFlag>, Option<DateTime<Utc>>), ()> {
    let (flags, date_tokens) = if tokens.first() == Some(&Token::OpenParen) {
        let close = tokens
            .iter()
            .position(|token| *token == Token::CloseParen)
            .ok_or(())?;
        if tokens[close + 1..]
            .iter()
            .any(|token| matches!(token, Token::OpenParen | Token::CloseParen))
        {
            return Err(());
        }
        (parse_flag_list(&tokens[..=close])?, &tokens[close + 1..])
    } else {
        (Vec::new(), tokens)
    };
    let date = match date_tokens {
        [] => None,
        [Token::Quoted(value)] => Some(
            DateTime::parse_from_str(value, "%d-%b-%Y %H:%M:%S %z")
                .map_err(|_| ())?
                .with_timezone(&Utc),
        ),
        _ => return Err(()),
    };
    Ok((flags, date))
}

pub(super) fn parse_flag(value: &str) -> Option<MailboxFlag> {
    match value.to_ascii_lowercase().as_str() {
        "\\seen" => Some(MailboxFlag::Seen),
        "\\answered" => Some(MailboxFlag::Answered),
        "\\flagged" => Some(MailboxFlag::Flagged),
        "\\deleted" => Some(MailboxFlag::Deleted),
        "\\draft" => Some(MailboxFlag::Draft),
        "\\recent" => Some(MailboxFlag::Recent),
        _ => None,
    }
}

pub(super) fn deduplicate_flags(flags: Vec<MailboxFlag>) -> Vec<MailboxFlag> {
    let mut unique = Vec::new();
    for flag in flags {
        if !unique.contains(&flag) && flag != MailboxFlag::Recent {
            unique.push(flag);
        }
    }
    unique
}

pub(super) async fn store_flags<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    store_flags_args(
        stream,
        connection,
        state,
        command,
        &command.arguments,
        false,
    )
    .await
}
