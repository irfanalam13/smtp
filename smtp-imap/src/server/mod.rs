use std::{
    env,
    io::{self, Cursor},
    net::SocketAddr,
    path::Path,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use rustls::{ServerConfig, pki_types::CertificateDer};
use smtp_core::{MailboxFlag, MailboxStore};
use smtp_ingress::{Authenticator, PostgresAuthRateLimiter, PostgresAuthenticator};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use tokio::{
    io::{
        AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
        BufStream,
    },
    net::{TcpListener, TcpStream},
    sync::{OwnedSemaphorePermit, Semaphore},
    task::JoinSet,
    time::timeout,
};
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;

use crate::{
    CommandLine, MAX_COMMAND_BYTES, MAX_LITERAL_BYTES, ParseError, Token, parse_command_line,
    parse_literal_marker,
};

const CAPABILITY: &str = "IMAP4rev1 AUTH=PLAIN NAMESPACE UIDPLUS MOVE IDLE";
const MAX_CONNECTIONS: usize = 1024;
const MAX_FLAGS: usize = 32;
const MAX_AUTH_USERNAME_BYTES: usize = 256;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    mailbox_store: MailboxStore,
    authenticator: PostgresAuthenticator,
    auth_rate_limiter: PostgresAuthRateLimiter,
    tls_acceptor: TlsAcceptor,
    max_connections: usize,
}

#[derive(Clone, Debug)]
struct Identity {
    tenant_id: Uuid,
    user_id: Uuid,
    credential_username: String,
}

#[derive(Clone, Debug)]
struct Selected {
    mailbox_id: Uuid,
    folder_id: Uuid,
    read_only: bool,
    snapshot: Vec<Uuid>,
    change_cursor: i64,
}

#[derive(Default)]
struct Connection {
    identity: Option<Identity>,
    selected: Option<Selected>,
}

#[derive(Clone, Debug)]
struct MessageRow {
    id: Uuid,
    uid: i64,
    flags: Vec<String>,
    raw_message: Vec<u8>,
    internal_date: DateTime<Utc>,
}

mod append;
mod authentication;
mod copy_move;
mod fetch;
mod flags;
mod folders;
mod idle;
mod mailbox;
mod mailbox_selection;
mod runtime;
mod search;
#[cfg(test)]
mod tests;

use append::*;
use authentication::*;
use copy_move::*;
use fetch::*;
use flags::*;
use folders::*;
use idle::*;
use mailbox::*;
use mailbox_selection::*;
pub use runtime::run_from_env;
use runtime::{read_bounded_line, write_tagged};
use search::*;
