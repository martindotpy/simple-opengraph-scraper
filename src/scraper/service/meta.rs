use super::{bytes::bytes_equal_insensitive, entities::decode_entities};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    Title,
    Description,
    Image,
}

/// Parses `<meta ...>` for property|name + content (any order, either quotes, case-insensitive).
/// is_open_graph=false only for `name=description` (fallback).
pub(crate) fn parse_meta(tag: &[u8]) -> Option<(Field, String, bool)> {
    let mut property: Option<String> = None;
    let mut content: Option<String> = None;
    let mut index = 5; // skip "<meta"

    while index < tag.len() {
        // Skip separators and closing
        while index < tag.len()
            && (tag[index].is_ascii_whitespace() || tag[index] == b'/' || tag[index] == b'>')
        {
            index += 1;
        }

        if index >= tag.len() || tag[index] == b'<' {
            break;
        }

        let name_start = index;

        while index < tag.len()
            && (tag[index].is_ascii_alphanumeric()
                || tag[index] == b'-'
                || tag[index] == b'_'
                || tag[index] == b':')
        {
            index += 1;
        }

        if name_start == index {
            index += 1;
            continue;
        }

        let name = &tag[name_start..index];

        while index < tag.len() && tag[index].is_ascii_whitespace() {
            index += 1;
        }

        if index >= tag.len() || tag[index] != b'=' {
            continue;
        }

        index += 1;

        while index < tag.len() && tag[index].is_ascii_whitespace() {
            index += 1;
        }

        if index >= tag.len() {
            break;
        }

        let value: String = if tag[index] == b'"' || tag[index] == b'\'' {
            let quote = tag[index];
            index += 1;
            let value_start = index;

            while index < tag.len() && tag[index] != quote {
                index += 1;
            }

            let value_text = String::from_utf8_lossy(&tag[value_start..index]).into_owned();
            index += 1; // skip quote
            value_text
        } else {
            let value_start = index;

            while index < tag.len()
                && !tag[index].is_ascii_whitespace()
                && tag[index] != b'>'
                && tag[index] != b'"'
                && tag[index] != b'\''
            {
                index += 1;
            }

            String::from_utf8_lossy(&tag[value_start..index]).into_owned()
        };

        if bytes_equal_insensitive(name, b"property") || bytes_equal_insensitive(name, b"name") {
            property = Some(value.to_ascii_lowercase());
        } else if bytes_equal_insensitive(name, b"content") {
            content = Some(decode_entities(&value));
        }
    }

    let property_value = property?;
    let content_value = content?;

    match property_value.as_str() {
        "og:title" | "twitter:title" => Some((Field::Title, content_value, true)),
        "og:description" | "twitter:description" => Some((Field::Description, content_value, true)),
        "description" => Some((Field::Description, content_value, false)),
        "og:image" | "twitter:image" => Some((Field::Image, content_value, true)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_orders_and_quotes() {
        let cases: &[(&[u8], Field, &str, bool)] = &[
            (
                br#"<meta property="og:title" content="Hello">"#,
                Field::Title,
                "Hello",
                true,
            ),
            (
                br#"<META CONTENT='World' NAME='twitter:title'>"#,
                Field::Title,
                "World",
                true,
            ),
        ];

        for (tag, expected_kind, expected_value, expected_open_graph) in cases {
            let (kind, value, is_open_graph) = parse_meta(tag).unwrap();

            assert_eq!(kind, *expected_kind);
            assert_eq!(value, *expected_value);
            assert_eq!(is_open_graph, *expected_open_graph);
        }

        assert!(parse_meta(br#"<meta property="og:title">"#).is_none());
    }

    #[test]
    fn parses_unquoted_values() {
        let (kind, value, is_open_graph) =
            parse_meta(br#"<meta property=og:title content=Hello>"#).unwrap();

        assert_eq!(kind, Field::Title);
        assert_eq!(value, "Hello");
        assert!(is_open_graph);
    }

    #[test]
    fn name_description_is_fallback() {
        let (kind, value, is_open_graph) =
            parse_meta(br#"<meta name="description" content="Plain">"#).unwrap();

        assert_eq!(kind, Field::Description);
        assert_eq!(value, "Plain");
        assert!(!is_open_graph);
    }

    #[test]
    fn rejects_unknown_properties() {
        assert!(parse_meta(br#"<meta property="og:video" content="x">"#).is_none());
        assert!(parse_meta(br#"<meta name="viewport" content="x">"#).is_none());
    }

    #[test]
    fn decodes_entities_in_content() {
        let (_, value, _) =
            parse_meta(br#"<meta property="og:title" content="a &amp; b">"#).unwrap();

        assert_eq!(value, "a & b");
    }
}
