#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    User(String),
    Pass(String),
    Stat,
    List(Option<u32>),
    Retr(u32),
    Dele(u32),
    Noop,
    Rset,
    Uidl(Option<u32>),
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    Empty,
    MissingArgument,
    InvalidNumber,
    Unsupported,
}

pub fn parse(input: &str) -> Result<Command, ParseError> {
    let mut parts = input.trim().splitn(2, ' ');
    let name = parts.next().ok_or(ParseError::Empty)?;
    let argument = parts.next().unwrap_or("").trim();
    match name.to_ascii_uppercase().as_str() {
        "USER" => Ok(Command::User(required(argument)?)),
        "PASS" => Ok(Command::Pass(required(argument)?)),
        "STAT" if argument.is_empty() => Ok(Command::Stat),
        "LIST" => number(argument).map(Command::List),
        "RETR" => Ok(Command::Retr(number_required(argument)?)),
        "DELE" => Ok(Command::Dele(number_required(argument)?)),
        "NOOP" if argument.is_empty() => Ok(Command::Noop),
        "RSET" if argument.is_empty() => Ok(Command::Rset),
        "UIDL" => number(argument).map(Command::Uidl),
        "QUIT" if argument.is_empty() => Ok(Command::Quit),
        _ => Err(ParseError::Unsupported),
    }
}

fn required(value: &str) -> Result<String, ParseError> {
    (!value.is_empty())
        .then(|| value.to_string())
        .ok_or(ParseError::MissingArgument)
}
fn number(value: &str) -> Result<Option<u32>, ParseError> {
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse()
            .map(Some)
            .map_err(|_| ParseError::InvalidNumber)
    }
}
fn number_required(value: &str) -> Result<u32, ParseError> {
    number(value)?.ok_or(ParseError::MissingArgument)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionState {
    Authorization,
    Transaction,
    Update,
    Closed,
}

pub struct Session {
    state: SessionState,
    username: Option<String>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            state: SessionState::Authorization,
            username: None,
        }
    }
    pub fn state(&self) -> &SessionState {
        &self.state
    }
    pub fn process(&mut self, command: Command) -> Vec<String> {
        match command {
            Command::User(username) if self.state == SessionState::Authorization => {
                self.username = Some(username);
                vec!["+OK user accepted".into()]
            }
            Command::Pass(_)
                if self.state == SessionState::Authorization && self.username.is_some() =>
            {
                self.state = SessionState::Transaction;
                vec!["+OK maildrop ready".into()]
            }
            Command::Stat if self.state == SessionState::Transaction => vec!["+OK 0 0".into()],
            Command::List(number) if self.state == SessionState::Transaction => match number {
                Some(number) => vec![format!("+OK {number} 0")],
                None => vec!["+OK 0 messages".into(), ".".into()],
            },
            Command::Retr(_) | Command::Dele(_) | Command::Uidl(_)
                if self.state == SessionState::Transaction =>
            {
                vec!["+OK command accepted".into()]
            }
            Command::Noop | Command::Rset if self.state == SessionState::Transaction => {
                vec!["+OK".into()]
            }
            Command::Quit => {
                self.state = SessionState::Closed;
                vec!["+OK goodbye".into()]
            }
            _ => vec!["-ERR invalid state or command".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enforces_pop3_authorization_transition() {
        let mut session = Session::new();
        assert!(session.process(Command::Stat)[0].starts_with("-ERR"));
        assert!(session.process(Command::User("user".into()))[0].starts_with("+OK"));
        assert!(session.process(Command::Pass("secret".into()))[0].contains("maildrop"));
        assert_eq!(session.state(), &SessionState::Transaction);
    }
}
