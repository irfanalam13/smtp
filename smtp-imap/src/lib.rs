use thiserror::Error;

pub mod server;

pub const MAX_COMMAND_BYTES: usize = 8192;
pub const MAX_LITERAL_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_SEQUENCE_SET_BYTES: usize = 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandLine {
    pub tag: String,
    pub name: String,
    pub arguments: Vec<Token>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    Atom(String),
    Quoted(String),
    OpenParen,
    CloseParen,
}

impl Token {
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Atom(value) | Self::Quoted(value) => Some(value),
            Self::OpenParen | Self::CloseParen => None,
        }
    }

    pub fn is_atom(&self, value: &str) -> bool {
        matches!(self, Self::Atom(atom) if atom.eq_ignore_ascii_case(value))
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ParseError {
    #[error("command is empty")]
    Empty,
    #[error("command contains invalid UTF-8 or control bytes")]
    InvalidEncoding,
    #[error("command exceeds its line limit")]
    LineTooLong,
    #[error("command tag is invalid")]
    InvalidTag,
    #[error("command is missing an argument")]
    MissingArgument,
    #[error("quoted string is malformed")]
    InvalidQuotedString,
    #[error("command syntax is invalid")]
    InvalidSyntax,
    #[error("literal exceeds its configured limit")]
    LiteralTooLarge,
}

pub fn parse_command_line(input: &[u8]) -> Result<CommandLine, ParseError> {
    if input.len() > MAX_COMMAND_BYTES {
        return Err(ParseError::LineTooLong);
    }
    let input = input.strip_suffix(b"\r\n").unwrap_or(input);
    let input = std::str::from_utf8(input).map_err(|_| ParseError::InvalidEncoding)?;
    if input.is_empty() {
        return Err(ParseError::Empty);
    }
    if input
        .bytes()
        .any(|byte| byte == 0 || (byte < 0x20 && byte != b' ') || byte == 0x7f)
    {
        return Err(ParseError::InvalidEncoding);
    }
    let tokens = tokenize(input)?;
    let mut tokens = tokens.into_iter();
    let tag = match tokens.next() {
        Some(Token::Atom(tag))
            if tag != "*"
                && tag.len() <= 64
                && tag.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) =>
        {
            tag
        }
        _ => return Err(ParseError::InvalidTag),
    };
    let name = match tokens.next() {
        Some(Token::Atom(name))
            if !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphabetic()) =>
        {
            name.to_ascii_uppercase()
        }
        _ => return Err(ParseError::MissingArgument),
    };
    Ok(CommandLine {
        tag,
        name,
        arguments: tokens.collect(),
    })
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = input.as_bytes();
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b' ' | b'\t' => cursor += 1,
            b'(' => {
                result.push(Token::OpenParen);
                cursor += 1;
            }
            b')' => {
                result.push(Token::CloseParen);
                cursor += 1;
            }
            b'"' => {
                cursor += 1;
                let mut value = Vec::new();
                let mut closed = false;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'"' => {
                            cursor += 1;
                            closed = true;
                            break;
                        }
                        b'\\' => {
                            cursor += 1;
                            let Some(escaped) = bytes.get(cursor).copied() else {
                                return Err(ParseError::InvalidQuotedString);
                            };
                            if escaped < 0x20 || escaped == 0x7f {
                                return Err(ParseError::InvalidQuotedString);
                            }
                            value.push(escaped);
                            cursor += 1;
                        }
                        byte if byte < 0x20 || byte == 0x7f => {
                            return Err(ParseError::InvalidQuotedString);
                        }
                        byte => {
                            value.push(byte);
                            cursor += 1;
                        }
                    }
                }
                if !closed {
                    return Err(ParseError::InvalidQuotedString);
                }
                result.push(Token::Quoted(
                    String::from_utf8(value).map_err(|_| ParseError::InvalidEncoding)?,
                ));
            }
            _ => {
                let start = cursor;
                while cursor < bytes.len() && !matches!(bytes[cursor], b' ' | b'\t' | b'(' | b')') {
                    cursor += 1;
                }
                let atom = std::str::from_utf8(&bytes[start..cursor])
                    .map_err(|_| ParseError::InvalidEncoding)?;
                result.push(Token::Atom(atom.to_owned()));
            }
        }
    }
    Ok(result)
}

pub fn parse_literal_marker(input: &str) -> Result<Option<(usize, bool)>, ParseError> {
    let Some(open) = input.rfind('{') else {
        return Ok(None);
    };
    if !input.ends_with('}') {
        return Ok(None);
    }
    let marker = &input[open + 1..input.len() - 1];
    let (digits, non_synchronizing) = marker
        .strip_suffix('+')
        .map_or((marker, false), |digits| (digits, true));
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseError::InvalidSyntax);
    }
    let size = digits
        .parse::<usize>()
        .map_err(|_| ParseError::LiteralTooLarge)?;
    if size > MAX_LITERAL_BYTES {
        return Err(ParseError::LiteralTooLarge);
    }
    Ok(Some((size, non_synchronizing)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tags_atoms_quoted_strings_and_parentheses() {
        let command = parse_command_line(b"A1 LOGIN \"tenant:alice\" \"pass word\"\r\n").unwrap();
        assert_eq!(command.tag, "A1");
        assert_eq!(command.name, "LOGIN");
        assert_eq!(
            command.arguments,
            vec![
                Token::Quoted("tenant:alice".into()),
                Token::Quoted("pass word".into())
            ]
        );
        assert_eq!(
            parse_command_line(b"a2 STORE 1 +FLAGS (\\Seen \\Flagged)\r\n")
                .unwrap()
                .arguments,
            vec![
                Token::Atom("1".into()),
                Token::Atom("+FLAGS".into()),
                Token::OpenParen,
                Token::Atom("\\Seen".into()),
                Token::Atom("\\Flagged".into()),
                Token::CloseParen
            ]
        );
    }

    #[test]
    fn rejects_invalid_encoding_controls_and_tags() {
        assert_eq!(
            parse_command_line(b"A1 NOOP\xff\r\n"),
            Err(ParseError::InvalidEncoding)
        );
        assert_eq!(
            parse_command_line(b"* NOOP\r\n"),
            Err(ParseError::InvalidTag)
        );
        assert_eq!(
            parse_command_line(b"A1 NOOP\0\r\n"),
            Err(ParseError::InvalidEncoding)
        );
    }

    #[test]
    fn parses_bounded_synchronizing_literals_only() {
        assert_eq!(
            parse_literal_marker("A1 APPEND INBOX {123}").unwrap(),
            Some((123, false))
        );
        assert_eq!(
            parse_literal_marker("A1 APPEND INBOX {123+}").unwrap(),
            Some((123, true))
        );
        assert_eq!(parse_literal_marker("A1 LOGIN user pass").unwrap(), None);
        assert_eq!(
            parse_literal_marker(&format!("A1 APPEND INBOX {{{}}}", MAX_LITERAL_BYTES + 1)),
            Err(ParseError::LiteralTooLarge)
        );
    }
}
