use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reply {
    pub code: u16,
    pub lines: Vec<String>,
}

impl Reply {
    pub fn new(code: u16, text: impl Into<String>) -> Self {
        Self {
            code,
            lines: vec![text.into()],
        }
    }

    pub fn multiline(code: u16, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            code,
            lines: lines.into_iter().map(Into::into).collect(),
        }
    }
}

impl fmt::Display for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, line) in self.lines.iter().enumerate() {
            let separator = if index + 1 == self.lines.len() {
                ' '
            } else {
                '-'
            };
            write!(f, "{:03}{separator}{line}\r\n", self.code)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Reply;

    #[test]
    fn formats_multiline_reply() {
        assert_eq!(
            Reply::multiline(250, ["smtp-test", "8BITMIME"]).to_string(),
            "250-smtp-test\r\n250 8BITMIME\r\n"
        );
    }
}
