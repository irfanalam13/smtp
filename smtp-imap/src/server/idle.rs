use super::*;

pub(super) async fn idle<S: AsyncRead + AsyncWrite + Unpin + Send>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    command: &CommandLine,
) -> io::Result<()> {
    let Some((_identity, _selected)) =
        selected_context(stream, connection, state, &command.tag, false).await?
    else {
        return Ok(());
    };
    if !command.arguments.is_empty() {
        return write_tagged(stream, &command.tag, "BAD", "IDLE takes no arguments").await;
    }
    stream.write_all(b"+ idling\r\n").await?;
    stream.flush().await?;
    let mut pending_line = Vec::new();
    loop {
        tokio::select! {
            result = timeout(
                Duration::from_secs(30),
                read_bounded_line(stream, &mut pending_line, 16),
            ) => {
                match result {
                    Ok(Ok(None)) => return Ok(()),
                    Ok(Ok(Some(line))) => {
                        let done = line.eq_ignore_ascii_case(b"DONE\r\n");
                        if done { break; }
                        stream.write_all(b"* BAD send DONE to end IDLE\r\n").await?;
                    }
                    Ok(Err(error)) => return Err(error),
                    Err(_) => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                notify_selected(stream, connection, state).await?;
            }
        }
    }
    write_tagged(stream, &command.tag, "OK", "IDLE terminated").await
}
