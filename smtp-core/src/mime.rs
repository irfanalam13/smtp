use mail_parser::{MessageParser, PartFlags};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedAttachment {
    pub filename: Option<String>,
    pub content_type: String,
    pub content_id: Option<String>,
    pub inline: bool,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParsedMessage {
    pub subject: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub body_text: Vec<String>,
    pub body_html: Vec<String>,
    pub attachments: Vec<ParsedAttachment>,
    pub parse_warnings: usize,
}

impl ParsedMessage {
    pub fn to_search_document(
        &self,
        message_id: uuid::Uuid,
        folder_id: uuid::Uuid,
    ) -> crate::search::SearchDocument {
        crate::search::SearchDocument {
            message_id,
            folder_id,
            subject: self.subject.clone().unwrap_or_default(),
            sender: self.from.first().cloned().unwrap_or_default(),
            recipients: self.to.iter().chain(&self.cc).cloned().collect(),
            body: self.body_text.join("\n"),
            attachment_names: self
                .attachments
                .iter()
                .filter_map(|attachment| attachment.filename.clone())
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MimeLimits {
    pub max_message_bytes: usize,
    pub max_parts: usize,
    pub max_decoded_bytes: usize,
    pub max_attachments: usize,
}

impl Default for MimeLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: 10 * 1024 * 1024,
            max_parts: 1_000,
            max_decoded_bytes: 20 * 1024 * 1024,
            max_attachments: 100,
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MimeError {
    #[error("message exceeds MIME parser input limit")]
    MessageTooLarge,
    #[error("message is not a parseable RFC 5322 message")]
    InvalidMessage,
    #[error("message exceeds MIME part limit")]
    TooManyParts,
    #[error("message exceeds attachment count limit")]
    TooManyAttachments,
    #[error("decoded MIME content exceeds configured limit")]
    DecodedContentTooLarge,
}

pub fn parse_message(raw: &[u8], limits: MimeLimits) -> Result<ParsedMessage, MimeError> {
    if raw.len() > limits.max_message_bytes {
        return Err(MimeError::MessageTooLarge);
    }
    let parser = MessageParser::new()
        .max_depth(32)
        .max_parts(limits.max_parts);
    let message = parser.parse(raw).ok_or(MimeError::InvalidMessage)?;
    let root = message.root();
    let parts: Vec<_> = message.parts().collect();
    if parts.len() > limits.max_parts
        || parts
            .iter()
            .any(|part| part.flags().contains(PartFlags::LIMIT_REACHED))
    {
        return Err(MimeError::TooManyParts);
    }

    let mut parsed = ParsedMessage {
        subject: message.subject().map(ToOwned::to_owned),
        message_id: message.message_id().map(ToOwned::to_owned),
        in_reply_to: message.in_reply_to().map(|references| {
            references
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        }),
        references: message
            .references()
            .map(|references| references.iter().map(ToString::to_string).collect())
            .unwrap_or_default(),
        from: address_strings(message.from()),
        to: address_strings(message.to()),
        cc: address_strings(message.cc()),
        ..ParsedMessage::default()
    };

    let mut decoded_total = 0usize;
    for part in root.text_body() {
        if part.has_problems() {
            parsed.parse_warnings += 1;
        }
        if let Some((text, problems)) = part.text_checked() {
            parsed.parse_warnings += usize::from(!problems.is_empty());
            add_decoded_size(&mut decoded_total, text.len(), limits.max_decoded_bytes)?;
            parsed.body_text.push(text.into_owned());
        }
    }
    for part in root.html_body() {
        if part.has_problems() {
            parsed.parse_warnings += 1;
        }
        if let Some((html, problems)) = part.text_checked() {
            parsed.parse_warnings += usize::from(!problems.is_empty());
            add_decoded_size(&mut decoded_total, html.len(), limits.max_decoded_bytes)?;
            parsed.body_html.push(html.into_owned());
        }
    }

    for part in parts
        .iter()
        .copied()
        .filter(|part| part.is_attachment() || part.attachment_name().is_some())
    {
        if parsed.attachments.len() >= limits.max_attachments {
            return Err(MimeError::TooManyAttachments);
        }
        if part.has_problems() {
            parsed.parse_warnings += 1;
        }
        let (content, problems) = part.decoded_checked();
        parsed.parse_warnings += usize::from(!problems.is_empty());
        add_decoded_size(&mut decoded_total, content.len(), limits.max_decoded_bytes)?;
        let content_type = part
            .content_type()
            .map(|value| {
                format!(
                    "{}/{}",
                    value.ctype(),
                    value.subtype().unwrap_or("octet-stream")
                )
            })
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        parsed.attachments.push(ParsedAttachment {
            filename: part.attachment_name().map(ToOwned::to_owned),
            content_type,
            content_id: part.content_id().map(ToOwned::to_owned),
            inline: part.is_inline(),
            content: content.into_owned(),
        });
    }
    Ok(parsed)
}

fn add_decoded_size(total: &mut usize, size: usize, maximum: usize) -> Result<(), MimeError> {
    *total = total
        .checked_add(size)
        .filter(|value| *value <= maximum)
        .ok_or(MimeError::DecodedContentTooLarge)?;
    Ok(())
}

fn address_strings(addresses: Option<mail_parser::AddressList<'_>>) -> Vec<String> {
    addresses
        .into_iter()
        .flat_map(|list| list.mailboxes())
        .filter_map(|mailbox| mailbox.address().map(ToOwned::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTIPART: &[u8] = b"From: Sender <sender@example.test>\r\n\
To: Reader <reader@example.test>\r\n\
Subject: =?UTF-8?Q?R=C3=A9sum=C3=A9?=\r\n\
Message-ID: <message-1@example.test>\r\n\
Content-Type: multipart/mixed; boundary=\"part-boundary\"\r\n\
\r\n\
--part-boundary\r\n\
Content-Type: multipart/alternative; boundary=\"body-boundary\"\r\n\
\r\n\
--body-boundary\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
Hello=2C world!\r\n\
--body-boundary\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>Hello, world!</p>\r\n\
--body-boundary--\r\n\
--part-boundary\r\n\
Content-Type: image/png; name*=UTF-8''logo.png\r\n\
Content-Disposition: inline; filename*=UTF-8''logo.png\r\n\
Content-ID: <logo@example.test>\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
aW1hZ2UtYnl0ZXM=\r\n\
--part-boundary\r\n\
Content-Type: application/octet-stream\r\n\
Content-Disposition: attachment; filename=\"data.bin\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
YXR0YWNobWVudA==\r\n\
--part-boundary--\r\n";

    #[test]
    fn parses_nested_mime_encoded_headers_and_attachments() {
        let parsed = parse_message(MULTIPART, MimeLimits::default()).unwrap();
        assert_eq!(parsed.subject.as_deref(), Some("Résumé"));
        assert_eq!(parsed.message_id.as_deref(), Some("message-1@example.test"));
        assert_eq!(parsed.from, vec!["sender@example.test"]);
        assert_eq!(parsed.to, vec!["reader@example.test"]);
        assert!(parsed.body_text[0].contains("Hello, world!"));
        assert!(parsed.body_html[0].contains("<p>Hello, world!</p>"));
        assert_eq!(parsed.attachments.len(), 2);
        assert!(parsed.attachments[0].inline);
        assert_eq!(parsed.attachments[0].filename.as_deref(), Some("logo.png"));
        assert_eq!(parsed.attachments[0].content, b"image-bytes");
        assert_eq!(parsed.attachments[1].content, b"attachment");
        let search_document = parsed.to_search_document(uuid::Uuid::nil(), uuid::Uuid::nil());
        assert_eq!(search_document.sender, "sender@example.test");
        assert_eq!(search_document.recipients, vec!["reader@example.test"]);
        assert_eq!(
            search_document.attachment_names,
            vec!["logo.png", "data.bin"]
        );
    }

    #[test]
    fn enforces_input_parts_and_decoded_content_limits() {
        assert_eq!(
            parse_message(
                MULTIPART,
                MimeLimits {
                    max_message_bytes: 8,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::MessageTooLarge)
        );
        assert_eq!(
            parse_message(
                MULTIPART,
                MimeLimits {
                    max_attachments: 1,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::TooManyAttachments)
        );
        assert_eq!(
            parse_message(
                MULTIPART,
                MimeLimits {
                    max_decoded_bytes: 5,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::DecodedContentTooLarge)
        );
    }
}
