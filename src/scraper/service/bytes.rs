pub(crate) fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|byte| *byte == needle)
}

pub(crate) fn bytes_equal_insensitive(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
}

pub(crate) fn find_insensitive(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| bytes_equal_insensitive(window, needle))
}

pub(crate) fn starts_with_insensitive(haystack: &[u8], prefix: &[u8]) -> bool {
    haystack.len() >= prefix.len() && bytes_equal_insensitive(&haystack[..prefix.len()], prefix)
}

pub(crate) fn is_split_tag_prefix(tail: &[u8]) -> bool {
    const CANDIDATES: &[&[u8]] = &[b"<meta", b"<title", b"</title", b"</head", b"<head"];

    // Both sides lowered per byte: no allocation on this hot path
    !tail.is_empty() && CANDIDATES.iter().any(|candidate| {
        candidate.len() >= tail.len()
            && candidate[..tail.len()]
                .iter()
                .zip(tail.iter())
                .all(|(candidate_byte, tail_byte)| {
                    candidate_byte.eq_ignore_ascii_case(tail_byte)
                })
    })
}

pub(crate) fn trailing_prefix_len(buffer: &[u8]) -> usize {
    let max = buffer.len().min(8);
    for length in (1..=max).rev() {
        if is_split_tag_prefix(&buffer[buffer.len() - length..]) {
            return length;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_case_insensitive() {
        assert_eq!(find_insensitive(b"<HEAD><META", b"<meta"), Some(6));
        assert_eq!(find_insensitive(b"<head>", b"</title"), None);
        assert_eq!(find_insensitive(b"abc", b""), None);
        assert_eq!(find_insensitive(b"ab", b"abcd"), None);
    }

    #[test]
    fn detects_split_prefixes() {
        assert!(is_split_tag_prefix(b"<"));
        assert!(is_split_tag_prefix(b"</"));
        assert!(is_split_tag_prefix(b"<M"));
        assert!(is_split_tag_prefix(b"<META"));
        assert!(!is_split_tag_prefix(b""));
        assert!(!is_split_tag_prefix(b"plain text"));
        assert!(!is_split_tag_prefix(b"<div"));
    }

    #[test]
    fn measures_trailing_prefix() {
        // Only suffixes starting with '<' can split a tag
        assert_eq!(trailing_prefix_len(b"abc</title"), 7);
        assert_eq!(trailing_prefix_len(b"text</head"), 6);
        assert_eq!(trailing_prefix_len(b"<head><meta pro"), 0);
        assert_eq!(trailing_prefix_len(b"plain text"), 0);
        assert_eq!(trailing_prefix_len(b""), 0);
    }

    #[test]
    fn starts_with_ignores_case() {
        assert!(starts_with_insensitive(b"<META x>", b"<meta"));
        assert!(!starts_with_insensitive(b"<m", b"<meta"));
    }
}
