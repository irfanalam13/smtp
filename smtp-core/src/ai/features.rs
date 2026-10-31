use crate::mime::ParsedMessage;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmailFeatures {
    pub text_bytes: usize,
    pub word_count: usize,
    pub http_link_references: usize,
    pub https_link_references: usize,
    pub bare_web_references: usize,
    pub plain_text_parts: usize,
    pub html_parts: usize,
    pub attachment_count: usize,
    pub executable_like_attachment_count: usize,
    pub parse_warning_count: usize,
}

pub fn extract_email_features(message: &ParsedMessage) -> EmailFeatures {
    let mut features = EmailFeatures {
        plain_text_parts: message.body_text.len(),
        html_parts: message.body_html.len(),
        attachment_count: message.attachments.len(),
        parse_warning_count: message.parse_warnings,
        ..EmailFeatures::default()
    };

    for text in message.body_text.iter().chain(&message.body_html) {
        features.text_bytes = features.text_bytes.saturating_add(text.len());
        features.word_count = features
            .word_count
            .saturating_add(text.split_whitespace().count());
        features.http_link_references = features
            .http_link_references
            .saturating_add(count_case_insensitive(text, "http://"));
        features.https_link_references = features
            .https_link_references
            .saturating_add(count_case_insensitive(text, "https://"));
        features.bare_web_references = features
            .bare_web_references
            .saturating_add(count_case_insensitive(text, "www."));
    }

    features.executable_like_attachment_count = message
        .attachments
        .iter()
        .filter(|attachment| {
            has_executable_like_extension(attachment.filename.as_deref())
                || has_executable_like_media_type(&attachment.content_type)
        })
        .count();

    features
}

fn count_case_insensitive(text: &str, needle: &str) -> usize {
    text.as_bytes()
        .windows(needle.len())
        .filter(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
        .count()
}

fn has_executable_like_extension(filename: Option<&str>) -> bool {
    let Some(extension) =
        filename.and_then(|value| value.rsplit_once('.').map(|(_, extension)| extension))
    else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "bat" | "cmd" | "com" | "exe" | "jar" | "js" | "msi" | "ps1" | "scr" | "vbs"
    )
}

fn has_executable_like_media_type(media_type: &str) -> bool {
    matches!(
        media_type.trim().to_ascii_lowercase().as_str(),
        "application/java-archive"
            | "application/x-dosexec"
            | "application/x-executable"
            | "application/x-msdownload"
            | "application/x-sh"
            | "text/javascript"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mime::ParsedAttachment;

    #[test]
    fn extracts_bounded_aggregate_features_without_copying_message_content() {
        let message = ParsedMessage {
            body_text: vec!["See HTTPS://example.test and http://other.test".to_owned()],
            body_html: vec!["<a href='https://third.test'>www.fourth.test</a>".to_owned()],
            attachments: vec![
                ParsedAttachment {
                    filename: Some("report.pdf".to_owned()),
                    content_type: "application/pdf".to_owned(),
                    content_id: None,
                    inline: false,
                    content: vec![0; 8],
                },
                ParsedAttachment {
                    filename: Some("invoice.exe".to_owned()),
                    content_type: "application/octet-stream".to_owned(),
                    content_id: None,
                    inline: false,
                    content: vec![0; 8],
                },
                ParsedAttachment {
                    filename: None,
                    content_type: "application/x-dosexec".to_owned(),
                    content_id: None,
                    inline: false,
                    content: vec![0; 8],
                },
            ],
            parse_warnings: 2,
            ..ParsedMessage::default()
        };

        assert_eq!(
            extract_email_features(&message),
            EmailFeatures {
                text_bytes: message.body_text[0].len() + message.body_html[0].len(),
                word_count: 6,
                http_link_references: 1,
                https_link_references: 2,
                bare_web_references: 1,
                plain_text_parts: 1,
                html_parts: 1,
                attachment_count: 3,
                executable_like_attachment_count: 2,
                parse_warning_count: 2,
            }
        );
    }

    #[test]
    fn feature_output_contains_no_message_strings_or_attachment_data() {
        let message = ParsedMessage {
            subject: Some("private-subject".to_owned()),
            from: vec!["person@example.test".to_owned()],
            body_text: vec!["secret-body".to_owned()],
            attachments: vec![ParsedAttachment {
                filename: Some("private.exe".to_owned()),
                content_type: "application/octet-stream".to_owned(),
                content_id: None,
                inline: false,
                content: b"secret attachment".to_vec(),
            }],
            ..ParsedMessage::default()
        };

        let features = extract_email_features(&message);
        let debug = format!("{features:?}");
        for private_value in [
            "private-subject",
            "person@example.test",
            "secret-body",
            "private.exe",
            "secret attachment",
        ] {
            assert!(!debug.contains(private_value));
        }
        assert_eq!(features.executable_like_attachment_count, 1);
    }

    #[test]
    fn executable_like_checks_ignore_case_and_require_a_known_extension() {
        assert!(has_executable_like_extension(Some("payload.EXE")));
        assert!(!has_executable_like_extension(Some("report.pdf")));
        assert!(!has_executable_like_extension(Some("no-extension")));
        assert!(has_executable_like_media_type("Application/X-DOSExec"));
        assert!(!has_executable_like_media_type("application/pdf"));
    }
}
