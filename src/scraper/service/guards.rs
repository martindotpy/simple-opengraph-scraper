use tracing::warn;

use crate::scraper::{
    domain::ScrapeError,
    service::{bytes::bytes_equal_insensitive, scanner::MAX_BYTES},
};

pub(crate) fn ensure_success(response: &reqwest::Response, host: &str) -> Result<(), ScrapeError> {
    if !response.status().is_success() {
        warn!(%host, status = %response.status(), "upstream returned error status");

        return Err(ScrapeError::Upstream);
    }

    Ok(())
}

pub(crate) fn ensure_size(response: &reqwest::Response, host: &str) -> Result<(), ScrapeError> {
    if is_body_too_large(response.content_length()) {
        warn!(%host, "response exceeds size limit");

        return Err(ScrapeError::TooLarge);
    }

    Ok(())
}

fn is_body_too_large(content_length: Option<u64>) -> bool {
    content_length.is_some_and(|length| length > MAX_BYTES)
}

pub(crate) fn ensure_html(response: &reqwest::Response, host: &str) -> Result<(), ScrapeError> {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or_default()
        .to_lowercase();

    if !is_html_content_type(&content_type) {
        warn!(%host, %content_type, "unsupported media type");

        return Err(ScrapeError::UnsupportedMedia);
    }

    Ok(())
}

/// Empty means no header sent: allow and let the scanner decide.
fn is_html_content_type(content_type: &str) -> bool {
    if content_type.is_empty() {
        return true;
    }

    let essence = content_type.split(';').next().unwrap_or("").trim();

    bytes_equal_insensitive(essence.as_bytes(), b"text/html")
        || bytes_equal_insensitive(essence.as_bytes(), b"application/xhtml+xml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_html_content_types() {
        assert!(is_html_content_type("text/html; charset=utf-8"));
        assert!(is_html_content_type("TEXT/HTML"));
        assert!(is_html_content_type("application/xhtml+xml"));
        assert!(is_html_content_type(""));
    }

    #[test]
    fn rejects_non_html_content_types() {
        assert!(!is_html_content_type("application/json"));
        assert!(!is_html_content_type("image/png"));
        assert!(!is_html_content_type("text/html-evil"));
        assert!(!is_html_content_type("image/png; text/html"));
        assert!(!is_html_content_type("text/htmlfoo"));
    }

    #[test]
    fn rejects_oversized_bodies() {
        assert!(is_body_too_large(Some(MAX_BYTES + 1)));
        assert!(!is_body_too_large(Some(MAX_BYTES)));
        assert!(!is_body_too_large(None));
    }
}
