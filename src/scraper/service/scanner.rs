use super::{
    bytes::{
        find_byte, find_insensitive, is_split_tag_prefix, starts_with_insensitive, trailing_prefix_len,
    },
    entities::decode_entities,
    fetcher::map_fetch_error,
    meta::{Field, parse_meta},
};
use crate::scraper::domain::ScrapeError;
use std::time::Duration;
use tracing::warn;

/// Safety cap
pub(crate) const MAX_BYTES: u64 = 512 * 1024;

const SCAN_WINDOW: usize = 8192;
const CHUNK_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(crate) struct OpengraphHtmlScanner {
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) image: Option<String>,
    pub(crate) total: u64,
    title_is_open_graph: bool,
    description_is_open_graph: bool,
    buffer: Vec<u8>,
}

impl OpengraphHtmlScanner {
    /// Reads chunks until complete or EOF, then cancels the stream.
    pub(crate) async fn collect(
        mut response: reqwest::Response,
        host: &str,
    ) -> Result<Self, ScrapeError> {
        let mut scanner = OpengraphHtmlScanner::default();

        loop {
            let chunk = tokio::time::timeout(CHUNK_TIMEOUT, response.chunk())
                .await
                .map_err(|_| {
                    warn!(%host, "upstream body stalled");

                    ScrapeError::Timeout
                })?
                .map_err(map_fetch_error)
                .map_err(|error| {
                    warn!(%host, error = ?error, "upstream body stream failed");

                    error
                })?;

            let Some(bytes) = chunk else { break }; // EOF

            if scanner.feed(&bytes).map_err(|error| {
                warn!(%host, error = ?error, "response exceeds size limit");

                error
            })? {
                break; // complete
            }
        }

        drop(response);

        Ok(scanner)
    }

    pub(crate) fn feed(&mut self, chunk: &[u8]) -> Result<bool, ScrapeError> {
        self.total += chunk.len() as u64;

        if self.total > MAX_BYTES {
            return Err(ScrapeError::TooLarge);
        }

        self.buffer.reserve(chunk.len());
        self.buffer.extend_from_slice(chunk);

        let mut index = 0;

        while index < self.buffer.len() {
            if self.buffer[index] != b'<' {
                index += 1;
            } else {
                let rest = &self.buffer[index..];

                if starts_with_insensitive(rest, b"<meta") {
                    // Split tag: keep from index
                    let Some(end_index) = find_byte(rest, b'>') else {
                        break;
                    };
                    if let Some((kind, value, is_open_graph)) = parse_meta(&rest[..=end_index]) {
                        self.assign(kind, value, is_open_graph);
                    }

                    index += end_index + 1;
                } else if starts_with_insensitive(rest, b"<title") {
                    // <title ...>content</title> (may span chunks)
                    let Some(open_tag_end) = find_byte(rest, b'>') else {
                        break;
                    };

                    let after_open = &rest[open_tag_end + 1..];

                    match find_insensitive(after_open, b"</title") {
                        None => {
                            // No close yet: wait for more chunks, unless broken <title>
                            if rest.len() > SCAN_WINDOW {
                                index += open_tag_end + 1;
                            } else {
                                break;
                            }
                        }
                        Some(close_tag_offset) => {
                            let inner = &after_open[..close_tag_offset];
                            let title_text = decode_entities(&String::from_utf8_lossy(inner));
                            let close_tag_end =
                                match find_byte(&after_open[close_tag_offset..], b'>') {
                                    Some(end_index) => {
                                        open_tag_end + 1 + close_tag_offset + end_index + 1
                                    }
                                    None => open_tag_end + 1 + close_tag_offset + 7,
                                };
                            let rest_length = rest.len();

                            self.assign(Field::Title, title_text, false);

                            index += close_tag_end.min(rest_length);
                        }
                    }
                } else if starts_with_insensitive(rest, b"</head") {
                    let Some(end_index) = find_byte(rest, b'>') else {
                        break;
                    };

                    index += end_index + 1;
                } else if rest.len() < 7 && is_split_tag_prefix(rest) {
                    // Trailing "<", "</", "<m"...: keep
                    break;
                } else {
                    index += 1;
                }
            }

            if self.is_complete() {
                self.buffer.clear();

                return Ok(true);
            }
        }

        // Drop processed bytes, keep tail for split tags
        if index >= self.buffer.len() {
            let keep = trailing_prefix_len(&self.buffer);

            self.buffer.drain(..self.buffer.len() - keep);
        } else if index > 0 {
            self.buffer.drain(..index);
        }

        // Window never grows (even on GB pages)
        if self.buffer.len() > SCAN_WINDOW {
            let keep = trailing_prefix_len(&self.buffer).clamp(64, SCAN_WINDOW);
            let drop_count = self.buffer.len() - keep;

            self.buffer.drain(..drop_count);
        }

        Ok(false)
    }

    fn is_complete(&self) -> bool {
        self.title.is_some() && self.description.is_some() && self.image.is_some()
    }

    fn assign(&mut self, kind: Field, value: String, is_open_graph: bool) {
        let value = value.trim().to_string();
        if value.is_empty() {
            return;
        }
        match kind {
            Field::Title => Self::assign_text(
                &mut self.title,
                &mut self.title_is_open_graph,
                value,
                is_open_graph,
            ),
            Field::Description => Self::assign_text(
                &mut self.description,
                &mut self.description_is_open_graph,
                value,
                is_open_graph,
            ),
            Field::Image => {
                if self.image.is_none() {
                    self.image = Some(value);
                }
            }
        }
    }

    /// OG overrides fallback, first OG wins.
    fn assign_text(
        slot: &mut Option<String>,
        slot_is_open_graph: &mut bool,
        value: String,
        is_open_graph: bool,
    ) {
        if is_open_graph {
            if slot.is_none() || !*slot_is_open_graph {
                *slot = Some(value);
                *slot_is_open_graph = true;
            }
        } else if slot.is_none() {
            *slot = Some(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_closes_early_and_drops_rest() {
        let mut scanner = OpengraphHtmlScanner::default();
        let done = scanner.feed(br#"<head><meta property="og:title" content="T"><meta property="og:description" content="D">"#).unwrap();
        assert!(!done);
        let done = scanner
            .feed(br#"<meta property="og:image" content="I"></head><body>9999"#)
            .unwrap();
        assert!(done);
        assert_eq!(scanner.title.as_deref(), Some("T"));
    }

    #[test]
    fn scanner_handles_split_tags() {
        let mut scanner = OpengraphHtmlScanner::default();
        assert!(!scanner.feed(br#"<head><meta pro"#).unwrap());
        assert!(!scanner.feed(br#"perty="og:title" con"#).unwrap());
        assert!(!scanner.feed(br#"tent="Split"></head>"#).unwrap());
        assert_eq!(scanner.title.as_deref(), Some("Split"));
    }

    #[test]
    fn scanner_stops_at_limit() {
        let mut scanner = OpengraphHtmlScanner::default();
        let large_input = vec![b'x'; 600 * 1024];
        assert!(matches!(
            scanner.feed(&large_input),
            Err(ScrapeError::TooLarge)
        ));
    }

    #[test]
    fn og_wins_over_title_fallback() {
        let mut scanner = OpengraphHtmlScanner::default();
        assert!(!scanner.feed(br#"<head><title>Fallback</title>"#).unwrap());
        assert_eq!(scanner.title.as_deref(), Some("Fallback"));
        assert!(!scanner.feed(br#"<meta property="og:title" content="Real"><meta property="og:description" content="D">"#).unwrap());
        assert_eq!(scanner.title.as_deref(), Some("Real"));
        let done = scanner
            .feed(br#"<meta property="og:image" content="I">"#)
            .unwrap();
        assert!(done);
    }

    #[test]
    fn window_stays_bounded_on_huge_input() {
        let mut scanner = OpengraphHtmlScanner::default();
        assert!(!scanner.feed(&vec![b'x'; 20 * 1024]).unwrap());
        assert!(scanner.buffer.len() <= 8192);
    }

    #[test]
    fn broken_title_without_close_is_skipped() {
        let mut scanner = OpengraphHtmlScanner::default();
        let mut broken = b"<head><title>".to_vec();
        broken.extend(vec![b'x'; 9000]);
        assert!(!scanner.feed(&broken).unwrap());
        assert_eq!(scanner.title, None);
    }

    #[test]
    fn first_image_wins() {
        let mut scanner = OpengraphHtmlScanner::default();
        assert!(!scanner
            .feed(br#"<head><meta property="og:image" content="first"><meta property="og:image" content="second">"#)
            .unwrap());
        assert_eq!(scanner.image.as_deref(), Some("first"));
    }

    #[test]
    fn empty_values_are_ignored() {
        let mut scanner = OpengraphHtmlScanner::default();
        assert!(
            !scanner
                .feed(br#"<head><meta property="og:title" content="   ">"#)
                .unwrap()
        );
        assert_eq!(scanner.title, None);
    }
}
