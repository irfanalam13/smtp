#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Ehlo(String),
    Helo(String),
    MailFrom {
        sender: String,
        size: Option<usize>,
        body_8bit: bool,
    },
    RcptTo(String),
    Data,
    Rset,
    Noop,
    Quit,
    StartTls,
    Auth {
        mechanism: String,
        initial_response: Option<String>,
    },
    Unsupported(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    Empty,
    MissingArgument,
    MalformedPath,
    MalformedParameter,
    UnsupportedParameter,
}

pub fn parse(input: &[u8]) -> Result<Command, ParseError> {
    let text = std::str::from_utf8(input).map_err(|_| ParseError::Empty)?;
    let text = text.trim_end_matches('\r');
    let (name, argument) = text
        .split_once(' ')
        .map_or((text, ""), |(name, arg)| (name, arg.trim()));
    if name.is_empty() {
        return Err(ParseError::Empty);
    }
    match name.to_ascii_uppercase().as_str() {
        "EHLO" => nonempty(argument).map(Command::Ehlo),
        "HELO" => nonempty(argument).map(Command::Helo),
        "MAIL" => parse_mail_from(argument),
        "RCPT" => parse_recipient(argument).map(Command::RcptTo),
        "DATA" if argument.is_empty() => Ok(Command::Data),
        "RSET" if argument.is_empty() => Ok(Command::Rset),
        "NOOP" if argument.is_empty() => Ok(Command::Noop),
        "QUIT" if argument.is_empty() => Ok(Command::Quit),
        "STARTTLS" if argument.is_empty() => Ok(Command::StartTls),
        "AUTH" if !argument.is_empty() => parse_auth(argument),
        "AUTH" => Err(ParseError::MissingArgument),
        other => Ok(Command::Unsupported(other.to_string())),
    }
}

fn parse_auth(argument: &str) -> Result<Command, ParseError> {
    let mut parts = argument.split_ascii_whitespace();
    let mechanism = parts.next().ok_or(ParseError::MissingArgument)?;
    let initial_response = parts.next().map(str::to_string);
    if parts.next().is_some() {
        return Err(ParseError::UnsupportedParameter);
    }
    Ok(Command::Auth {
        mechanism: mechanism.to_ascii_uppercase(),
        initial_response,
    })
}

fn nonempty(argument: &str) -> Result<String, ParseError> {
    (!argument.is_empty())
        .then(|| argument.to_string())
        .ok_or(ParseError::MissingArgument)
}

fn parse_mail_from(argument: &str) -> Result<Command, ParseError> {
    let rest = argument
        .strip_prefix("FROM:")
        .ok_or(ParseError::MissingArgument)?;
    let (path, parameters) = rest.split_once('>').ok_or(ParseError::MalformedPath)?;
    if !path.starts_with('<') {
        return Err(ParseError::MalformedPath);
    }
    let sender = &path[1..];
    if sender.is_empty() {
        if !parameters.trim().is_empty() {
            return Err(ParseError::UnsupportedParameter);
        }
    } else {
        validate_path(sender)?;
    }
    let mut size = None;
    let mut body_8bit = false;
    for parameter in parameters.split_ascii_whitespace() {
        let Some((name, value)) = parameter.split_once('=') else {
            return Err(ParseError::UnsupportedParameter);
        };
        match name.to_ascii_uppercase().as_str() {
            "SIZE" => {
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| ParseError::MalformedParameter)?;
                size = Some(parsed);
            }
            "BODY" if value.eq_ignore_ascii_case("8BITMIME") => body_8bit = true,
            "BODY" if value.eq_ignore_ascii_case("7BIT") => {}
            _ => return Err(ParseError::UnsupportedParameter),
        }
    }
    Ok(Command::MailFrom {
        sender: sender.to_string(),
        size,
        body_8bit,
    })
}

fn parse_recipient(argument: &str) -> Result<String, ParseError> {
    let rest = argument
        .strip_prefix("TO:")
        .ok_or(ParseError::MissingArgument)?;
    if rest.contains(' ') || !rest.starts_with('<') || !rest.ends_with('>') {
        return Err(ParseError::MalformedPath);
    }
    let path = &rest[1..rest.len() - 1];
    validate_path(path)?;
    Ok(path.to_string())
}

fn validate_path(path: &str) -> Result<(), ParseError> {
    if path.matches('@').count() != 1 {
        return Err(ParseError::MalformedPath);
    }
    let (local, domain) = path.split_once('@').expect("one @ was checked");
    if local.is_empty()
        || domain.is_empty()
        || !path.is_ascii()
        || local
            .chars()
            .any(|c| c.is_ascii_whitespace() || c.is_ascii_control())
        || domain
            .chars()
            .any(|c| c.is_ascii_whitespace() || c.is_ascii_control())
    {
        return Err(ParseError::MalformedPath);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Command, ParseError, parse};

    #[test]
    fn parses_case_insensitively_and_preserves_path() {
        assert_eq!(
            parse(b"mAiL FROM:<a@example.test>"),
            Ok(Command::MailFrom {
                sender: "a@example.test".into(),
                size: None,
                body_8bit: false,
            })
        );
    }

    #[test]
    fn rejects_bad_paths_and_arguments() {
        assert_eq!(parse(b"MAIL FROM:bad"), Err(ParseError::MalformedPath));
        assert_eq!(parse(b"EHLO"), Err(ParseError::MissingArgument));
    }

    #[test]
    fn accepts_null_reverse_path_for_generated_mail() {
        assert_eq!(
            parse(b"MAIL FROM:<>"),
            Ok(Command::MailFrom {
                sender: String::new(),
                size: None,
                body_8bit: false,
            })
        );
    }

    #[test]
    fn parses_supported_mail_parameters() {
        assert_eq!(
            parse(b"MAIL FROM:<a@example.test> SIZE=42 BODY=8BITMIME"),
            Ok(Command::MailFrom {
                sender: "a@example.test".into(),
                size: Some(42),
                body_8bit: true,
            })
        );
    }
}
