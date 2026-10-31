use super::*;

pub(super) async fn login<S>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    peer: SocketAddr,
    command: &CommandLine,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    if connection.identity.is_some() {
        return write_tagged(stream, &command.tag, "BAD", "already authenticated").await;
    }
    if command.arguments.len() != 2 {
        return write_tagged(
            stream,
            &command.tag,
            "BAD",
            "LOGIN requires user and password",
        )
        .await;
    }
    let username = command.arguments[0].value().unwrap_or_default();
    let password = command.arguments[1].value().unwrap_or_default();
    authenticate(
        stream,
        connection,
        state,
        peer,
        &command.tag,
        username,
        password,
    )
    .await
}

pub(super) async fn authenticate_plain<S>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    peer: SocketAddr,
    command: &CommandLine,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    if connection.identity.is_some() {
        return write_tagged(stream, &command.tag, "BAD", "already authenticated").await;
    }
    if command.arguments.len() != 1 || !command.arguments[0].is_atom("PLAIN") {
        return write_tagged(
            stream,
            &command.tag,
            "NO",
            "only AUTHENTICATE PLAIN is supported",
        )
        .await;
    }
    stream.write_all(b"+ \r\n").await?;
    stream.flush().await?;
    let mut pending_line = Vec::new();
    let response = timeout(
        COMMAND_TIMEOUT,
        read_bounded_line(stream, &mut pending_line, 2048),
    )
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "AUTHENTICATE response timed out"))?;
    let Some(response) = response? else {
        return write_tagged(stream, &command.tag, "NO", "authentication failed").await;
    };
    if !response.ends_with(b"\r\n") || response == b"*\r\n" {
        return write_tagged(stream, &command.tag, "NO", "authentication failed").await;
    }
    let encoded = response.strip_suffix(b"\n").unwrap_or(&response);
    let encoded = encoded.strip_suffix(b"\r").unwrap_or(encoded);
    let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
    else {
        return write_tagged(stream, &command.tag, "NO", "authentication failed").await;
    };
    let mut fields = decoded.splitn(3, |byte| *byte == 0);
    let _authorization_id = fields.next();
    let username = fields
        .next()
        .and_then(|value| std::str::from_utf8(value).ok())
        .unwrap_or_default();
    let password = fields
        .next()
        .and_then(|value| std::str::from_utf8(value).ok())
        .unwrap_or_default();
    authenticate(
        stream,
        connection,
        state,
        peer,
        &command.tag,
        username,
        password,
    )
    .await
}

pub(super) async fn authenticate<S>(
    stream: &mut BufStream<S>,
    connection: &mut Connection,
    state: &AppState,
    peer: SocketAddr,
    tag: &str,
    username: &str,
    password: &str,
) -> io::Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin + Send,
{
    if username.len() > MAX_AUTH_USERNAME_BYTES
        || username.contains('\0')
        || password.contains('\0')
    {
        return write_tagged(stream, tag, "NO", "authentication failed").await;
    }
    match state
        .auth_rate_limiter
        .is_limited(peer.ip(), username)
        .await
    {
        Ok(true) => {
            return write_tagged(
                stream,
                tag,
                "NO",
                "[UNAVAILABLE] authentication temporarily limited",
            )
            .await;
        }
        Ok(false) => {}
        Err(error) => {
            tracing::error!(%error, "IMAP authentication rate-limit lookup failed");
            return write_tagged(stream, tag, "NO", "authentication service unavailable").await;
        }
    }
    match state.authenticator.authenticate(username, password).await {
        Ok(Some(authenticated)) => {
            let row = sqlx::query(
                "SELECT credential.user_id
                 FROM smtp_credentials AS credential
                 JOIN users AS account
                   ON account.id = credential.user_id AND account.status = 'active'
                 JOIN tenant_memberships AS membership
                   ON membership.tenant_id = credential.tenant_id
                  AND membership.user_id = credential.user_id
                 JOIN tenants AS tenant
                   ON tenant.id = credential.tenant_id AND tenant.status = 'active'
                 WHERE credential.tenant_id = $1 AND credential.username = $2
                   AND credential.revoked_at IS NULL
                   AND (credential.expires_at IS NULL OR credential.expires_at > NOW())",
            )
            .bind(authenticated.tenant_id)
            .bind(&authenticated.username)
            .fetch_optional(&state.pool)
            .await;
            match row {
                Ok(Some(row)) => {
                    let user_id: Uuid = row.try_get("user_id").map_err(io::Error::other)?;
                    connection.identity = Some(Identity {
                        tenant_id: authenticated.tenant_id,
                        user_id,
                        credential_username: authenticated.username,
                    });
                    if let Err(error) = state
                        .auth_rate_limiter
                        .record_success(peer.ip(), username)
                        .await
                    {
                        tracing::error!(%error, "IMAP authentication rate-limit reset failed");
                        connection.identity = None;
                        return write_tagged(
                            stream,
                            tag,
                            "NO",
                            "authentication service unavailable",
                        )
                        .await;
                    }
                    write_tagged(stream, tag, "OK", "LOGIN completed").await?;
                    tracing::info!(user_id = %user_id, peer = %peer, "IMAP authentication succeeded");
                }
                Ok(None) => {
                    if let Err(error) = state
                        .auth_rate_limiter
                        .record_failure(peer.ip(), username)
                        .await
                    {
                        tracing::error!(%error, "IMAP authentication failure could not be rate-limited");
                        return write_tagged(
                            stream,
                            tag,
                            "NO",
                            "authentication service unavailable",
                        )
                        .await;
                    }
                    write_tagged(stream, tag, "NO", "authentication failed").await?
                }
                Err(error) => {
                    tracing::error!(%error, "IMAP identity lookup failed");
                    write_tagged(stream, tag, "NO", "authentication service unavailable").await?;
                }
            }
        }
        Ok(None) => {
            if let Err(error) = state
                .auth_rate_limiter
                .record_failure(peer.ip(), username)
                .await
            {
                tracing::error!(%error, "IMAP authentication failure could not be rate-limited");
                return write_tagged(stream, tag, "NO", "authentication service unavailable").await;
            }
            write_tagged(stream, tag, "NO", "authentication failed").await?
        }
        Err(error) => {
            tracing::error!(%error, "IMAP credential verification failed");
            write_tagged(stream, tag, "NO", "authentication service unavailable").await?;
        }
    }
    Ok(())
}
