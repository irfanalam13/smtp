use super::*;

pub(super) async fn fetch_or_uid<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    let uid_mode = command.name == "UID";
    let (subcommand, arguments) = if uid_mode {
        let Some(Token::Atom(subcommand)) = command.arguments.first() else {
            return write_tagged(stream, &command.tag, "BAD", "UID requires a subcommand").await;
        };
        (subcommand.to_ascii_uppercase(), &command.arguments[1..])
    } else {
        ("FETCH".to_owned(), &command.arguments[..])
    };
    match subcommand.as_str() {
        "FETCH" => fetch(stream, connection, state, command, arguments, uid_mode).await,
        "STORE" => store_flags_args(stream, connection, state, command, arguments, uid_mode).await,
        "SEARCH" => search_args(stream, connection, state, command, arguments, uid_mode).await,
        "COPY" | "MOVE" => {
            copy_or_move_args(
                stream,
                connection,
                state,
                command,
                arguments,
                uid_mode,
                subcommand == "MOVE",
            )
            .await
        }
        "EXPUNGE" if uid_mode => uid_expunge(stream, connection, state, command, arguments).await,
        _ => write_tagged(stream, &command.tag, "BAD", "unsupported UID subcommand").await,
    }
}

pub(super) async fn fetch<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
    arguments: &[Token],
    uid_mode: bool,
) -> io::Result<()> {
    let Some((_identity, selected)) =
        selected_context(stream, connection, state, &command.tag, false).await?
    else {
        return Ok(());
    };
    if arguments.len() < 2 {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "FETCH requires a sequence set and item list",
        )
        .await;
    }
    let sequence_set = arguments[0].value().unwrap_or_default();
    let mut items = parse_fetch_items(&arguments[1..]);
    if items.iter().any(|item| item.eq_ignore_ascii_case("ALL")) {
        items.retain(|item| !item.eq_ignore_ascii_case("ALL"));
        items.extend(["FLAGS".into(), "INTERNALDATE".into(), "RFC822.SIZE".into()]);
    }
    if items.iter().any(|item| item.eq_ignore_ascii_case("FAST")) {
        items.retain(|item| !item.eq_ignore_ascii_case("FAST"));
        items.extend(["FLAGS".into(), "INTERNALDATE".into(), "RFC822.SIZE".into()]);
    }
    if items.iter().any(|item| item.eq_ignore_ascii_case("FULL")) {
        items.retain(|item| !item.eq_ignore_ascii_case("FULL"));
        items.extend([
            "FLAGS".into(),
            "INTERNALDATE".into(),
            "RFC822.SIZE".into(),
            "BODY[HEADER]".into(),
        ]);
    }
    if uid_mode && !items.iter().any(|item| item.eq_ignore_ascii_case("UID")) {
        items.push("UID".into());
    }
    if items.is_empty() || items.iter().any(|item| !supported_fetch_item(item)) {
        return write_tagged(stream, &command.tag, "BAD", "unsupported FETCH data item").await;
    }
    let messages = load_messages(&state.pool, selected.mailbox_id, selected.folder_id)
        .await
        .map_err(io::Error::other)?;
    let wanted = match sequence_members(sequence_set, messages.len(), uid_mode, &messages) {
        Ok(wanted) => wanted,
        Err(_) => {
            return write_tagged(stream, &command.tag, "BAD", "invalid message sequence set").await;
        }
    };
    for (index, message) in messages.iter().enumerate() {
        let sequence = index + 1;
        if !wanted.contains(&sequence) {
            continue;
        }
        let mut flags = message.flags.clone();
        let wants_body = items.iter().any(|item| is_body_item(item));
        let peek = items
            .iter()
            .any(|item| item.to_ascii_uppercase().contains("PEEK"));
        if wants_body
            && !peek
            && !flags.iter().any(|flag| flag == "\\Seen")
            && let Some((identity, selected)) =
                selected_context(stream, connection, state, &command.tag, true).await?
        {
            let mut new_flags = flags
                .iter()
                .filter_map(|flag| parse_flag(flag))
                .collect::<Vec<_>>();
            new_flags.push(MailboxFlag::Seen);
            state
                .mailbox_store
                .set_message_flags(
                    identity.tenant_id,
                    identity.user_id,
                    selected.mailbox_id,
                    message.id,
                    &deduplicate_flags(new_flags),
                )
                .await
                .map_err(io::Error::other)?;
            flags.push("\\Seen".to_owned());
        }
        let mut response = Vec::new();
        response.extend_from_slice(format!("* {sequence} FETCH (").as_bytes());
        let mut first = true;
        for item in &items {
            let upper = item.to_ascii_uppercase();
            let field = match upper.as_str() {
                "UID" => format!("UID {}", message.uid),
                "FLAGS" => format!("FLAGS ({})", flags.join(" ")),
                "RFC822.SIZE" => format!("RFC822.SIZE {}", message.raw_message.len()),
                "INTERNALDATE" => format!(
                    "INTERNALDATE \"{}\"",
                    message.internal_date.format("%d-%b-%Y %H:%M:%S %z")
                ),
                "RFC822" | "BODY[]" | "BODY.PEEK[]" => {
                    let body_field = if upper == "RFC822" {
                        format!("RFC822 {{{}}}\r\n", message.raw_message.len())
                    } else {
                        format!("BODY[] {{{}}}\r\n", message.raw_message.len())
                    };
                    if !first {
                        response.extend_from_slice(b" ");
                    }
                    response.extend_from_slice(body_field.as_bytes());
                    response.extend_from_slice(&message.raw_message);
                    response.extend_from_slice(b"\r\n");
                    first = false;
                    continue;
                }
                "BODY[HEADER]" | "BODY.PEEK[HEADER]" | "RFC822.HEADER" => {
                    let header = message_header(&message.raw_message);
                    let body_field = format!(
                        "{} {{{}}}\r\n",
                        if upper.starts_with("BODY") {
                            "BODY[HEADER]"
                        } else {
                            "RFC822.HEADER"
                        },
                        header.len()
                    );
                    if !first {
                        response.extend_from_slice(b" ");
                    }
                    response.extend_from_slice(body_field.as_bytes());
                    response.extend_from_slice(&header);
                    response.extend_from_slice(b"\r\n");
                    first = false;
                    continue;
                }
                "BODY[TEXT]" | "BODY.PEEK[TEXT]" | "RFC822.TEXT" => {
                    let body = message_body(&message.raw_message);
                    let body_field = format!(
                        "{} {{{}}}\r\n",
                        if upper.starts_with("BODY") {
                            "BODY[TEXT]"
                        } else {
                            "RFC822.TEXT"
                        },
                        body.len()
                    );
                    if !first {
                        response.extend_from_slice(b" ");
                    }
                    response.extend_from_slice(body_field.as_bytes());
                    response.extend_from_slice(&body);
                    response.extend_from_slice(b"\r\n");
                    first = false;
                    continue;
                }
                _ if upper.starts_with("BODY[]<") || upper.starts_with("BODY.PEEK[]<") => {
                    let body = partial_body(&message.raw_message, &upper);
                    let body_field = format!("BODY[] {{{}}}\r\n", body.len());
                    if !first {
                        response.extend_from_slice(b" ");
                    }
                    response.extend_from_slice(body_field.as_bytes());
                    response.extend_from_slice(&body);
                    response.extend_from_slice(b"\r\n");
                    first = false;
                    continue;
                }
                _ => continue,
            };
            if !first {
                response.extend_from_slice(b" ");
            }
            response.extend_from_slice(field.as_bytes());
            first = false;
        }
        response.extend_from_slice(b")\r\n");
        stream.write_all(&response).await?;
    }
    write_tagged(stream, &command.tag, "OK", "FETCH completed").await
}

pub(super) fn parse_fetch_items(tokens: &[Token]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(Token::value)
        .map(str::to_owned)
        .collect()
}

pub(super) fn supported_fetch_item(item: &str) -> bool {
    let item = item.to_ascii_uppercase();
    matches!(
        item.as_str(),
        "ALL"
            | "FAST"
            | "FULL"
            | "UID"
            | "FLAGS"
            | "RFC822.SIZE"
            | "INTERNALDATE"
            | "RFC822"
            | "BODY[]"
            | "BODY.PEEK[]"
            | "BODY[HEADER]"
            | "BODY.PEEK[HEADER]"
            | "RFC822.HEADER"
            | "BODY[TEXT]"
            | "BODY.PEEK[TEXT]"
            | "RFC822.TEXT"
    ) || item.starts_with("BODY[]<")
        || item.starts_with("BODY.PEEK[]<")
}

pub(super) fn is_body_item(item: &str) -> bool {
    let upper = item.to_ascii_uppercase();
    upper == "RFC822"
        || upper.ends_with("[]")
        || upper.ends_with("[HEADER]")
        || upper.ends_with("[TEXT]")
        || upper.starts_with("BODY[]<")
        || upper.starts_with("BODY.PEEK[]<")
        || upper == "RFC822.HEADER"
        || upper == "RFC822.TEXT"
}

pub(super) fn message_header(raw: &[u8]) -> Vec<u8> {
    raw.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|end| raw[..end + 2].to_vec())
        .unwrap_or_default()
}

pub(super) fn message_body(raw: &[u8]) -> Vec<u8> {
    raw.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| raw[start + 4..].to_vec())
        .unwrap_or_default()
}

pub(super) fn partial_body(raw: &[u8], item: &str) -> Vec<u8> {
    let Some(partial) = item
        .rsplit_once('<')
        .map(|(_, partial)| partial.trim_end_matches('>'))
    else {
        return raw.to_vec();
    };
    let mut values = partial.split('.');
    let Ok(offset) = values.next().unwrap_or_default().parse::<usize>() else {
        return Vec::new();
    };
    let Some(length) = values.next().and_then(|value| value.parse::<usize>().ok()) else {
        return Vec::new();
    };
    let start = offset.min(raw.len());
    let end = start.saturating_add(length).min(raw.len());
    raw[start..end].to_vec()
}

pub(super) fn sequence_members(
    sequence_set: &str,
    message_count: usize,
    uid_mode: bool,
    messages: &[MessageRow],
) -> Result<Vec<usize>, ()> {
    if sequence_set.is_empty() || sequence_set.len() > crate::MAX_SEQUENCE_SET_BYTES {
        return Err(());
    }
    let mut result = Vec::new();
    for component in sequence_set.split(',') {
        let mut bounds = component.split(':');
        let start = parse_sequence_bound(bounds.next().ok_or(())?, message_count)?;
        let end = parse_sequence_bound(bounds.next().unwrap_or(component), message_count)?;
        if bounds.next().is_some() {
            return Err(());
        }
        let min = start.min(end);
        let max = start.max(end);
        for (index, message) in messages.iter().enumerate() {
            let value = if uid_mode {
                message.uid as usize
            } else {
                index + 1
            };
            if (min..=max).contains(&value) && !result.contains(&(index + 1)) {
                result.push(index + 1);
            }
        }
    }
    Ok(result)
}

pub(super) fn parse_sequence_bound(value: &str, max: usize) -> Result<usize, ()> {
    if value == "*" {
        return Ok(max.max(1));
    }
    let parsed = value.parse::<usize>().map_err(|_| ())?;
    if parsed == 0 {
        return Err(());
    }
    Ok(parsed)
}
