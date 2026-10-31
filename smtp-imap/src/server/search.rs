use super::*;

pub(super) async fn search<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    search_args(
        stream,
        connection,
        state,
        command,
        &command.arguments,
        false,
    )
    .await
}

pub(super) async fn search_args<S: AsyncRead + AsyncWrite + Unpin + Send>(
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
    let messages = load_messages(&state.pool, selected.mailbox_id, selected.folder_id)
        .await
        .map_err(io::Error::other)?;
    let mut matched: Option<Vec<usize>> = None;
    let mut index = 0;
    while index < arguments.len() {
        let Some(key) = arguments[index].value() else {
            return write_tagged(stream, &command.tag, "BAD", "invalid SEARCH key").await;
        };
        let key = key.to_ascii_uppercase();
        index += 1;
        let mut candidates = Vec::new();
        match key.as_str() {
            "ALL" | "RECENT" => candidates.extend(0..messages.len()),
            "SEEN" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| message.flags.iter().any(|flag| flag == "\\Seen"))
                    .map(|(i, _)| i),
            ),
            "UNSEEN" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| !message.flags.iter().any(|flag| flag == "\\Seen"))
                    .map(|(i, _)| i),
            ),
            "ANSWERED" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| message.flags.iter().any(|flag| flag == "\\Answered"))
                    .map(|(i, _)| i),
            ),
            "UNANSWERED" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| !message.flags.iter().any(|flag| flag == "\\Answered"))
                    .map(|(i, _)| i),
            ),
            "FLAGGED" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| message.flags.iter().any(|flag| flag == "\\Flagged"))
                    .map(|(i, _)| i),
            ),
            "UNFLAGGED" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| !message.flags.iter().any(|flag| flag == "\\Flagged"))
                    .map(|(i, _)| i),
            ),
            "DELETED" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| message.flags.iter().any(|flag| flag == "\\Deleted"))
                    .map(|(i, _)| i),
            ),
            "UNDELETED" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| !message.flags.iter().any(|flag| flag == "\\Deleted"))
                    .map(|(i, _)| i),
            ),
            "DRAFT" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| message.flags.iter().any(|flag| flag == "\\Draft"))
                    .map(|(i, _)| i),
            ),
            "UNDRAFT" => candidates.extend(
                messages
                    .iter()
                    .enumerate()
                    .filter(|(_, message)| !message.flags.iter().any(|flag| flag == "\\Draft"))
                    .map(|(i, _)| i),
            ),
            "UID" => {
                let Some(set) = arguments.get(index).and_then(Token::value) else {
                    return write_tagged(
                        stream,
                        &command.tag,
                        "BAD",
                        "UID SEARCH requires a sequence set",
                    )
                    .await;
                };
                index += 1;
                let uids = match sequence_members(set, messages.len(), true, &messages) {
                    Ok(uids) => uids
                        .into_iter()
                        .map(|one_based| one_based - 1)
                        .collect::<Vec<_>>(),
                    Err(_) => {
                        return write_tagged(
                            stream,
                            &command.tag,
                            "BAD",
                            "invalid UID sequence set",
                        )
                        .await;
                    }
                };
                candidates.extend(uids);
            }
            "LARGER" | "SMALLER" => {
                let Some(size) = arguments
                    .get(index)
                    .and_then(Token::value)
                    .and_then(|value| value.parse::<usize>().ok())
                else {
                    return write_tagged(stream, &command.tag, "BAD", "invalid SEARCH size").await;
                };
                index += 1;
                candidates.extend(
                    messages
                        .iter()
                        .enumerate()
                        .filter(|(_, message)| {
                            if key == "LARGER" {
                                message.raw_message.len() > size
                            } else {
                                message.raw_message.len() < size
                            }
                        })
                        .map(|(i, _)| i),
                );
            }
            "FROM" | "TO" | "CC" | "BCC" | "SUBJECT" | "BODY" | "TEXT" => {
                let Some(value) = arguments.get(index).and_then(Token::value) else {
                    return write_tagged(
                        stream,
                        &command.tag,
                        "BAD",
                        "SEARCH text key requires a value",
                    )
                    .await;
                };
                let needle = value.to_ascii_lowercase();
                index += 1;
                candidates.extend(
                    messages
                        .iter()
                        .enumerate()
                        .filter(|(_, message)| {
                            let haystack = match key.as_str() {
                                "BODY" => {
                                    String::from_utf8_lossy(&message_body(&message.raw_message))
                                        .to_ascii_lowercase()
                                }
                                "TEXT" => String::from_utf8_lossy(&message.raw_message)
                                    .to_ascii_lowercase(),
                                _ => String::from_utf8_lossy(&message_header(&message.raw_message))
                                    .to_ascii_lowercase(),
                            };
                            haystack.contains(&needle)
                        })
                        .map(|(i, _)| i),
                );
            }
            _ => return write_tagged(stream, &command.tag, "BAD", "unsupported SEARCH key").await,
        }
        candidates.sort_unstable();
        candidates.dedup();
        if let Some(matched) = &mut matched {
            matched.retain(|candidate| candidates.binary_search(candidate).is_ok());
        } else {
            matched = Some(candidates);
        }
    }
    let matched = matched.unwrap_or_else(|| (0..messages.len()).collect());
    let values = matched
        .iter()
        .map(|index| {
            if uid_mode {
                messages[*index].uid.to_string()
            } else {
                (index + 1).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    stream
        .write_all(
            format!(
                "* SEARCH{space}{values}\r\n",
                space = if values.is_empty() { "" } else { " " }
            )
            .as_bytes(),
        )
        .await?;
    write_tagged(stream, &command.tag, "OK", "SEARCH completed").await
}
