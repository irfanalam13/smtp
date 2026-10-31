use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use mail_parser::{MessageParser, PartFlags};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    sync::Semaphore,
    time::{Instant as TokioInstant, timeout_at},
};

pub const CANONICAL_MESSAGE_SCHEMA_VERSION: u16 = 1;
pub const MAX_CANONICAL_MESSAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_HEADER_BYTES: usize = 256 * 1024;
const MAX_HEADER_COUNT: usize = 200;
const MAX_MIME_DEPTH: usize = 16;
const MAX_MIME_PARTS: usize = 1_000;
const MAX_DECODED_BYTES: usize = 20 * 1024 * 1024;
const MAX_MIME_ATTACHMENTS: usize = 100;
const MAX_EXPANSION_RATIO: usize = 20;
const MAX_PARSE_TIME: Duration = Duration::from_secs(2);
const MAX_INDEXED_TEXT_BYTES: usize = 256 * 1024;
const MAX_INDEXED_METADATA_BYTES: usize = 768 * 1024;
static MIME_PARSE_PERMITS: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ParsedAttachment {
    pub filename: Option<String>,
    pub content_type: String,
    pub content_id: Option<String>,
    pub inline: bool,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageEnvelope {
    pub sender: String,
    pub recipients: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalMessage {
    schema_version: u16,
    envelope: MessageEnvelope,
    raw_message: Vec<u8>,
    parsed: IndexedMessageMetadata,
}

impl CanonicalMessage {
    pub fn parse(
        envelope: MessageEnvelope,
        raw_message: Vec<u8>,
        limits: MimeLimits,
    ) -> Result<Self, MimeError> {
        let parsed = parse_message(&raw_message, limits)?.to_indexed_metadata();
        Ok(Self {
            schema_version: CANONICAL_MESSAGE_SCHEMA_VERSION,
            envelope,
            raw_message,
            parsed,
        })
    }

    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn envelope(&self) -> &MessageEnvelope {
        &self.envelope
    }

    pub fn raw_message(&self) -> &[u8] {
        &self.raw_message
    }

    pub fn parsed(&self) -> &IndexedMessageMetadata {
        &self.parsed
    }

    pub fn indexed_metadata(&self) -> IndexedMessageMetadata {
        self.parsed.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IndexedAttachmentMetadata {
    pub filename: Option<String>,
    pub content_type: String,
    pub content_id: Option<String>,
    pub inline: bool,
    pub decoded_size: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct IndexedMessageMetadata {
    pub subject: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub body_text: Vec<String>,
    pub body_html: Vec<String>,
    pub attachments: Vec<IndexedAttachmentMetadata>,
    pub parse_warnings: usize,
}

impl ParsedMessage {
    pub fn to_indexed_metadata(&self) -> IndexedMessageMetadata {
        let mut remaining = MAX_INDEXED_TEXT_BYTES;
        IndexedMessageMetadata {
            subject: self.subject.clone(),
            message_id: self.message_id.clone(),
            in_reply_to: self.in_reply_to.clone(),
            references: self.references.clone(),
            from: self.from.clone(),
            to: self.to.clone(),
            cc: self.cc.clone(),
            body_text: bounded_text_parts(&self.body_text, &mut remaining),
            body_html: bounded_text_parts(&self.body_html, &mut remaining),
            attachments: self
                .attachments
                .iter()
                .map(|attachment| IndexedAttachmentMetadata {
                    filename: attachment.filename.clone(),
                    content_type: attachment.content_type.clone(),
                    content_id: attachment.content_id.clone(),
                    inline: attachment.inline,
                    decoded_size: attachment.content.len(),
                })
                .collect(),
            parse_warnings: self.parse_warnings,
        }
    }

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
    pub max_header_bytes: usize,
    pub max_header_count: usize,
    pub max_depth: usize,
    pub max_parts: usize,
    pub max_decoded_bytes: usize,
    pub max_attachments: usize,
    pub max_expansion_ratio: usize,
    pub max_parse_time: Duration,
}

impl Default for MimeLimits {
    fn default() -> Self {
        Self {
            max_message_bytes: MAX_CANONICAL_MESSAGE_BYTES,
            max_header_bytes: 256 * 1024,
            max_header_count: 200,
            max_depth: 16,
            max_parts: 1_000,
            max_decoded_bytes: 20 * 1024 * 1024,
            max_attachments: 100,
            max_expansion_ratio: 20,
            max_parse_time: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MimeError {
    #[error("message exceeds MIME parser input limit")]
    MessageTooLarge,
    #[error("message headers exceed configured byte limit")]
    HeadersTooLarge,
    #[error("message exceeds configured header count limit")]
    TooManyHeaders,
    #[error("message is not a parseable RFC 5322 message")]
    InvalidMessage,
    #[error("message exceeds MIME part limit")]
    TooManyParts,
    #[error("message exceeds attachment count limit")]
    TooManyAttachments,
    #[error("decoded MIME content exceeds configured limit")]
    DecodedContentTooLarge,
    #[error("decoded MIME content exceeds configured expansion ratio")]
    ExpansionRatioExceeded,
    #[error("indexed message metadata exceeds configured storage limit")]
    IndexedMetadataTooLarge,
    #[error("MIME parsing exceeded configured time limit")]
    ParseTimedOut,
    #[error("MIME parser worker failed: {0}")]
    ParseWorkerFailed(String),
}

pub async fn parse_canonical_message(
    envelope: MessageEnvelope,
    raw_message: Vec<u8>,
    mut limits: MimeLimits,
) -> Result<CanonicalMessage, MimeError> {
    limits.max_parse_time = limits.max_parse_time.min(MAX_PARSE_TIME);
    if limits.max_parse_time.is_zero() {
        return Err(MimeError::ParseTimedOut);
    }
    let deadline = TokioInstant::now() + limits.max_parse_time;
    let permits = MIME_PARSE_PERMITS
        .get_or_init(|| Arc::new(Semaphore::new(4)))
        .clone();
    let permit = timeout_at(deadline, permits.acquire_owned())
        .await
        .map_err(|_| MimeError::ParseTimedOut)?
        .map_err(|error| MimeError::ParseWorkerFailed(error.to_string()))?;
    let task = tokio::task::spawn_blocking(move || {
        // Keep capacity reserved until a timed-out blocking parser actually exits.
        let _permit = permit;
        CanonicalMessage::parse(envelope, raw_message, limits)
    });
    match timeout_at(deadline + Duration::from_millis(50), task).await {
        Err(_) => Err(MimeError::ParseTimedOut),
        Ok(Err(error)) => Err(MimeError::ParseWorkerFailed(error.to_string())),
        Ok(Ok(result)) => result,
    }
}

pub fn parse_message(raw: &[u8], limits: MimeLimits) -> Result<ParsedMessage, MimeError> {
    let limits = MimeLimits {
        max_message_bytes: limits.max_message_bytes.min(MAX_CANONICAL_MESSAGE_BYTES),
        max_header_bytes: limits.max_header_bytes.min(MAX_HEADER_BYTES),
        max_header_count: limits.max_header_count.min(MAX_HEADER_COUNT),
        max_depth: limits.max_depth.min(MAX_MIME_DEPTH),
        max_parts: limits.max_parts.min(MAX_MIME_PARTS),
        max_decoded_bytes: limits.max_decoded_bytes.min(MAX_DECODED_BYTES),
        max_attachments: limits.max_attachments.min(MAX_MIME_ATTACHMENTS),
        max_expansion_ratio: limits.max_expansion_ratio.min(MAX_EXPANSION_RATIO),
        max_parse_time: limits.max_parse_time.min(MAX_PARSE_TIME),
    };
    if raw.len() > limits.max_message_bytes {
        return Err(MimeError::MessageTooLarge);
    }
    if limits.max_header_bytes == 0
        || limits.max_header_count == 0
        || limits.max_depth == 0
        || limits.max_parts == 0
        || limits.max_expansion_ratio == 0
        || limits.max_parse_time.is_zero()
    {
        return Err(MimeError::InvalidMessage);
    }
    let started_at = Instant::now();
    validate_header_limits(raw, limits)?;
    if started_at.elapsed() > limits.max_parse_time {
        return Err(MimeError::ParseTimedOut);
    }
    let parser = MessageParser::new()
        .max_depth(limits.max_depth)
        .max_parts(limits.max_parts);
    let message = parser.parse(raw).ok_or(MimeError::InvalidMessage)?;
    if started_at.elapsed() > limits.max_parse_time {
        return Err(MimeError::ParseTimedOut);
    }
    let root = message.root();
    let parts: Vec<_> = message.parts().collect();
    if parts.len() > limits.max_parts
        || parts
            .iter()
            .any(|part| part.flags().contains(PartFlags::LIMIT_REACHED))
    {
        return Err(MimeError::TooManyParts);
    }
    if message.header_count() > limits.max_header_count {
        return Err(MimeError::TooManyHeaders);
    }
    let mut total_header_bytes = 0usize;
    for part in &parts {
        for header in part.headers() {
            total_header_bytes = total_header_bytes
                .checked_add(header.raw_name().len())
                .and_then(|size| size.checked_add(header.raw_value().len()))
                .and_then(|size| size.checked_add(4))
                .filter(|size| *size <= limits.max_header_bytes)
                .ok_or(MimeError::HeadersTooLarge)?;
        }
    }

    let attachment_parts = parts
        .iter()
        .copied()
        .filter(|part| part.is_attachment() || part.attachment_name().is_some())
        .collect::<Vec<_>>();
    if attachment_parts.len() > limits.max_attachments {
        return Err(MimeError::TooManyAttachments);
    }

    let mut decoded_input_total = 0usize;
    for part in &parts {
        if !part.is_multipart() {
            add_decoded_size(
                &mut decoded_input_total,
                part.decoded_len(),
                limits.max_decoded_bytes,
            )?;
        }
    }
    if decoded_input_total > raw.len().saturating_mul(limits.max_expansion_ratio) {
        return Err(MimeError::ExpansionRatioExceeded);
    }
    if started_at.elapsed() > limits.max_parse_time {
        return Err(MimeError::ParseTimedOut);
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

    parsed.parse_warnings = parts.iter().filter(|part| part.has_problems()).count();
    let mut decoded_total = 0usize;
    for part in root.text_body() {
        if let Some((text, problems)) = part.text_checked() {
            parsed.parse_warnings += usize::from(!problems.is_empty());
            add_decoded_size(&mut decoded_total, text.len(), limits.max_decoded_bytes)?;
            parsed.body_text.push(text.into_owned());
        }
    }
    for part in root.html_body() {
        if let Some((html, problems)) = part.text_checked() {
            parsed.parse_warnings += usize::from(!problems.is_empty());
            add_decoded_size(&mut decoded_total, html.len(), limits.max_decoded_bytes)?;
            parsed.body_html.push(html.into_owned());
        }
    }

    for part in attachment_parts {
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
    if decoded_total > raw.len().saturating_mul(limits.max_expansion_ratio) {
        return Err(MimeError::ExpansionRatioExceeded);
    }
    let indexed_metadata = parsed.to_indexed_metadata();
    if serde_json::to_vec(&indexed_metadata)
        .map_err(|_| MimeError::IndexedMetadataTooLarge)?
        .len()
        > MAX_INDEXED_METADATA_BYTES
    {
        return Err(MimeError::IndexedMetadataTooLarge);
    }
    if started_at.elapsed() > limits.max_parse_time {
        return Err(MimeError::ParseTimedOut);
    }
    Ok(parsed)
}

fn validate_header_limits(raw: &[u8], limits: MimeLimits) -> Result<(), MimeError> {
    let mut header_bytes = 0usize;
    let mut header_count = 0usize;
    for line in raw.split_inclusive(|byte| *byte == b'\n') {
        let content = line.strip_suffix(b"\n").unwrap_or(line);
        let content = content.strip_suffix(b"\r").unwrap_or(content);
        if content.is_empty() {
            return Ok(());
        }
        header_bytes = header_bytes
            .checked_add(line.len())
            .filter(|value| *value <= limits.max_header_bytes)
            .ok_or(MimeError::HeadersTooLarge)?;
        if !matches!(content.first(), Some(b' ' | b'\t')) {
            header_count += 1;
            if header_count > limits.max_header_count {
                return Err(MimeError::TooManyHeaders);
            }
        }
    }
    Err(MimeError::InvalidMessage)
}

fn bounded_text_parts(parts: &[String], remaining: &mut usize) -> Vec<String> {
    let mut bounded = Vec::new();
    for part in parts {
        if *remaining == 0 {
            break;
        }
        let mut end = part.len().min(*remaining);
        while !part.is_char_boundary(end) {
            end -= 1;
        }
        bounded.push(part[..end].to_owned());
        *remaining -= end;
        if end < part.len() {
            break;
        }
    }
    bounded
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
    use base64::Engine as _;

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

    #[test]
    fn rejects_large_transfer_decoded_attachment_before_decoding() {
        let payload = vec![b'x'; 1024 * 1024];
        let encoded = base64::engine::general_purpose::STANDARD.encode(payload);
        let raw = format!(
            "Content-Type: application/octet-stream\r\n\
Content-Disposition: attachment; filename=\"large.bin\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n{encoded}\r\n"
        );
        assert_eq!(
            parse_message(
                raw.as_bytes(),
                MimeLimits {
                    max_decoded_bytes: 1024,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::DecodedContentTooLarge)
        );
    }

    #[test]
    fn canonical_message_keeps_envelope_separate_and_raw_bytes_exact() {
        let raw = MULTIPART.to_vec();
        let canonical = CanonicalMessage::parse(
            MessageEnvelope {
                sender: "bounce@example.test".into(),
                recipients: vec!["mailbox@example.test".into()],
            },
            raw.clone(),
            MimeLimits::default(),
        )
        .unwrap();

        assert_eq!(canonical.schema_version(), CANONICAL_MESSAGE_SCHEMA_VERSION);
        assert_eq!(canonical.envelope().sender, "bounce@example.test");
        assert_eq!(
            canonical.envelope().recipients,
            vec!["mailbox@example.test"]
        );
        assert_eq!(canonical.parsed().from, vec!["sender@example.test"]);
        assert_eq!(canonical.parsed().to, vec!["reader@example.test"]);
        assert_eq!(canonical.raw_message(), raw);

        let metadata = canonical.indexed_metadata();
        assert_eq!(metadata.subject.as_deref(), Some("Résumé"));
        assert_eq!(metadata.attachments.len(), 2);
        assert_eq!(metadata.attachments[0].decoded_size, b"image-bytes".len());
    }

    #[test]
    fn enforces_header_byte_and_count_limits() {
        assert_eq!(
            parse_message(
                b"Subject: hello\r\nX-Test: yes\r\n\r\nbody",
                MimeLimits {
                    max_header_count: 1,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::TooManyHeaders)
        );
        assert_eq!(
            parse_message(
                b"Subject: hello\r\n\r\nbody",
                MimeLimits {
                    max_header_bytes: 8,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::HeadersTooLarge)
        );
        assert!(parse_message(b"\r\nbody", MimeLimits::default()).is_ok());
    }

    #[test]
    fn unfolds_headers_and_reports_malformed_transfer_encoding() {
        let folded = parse_message(
            b"Subject: folded\r\n\tRFC 5322 header\r\n\r\nbody\r\n",
            MimeLimits::default(),
        )
        .unwrap();
        assert_eq!(folded.subject.as_deref(), Some("folded RFC 5322 header"));

        let malformed = parse_message(
            b"Content-Type: text/plain\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
%%%not-base64%%%\r\n",
            MimeLimits::default(),
        )
        .unwrap();
        assert!(malformed.parse_warnings > 0);
    }

    #[test]
    fn enforces_nested_part_and_expansion_limits() {
        assert_eq!(
            parse_message(
                MULTIPART,
                MimeLimits {
                    max_parts: 1,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::TooManyParts)
        );
        assert_eq!(
            parse_message(
                MULTIPART,
                MimeLimits {
                    max_depth: 1,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::TooManyParts)
        );
    }

    #[test]
    fn enforces_limits_across_nested_message_headers() {
        let nested = b"Subject: outer\r\n\
Content-Type: message/rfc822\r\n\
\r\n\
From: sender@example.test\r\n\
To: reader@example.test\r\n\
Subject: nested\r\n\
\r\n\
body";
        assert_eq!(
            parse_message(
                nested,
                MimeLimits {
                    max_header_count: 4,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::TooManyHeaders)
        );

        let mut nested_headers = b"Subject: outer\r\n\
Content-Type: message/rfc822\r\n\
\r\n\
X-Nested: "
            .to_vec();
        nested_headers.extend(std::iter::repeat_n(b'x', 100));
        nested_headers.extend_from_slice(b"\r\n\r\nbody");
        assert_eq!(
            parse_message(
                &nested_headers,
                MimeLimits {
                    max_header_bytes: 120,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::HeadersTooLarge)
        );
    }

    #[test]
    fn caller_cannot_raise_the_absolute_message_size_limit() {
        let raw = vec![b'x'; MAX_CANONICAL_MESSAGE_BYTES + 1];
        assert_eq!(
            parse_message(
                &raw,
                MimeLimits {
                    max_message_bytes: usize::MAX,
                    ..MimeLimits::default()
                }
            ),
            Err(MimeError::MessageTooLarge)
        );
    }

    #[tokio::test]
    async fn rejects_a_zero_parse_deadline_before_queueing_work() {
        let result = parse_canonical_message(
            MessageEnvelope {
                sender: String::new(),
                recipients: Vec::new(),
            },
            b"\r\nbody".to_vec(),
            MimeLimits {
                max_parse_time: Duration::ZERO,
                ..MimeLimits::default()
            },
        )
        .await;
        assert_eq!(result, Err(MimeError::ParseTimedOut));
    }

    #[test]
    fn indexed_body_text_is_bounded_without_truncating_utf8() {
        let text = "é".repeat(MAX_INDEXED_TEXT_BYTES);
        let parsed = ParsedMessage {
            body_text: vec![text],
            ..ParsedMessage::default()
        };
        let indexed = parsed.to_indexed_metadata();
        assert_eq!(
            indexed.body_text.iter().map(String::len).sum::<usize>(),
            MAX_INDEXED_TEXT_BYTES
        );
        assert!(indexed.body_text[0].is_char_boundary(indexed.body_text[0].len()));
    }
}
