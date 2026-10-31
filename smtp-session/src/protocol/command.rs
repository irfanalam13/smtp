#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Ehlo(String),
    Helo(String),
    MailFrom {
        sender: String,
        size: Option<usize>,
        body_8bit: bool,
        has_body_parameter: bool,
        has_esmtp_parameters: bool,
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
    DuplicateParameter,
    InvalidCommand,
}

pub fn parse(input: &[u8]) -> Result<Command, ParseError> {
    if input.is_empty() {
        return Err(ParseError::Empty);
    }
    if input
        .iter()
        .any(|byte| !byte.is_ascii() || byte.is_ascii_control())
    {
        return Err(ParseError::InvalidCommand);
    }
    let text = std::str::from_utf8(input).map_err(|_| ParseError::InvalidCommand)?;
    let (name, argument) = text.split_once(' ').map_or((text, ""), |(name, arg)| {
        (name, arg.trim_start_matches(' '))
    });
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
        "NOOP" => Ok(Command::Noop),
        "QUIT" if argument.is_empty() => Ok(Command::Quit),
        "STARTTLS" if argument.is_empty() => Ok(Command::StartTls),
        "AUTH" if !argument.is_empty() => parse_auth(argument),
        "AUTH" => Err(ParseError::MissingArgument),
        "DATA" | "RSET" | "QUIT" | "STARTTLS" => Err(ParseError::MalformedParameter),
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
    (valid_ehlo_argument(argument))
        .then(|| argument.to_string())
        .ok_or(ParseError::MissingArgument)
}

fn valid_ehlo_argument(argument: &str) -> bool {
    !argument.is_empty()
        && argument.len() <= 255
        && argument
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
}

fn parse_mail_from(argument: &str) -> Result<Command, ParseError> {
    let rest = argument
        .get(..5)
        .filter(|prefix| prefix.eq_ignore_ascii_case("FROM:"))
        .map(|_| &argument[5..])
        .ok_or(ParseError::MissingArgument)?;
    if !rest.starts_with('<') {
        return Err(ParseError::MalformedPath);
    }
    let path_end = rest.find('>').ok_or(ParseError::MalformedPath)?;
    let sender = &rest[1..path_end];
    if !sender.is_empty() {
        validate_path(sender)?;
    }
    let parameter_suffix = &rest[path_end + 1..];
    if !parameter_suffix.is_empty() && !parameter_suffix.starts_with(' ') {
        return Err(ParseError::MalformedParameter);
    }
    let parameters = parameter_suffix.trim_start_matches(' ');
    let mut size = None;
    let mut body_8bit = false;
    let mut has_esmtp_parameters = false;
    let mut has_body_parameter = false;
    let mut saw_size = false;
    let mut saw_body = false;
    for parameter in parameters.split_ascii_whitespace() {
        has_esmtp_parameters = true;
        if parameter.contains('>') || parameter.contains('<') {
            return Err(ParseError::UnsupportedParameter);
        }
        let Some((name, value)) = parameter.split_once('=') else {
            return Err(ParseError::UnsupportedParameter);
        };
        if value.is_empty() || value.contains('=') {
            return Err(ParseError::MalformedParameter);
        }
        match name.to_ascii_uppercase().as_str() {
            "SIZE" => {
                if saw_size {
                    return Err(ParseError::DuplicateParameter);
                }
                saw_size = true;
                if !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(ParseError::MalformedParameter);
                }
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| ParseError::MalformedParameter)?;
                size = Some(parsed);
            }
            "BODY" if value.eq_ignore_ascii_case("8BITMIME") => {
                if saw_body {
                    return Err(ParseError::DuplicateParameter);
                }
                saw_body = true;
                has_body_parameter = true;
                body_8bit = true;
            }
            "BODY" if value.eq_ignore_ascii_case("7BIT") => {
                if saw_body {
                    return Err(ParseError::DuplicateParameter);
                }
                saw_body = true;
                has_body_parameter = true;
            }
            _ => return Err(ParseError::UnsupportedParameter),
        }
    }
    Ok(Command::MailFrom {
        sender: sender.to_string(),
        size,
        body_8bit,
        has_body_parameter,
        has_esmtp_parameters,
    })
}

fn parse_recipient(argument: &str) -> Result<String, ParseError> {
    let rest = argument
        .get(..3)
        .filter(|prefix| prefix.eq_ignore_ascii_case("TO:"))
        .map(|_| &argument[3..])
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
    if local.is_empty() || local.len() > 64 || domain.is_empty() || domain.len() > 253 {
        return Err(ParseError::MalformedPath);
    }
    let atom = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-/=?^_`{|}~.".contains(&byte))
            && !part.starts_with('.')
            && !part.ends_with('.')
            && !part.contains("..")
    };
    if !atom(local)
        || domain.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                || label.starts_with('-')
                || label.ends_with('-')
        })
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
                has_body_parameter: false,
                has_esmtp_parameters: false,
            })
        );
        assert_eq!(
            parse(b"mail from:<a@example.test>"),
            Ok(Command::MailFrom {
                sender: "a@example.test".into(),
                size: None,
                body_8bit: false,
                has_body_parameter: false,
                has_esmtp_parameters: false,
            })
        );
        assert_eq!(
            parse(b"rcpt to:<b@example.test>"),
            Ok(Command::RcptTo("b@example.test".into()))
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
                has_body_parameter: false,
                has_esmtp_parameters: false,
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
                has_body_parameter: true,
                has_esmtp_parameters: true,
            })
        );
    }

    #[test]
    fn accepts_esmtp_parameters_with_null_reverse_path() {
        assert_eq!(
            parse(b"MAIL FROM:<> SIZE=0 BODY=7BIT"),
            Ok(Command::MailFrom {
                sender: String::new(),
                size: Some(0),
                body_8bit: false,
                has_body_parameter: true,
                has_esmtp_parameters: true,
            })
        );
    }

    #[test]
    fn rejects_duplicate_and_malformed_esmtp_parameters() {
        for line in [
            &b"MAIL FROM:<a@example.test> SIZE=1 SIZE=2"[..],
            b"MAIL FROM:<a@example.test> BODY=7BIT BODY=8BITMIME",
            b"MAIL FROM:<a@example.test> SIZE=",
            b"MAIL FROM:<a@example.test> SIZE=1=2",
            b"MAIL FROM:<a@example.test> FUTURE=value",
        ] {
            assert!(
                parse(line).is_err(),
                "accepted {}",
                String::from_utf8_lossy(line)
            );
        }
    }

    #[test]
    fn rejects_non_crlf_control_and_invalid_address_syntax() {
        for line in [
            &b"MAIL FROM:<.a@example.test>"[..],
            b"MAIL FROM:<a..b@example.test>",
            b"MAIL FROM:<a@example-.test>",
            b"EHLO client\tpipelining",
            b"NOOP\r",
        ] {
            assert!(
                parse(line).is_err(),
                "accepted {}",
                String::from_utf8_lossy(line)
            );
        }
    }

    #[test]
    fn parser_never_panics_on_bounded_adversarial_input() {
        for first in 0..=u8::MAX {
            for second in 0..=u8::MAX {
                let _ = parse(&[first, second]);
            }
        }
        for prefix in [b"MAIL FROM:<".as_slice(), b"RCPT TO:<", b"EHLO "] {
            for byte in 0..=u8::MAX {
                let mut input = prefix.to_vec();
                input.push(byte);
                let _ = parse(&input);
            }
        }
        let mut state = 0x9e37_79b9_u32;
        let mut input = Vec::new();
        for case in 0..4_096 {
            input.clear();
            for _ in 0..case % 1_025 {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                input.push(state as u8);
            }
            let _ = parse(&input);
        }
    }
}
