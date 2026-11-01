pub mod auth;
pub mod inbound;
pub mod protocol;
pub mod runtime;
pub mod transport;

// Preserve established module paths while implementations live under
// feature-focused folders.
pub use auth::email_auth;
pub use inbound::scanner;
pub use protocol::{command, message, reply, session};
pub use runtime::{config, error, metrics};
pub use transport::server;

pub use auth::{
    AuthRateLimiter, AuthenticatedIdentity, AuthenticationError, Authenticator,
    PostgresAuthRateLimiter, PostgresAuthenticator, SenderAuthorizer,
};
pub use config::{Config, ListenerProfile};
pub use message::Message;
pub use server::Server;
pub use session::{Session, SessionDisposition, SessionRunResult};
