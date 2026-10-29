use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BimiPolicy {
    pub version: String,
    pub logo_url: String,
    pub authority_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BimiResult {
    Pass,
    Fail,
    None,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum BimiError {
    #[error("BIMI record is malformed")]
    Malformed,
    #[error("BIMI URL must use HTTPS")]
    InsecureUrl,
}

pub fn parse_bimi_record(record: &str) -> Result<BimiPolicy, BimiError> {
    let mut version = None;
    let mut logo_url = None;
    let mut authority_url = None;
    for field in record
        .split(';')
        .map(str::trim)
        .filter(|field| !field.is_empty())
    {
        let (key, value) = field.split_once('=').ok_or(BimiError::Malformed)?;
        match key.trim() {
            "v" if version.is_none() => version = Some(value.trim().to_owned()),
            "l" if logo_url.is_none() => logo_url = Some(value.trim().to_owned()),
            "a" if authority_url.is_none() => authority_url = Some(value.trim().to_owned()),
            "v" | "l" | "a" => return Err(BimiError::Malformed),
            _ => {}
        }
    }
    let policy = BimiPolicy {
        version: version.ok_or(BimiError::Malformed)?,
        logo_url: logo_url.ok_or(BimiError::Malformed)?,
        authority_url,
    };
    validate_bimi_policy(&policy)?;
    Ok(policy)
}

pub fn validate_bimi_policy(policy: &BimiPolicy) -> Result<(), BimiError> {
    if policy.version != "BIMI1" {
        return Err(BimiError::Malformed);
    }
    if !valid_https_url(&policy.logo_url)
        || policy
            .authority_url
            .as_ref()
            .is_some_and(|url| !valid_https_url(url))
    {
        return Err(BimiError::InsecureUrl);
    }
    Ok(())
}

fn valid_https_url(url: &str) -> bool {
    let Some(authority) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = authority.split('/').next().unwrap_or_default();
    !authority.is_empty()
        && !authority.contains('@')
        && !url.chars().any(char::is_control)
        && !url.chars().any(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_logo_records() {
        let policy = parse_bimi_record("v=BIMI1; l=https://example.test/logo.svg").unwrap();
        assert_eq!(policy.version, "BIMI1");
    }

    #[test]
    fn rejects_urls_without_a_host_or_with_credentials() {
        assert_eq!(
            parse_bimi_record("v=BIMI1; l=https:///logo.svg"),
            Err(BimiError::InsecureUrl)
        );
        assert_eq!(
            parse_bimi_record("v=BIMI1; l=https://user@example.test/logo.svg"),
            Err(BimiError::InsecureUrl)
        );
    }
}
