use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Login {
        username: String,
        password: String,
    },
    Select(String),
    Fetch {
        sequence: String,
        items: String,
    },
    Store {
        sequence: String,
        operation: String,
        flags: String,
    },
    Search(String),
    Copy {
        sequence: String,
        mailbox: String,
    },
    Move {
        sequence: String,
        mailbox: String,
    },
    Append {
        mailbox: String,
        message: Vec<u8>,
    },
    Delete(String),
    Expunge,
    Logout,
    Noop,
    Capability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    Empty,
    MissingTag,
    MissingArgument,
    InvalidLiteral,
    Unsupported,
}

pub fn parse(input: &str) -> Result<(String, Command), ParseError> {
    let mut parts = input
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .splitn(3, ' ');
    let tag = parts
        .next()
        .filter(|value| !value.is_empty())
        .ok_or(ParseError::MissingTag)?;
    let name = parts.next().ok_or(ParseError::MissingArgument)?;
    let argument = parts.next().unwrap_or("").trim();
    let command = match name.to_ascii_uppercase().as_str() {
        "LOGIN" => {
            let mut values = argument.splitn(2, ' ');
            Command::Login {
                username: values
                    .next()
                    .filter(|v| !v.is_empty())
                    .ok_or(ParseError::MissingArgument)?
                    .to_string(),
                password: values
                    .next()
                    .filter(|v| !v.is_empty())
                    .ok_or(ParseError::MissingArgument)?
                    .to_string(),
            }
        }
        "SELECT" => Command::Select(required(argument)?),
        "FETCH" => {
            let (sequence, items) = argument
                .split_once(' ')
                .ok_or(ParseError::MissingArgument)?;
            Command::Fetch {
                sequence: sequence.into(),
                items: items.into(),
            }
        }
        "STORE" => {
            let mut values = argument.splitn(3, ' ');
            Command::Store {
                sequence: values.next().ok_or(ParseError::MissingArgument)?.into(),
                operation: values.next().ok_or(ParseError::MissingArgument)?.into(),
                flags: values.next().ok_or(ParseError::MissingArgument)?.into(),
            }
        }
        "SEARCH" => Command::Search(argument.to_string()),
        "COPY" | "MOVE" => {
            let (sequence, mailbox) = argument
                .split_once(' ')
                .ok_or(ParseError::MissingArgument)?;
            if name.eq_ignore_ascii_case("COPY") {
                Command::Copy {
                    sequence: sequence.into(),
                    mailbox: mailbox.into(),
                }
            } else {
                Command::Move {
                    sequence: sequence.into(),
                    mailbox: mailbox.into(),
                }
            }
        }
        "APPEND" => {
            let mailbox = argument
                .strip_suffix('}')
                .and_then(|value| value.rsplit_once('{'))
                .map(|(name, _)| name.trim())
                .ok_or(ParseError::InvalidLiteral)?;
            Command::Append {
                mailbox: mailbox.to_string(),
                message: Vec::new(),
            }
        }
        "DELETE" => Command::Delete(required(argument)?),
        "EXPUNGE" => Command::Expunge,
        "LOGOUT" => Command::Logout,
        "NOOP" => Command::Noop,
        "CAPABILITY" => Command::Capability,
        _ => return Err(ParseError::Unsupported),
    };
    Ok((tag.to_string(), command))
}

fn required(value: &str) -> Result<String, ParseError> {
    (!value.is_empty())
        .then(|| value.to_string())
        .ok_or(ParseError::MissingArgument)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionState {
    NotAuthenticated,
    Authenticated,
    Selected { mailbox: String },
    Closed,
}

pub struct Session {
    state: SessionState,
    max_literal_bytes: usize,
}

impl Session {
    pub fn new(max_literal_bytes: usize) -> Self {
        Self {
            state: SessionState::NotAuthenticated,
            max_literal_bytes,
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    pub fn process(&mut self, tag: &str, command: Command) -> Vec<String> {
        let result = match command {
            Command::Capability => vec![
                "* CAPABILITY IMAP4rev2".into(),
                tagged(tag, "OK capability list"),
            ],
            Command::Login { .. } if self.state == SessionState::NotAuthenticated => {
                self.state = SessionState::Authenticated;
                vec![tagged(tag, "OK LOGIN completed")]
            }
            Command::Login { .. } => vec![tagged(tag, "BAD LOGIN not permitted")],
            Command::Select(mailbox)
                if self.state != SessionState::NotAuthenticated
                    && self.state != SessionState::Closed =>
            {
                self.state = SessionState::Selected { mailbox };
                vec![
                    "* 0 EXISTS".into(),
                    "* 0 RECENT".into(),
                    tagged(tag, "OK [READ-WRITE] SELECT completed"),
                ]
            }
            Command::Select(_) => vec![tagged(tag, "NO authentication required")],
            Command::Logout => {
                self.state = SessionState::Closed;
                vec![
                    "* BYE logging out".into(),
                    tagged(tag, "OK LOGOUT completed"),
                ]
            }
            Command::Noop => vec![tagged(tag, "OK NOOP completed")],
            command
                if matches!(self.state, SessionState::Selected { .. })
                    && command_requires_selected(&command) =>
            {
                vec![tagged(tag, "OK command accepted by mailbox boundary")]
            }
            _ => vec![tagged(tag, "BAD command invalid in current state")],
        };
        result
    }
}

fn command_requires_selected(command: &Command) -> bool {
    matches!(
        command,
        Command::Fetch { .. }
            | Command::Store { .. }
            | Command::Search(_)
            | Command::Copy { .. }
            | Command::Move { .. }
            | Command::Append { .. }
            | Command::Delete(_)
            | Command::Expunge
    )
}

fn tagged(tag: &str, message: &str) -> String {
    format!("{tag} {message}")
}

impl fmt::Debug for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Session")
            .field("state", &self.state)
            .field("max_literal_bytes", &self.max_literal_bytes)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_imap_auth_and_selection_states() {
        let (tag, command) = parse("a1 LOGIN user pass\r\n").unwrap();
        let mut session = Session::new(1024);
        assert_eq!(
            session.process(&tag, command),
            vec!["a1 OK LOGIN completed"]
        );
        let (tag, command) = parse("a2 SELECT INBOX\r\n").unwrap();
        assert!(
            session
                .process(&tag, command)
                .iter()
                .any(|line| line.contains("SELECT completed"))
        );
    }

    #[test]
    fn parses_core_commands() {
        assert!(matches!(
            parse("a1 FETCH 1 BODY[]").unwrap().1,
            Command::Fetch { .. }
        ));
        assert!(matches!(parse("a2 EXPUNGE").unwrap().1, Command::Expunge));
    }
}
