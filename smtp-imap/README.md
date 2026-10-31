# smtp-imap

This crate provides an implicit-TLS IMAP4rev1 listener backed by the shared
PostgreSQL mailbox store. It deliberately advertises IMAP4rev1 rather than
IMAP4rev2: only implemented commands and extensions are listed in CAPABILITY.

## Configuration

The service requires:

- `DATABASE_URL`: PostgreSQL connection string.
- `IMAP_TLS_CERT` and `IMAP_TLS_KEY`: PEM certificate chain and private key.

Optional settings:

- `IMAP_ADDR`: listen address; defaults to `127.0.0.1:993`.
- `IMAP_MAX_CONNECTIONS`: concurrent connection limit, 1–1024; defaults to 256.
- `IMAP_SERVER_NAME`: greeting label; defaults to `smtp-platform`.

The listener is implicit TLS only. It never accepts a plaintext IMAP session;
LOGIN and AUTHENTICATE PLAIN are only reachable after TLS negotiation.

## Authentication and mailbox names

LOGIN username is `<tenant-uuid>:<smtp-credential-username>`. Use the same
tenant-scoped credentials managed by the SMTP credential API. The credential
must be bound to an active tenant member; unbound legacy credentials do not
authenticate to IMAP. Password verification uses the shared Argon2id credential
verifier. Passwords are never logged.

`INBOX` selects the first mailbox owned by the authenticated account, preferring
one whose address matches the credential username. `LIST` also returns
mailbox addresses and their folders; those can be used as qualified mailbox
names (`address@example.test/Folder`). Mailbox grants and membership are
rechecked for selected mailbox operations.

## Supported profile

The service supports CAPABILITY, NOOP, LOGOUT, LOGIN, AUTHENTICATE PLAIN,
NAMESPACE, LIST, LSUB, STATUS, SELECT, EXAMINE, CLOSE, UNSELECT, FETCH, STORE,
SEARCH, COPY, MOVE, APPEND, CREATE, DELETE, RENAME, EXPUNGE, UID FETCH/STORE/
SEARCH/COPY/MOVE/EXPUNGE, and IDLE. Message literals are bounded to 10 MiB.
APPEND uses synchronizing literals; LITERAL+ is not advertised. Unsupported
commands, FETCH items, and SEARCH keys receive explicit BAD responses.

This is an initial IMAP4rev1 profile, not a claim of full RFC 3501 conformance.
CONDSTORE/QRESYNC, ESEARCH, SORT, THREAD, ID, COMPRESS, STARTTLS and IMAP4rev2
are not advertised. The durable mailbox change journal is polled during IDLE
and NOOP to report message arrivals/removals to concurrent sessions.

## Source layout

- `src/lib.rs` contains bounded command and literal parsing.
- `src/server/mod.rs` defines shared connection state and composes the server
  modules; `runtime.rs` owns listener setup, TLS, connection I/O, and command
  dispatch.
- `authentication.rs` handles LOGIN and AUTHENTICATE PLAIN.
- `mailbox.rs` and `mailbox_selection.rs` handle mailbox discovery, folder
  resolution, selection state, status, and change notifications.
- `fetch.rs`, `flags.rs`, and `search.rs` implement message reads and queries.
- `append.rs`, `copy_move.rs`, and `folders.rs` implement mailbox mutations.
- `idle.rs` handles the IDLE command; `tests.rs` holds server unit tests.

Run with `cargo run -p smtp-imap`. PostgreSQL migrations must be applied first,
including `smtp_auth/000027_bind_smtp_credentials_to_users`.
