pub(crate) fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }

    let mut output = String::with_capacity(text.len());
    let mut index = 0;

    while index < text.len() {
        // Advance full chars to keep multibyte UTF-8 intact
        if text.as_bytes()[index] != b'&' {
            let character = text[index..].chars().next().unwrap_or('\u{FFFD}');

            output.push(character);

            index += character.len_utf8();

            continue;
        }

        let rest = &text[index..];
        if let Some(end) = rest.find(';')
            && end <= 10
            && push_entity(&rest[..=end], &mut output)
        {
            index += end + 1;

            continue;
        }

        output.push('&');
        index += 1;
    }

    output
}

/// Pushes the decoded entity: false means not an entity, keep the raw '&'.
fn push_entity(entity: &str, output: &mut String) -> bool {
    match entity {
        "&amp;" => output.push('&'),
        "&lt;" => output.push('<'),
        "&gt;" => output.push('>'),
        "&quot;" => output.push('"'),
        "&#39;" | "&#x27;" | "&#X27;" => output.push('\''),
        _ if entity.starts_with("&#") => {
            let number_text = &entity[2..entity.len() - 1];
            let codepoint = match number_text
                .strip_prefix('x')
                .or_else(|| number_text.strip_prefix('X'))
            {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => number_text.parse::<u32>().ok(),
            };

            match codepoint.and_then(char::from_u32) {
                Some(character) => output.push(character),
                None => return false,
            }
        }
        _ => return false,
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_entities() {
        assert_eq!(decode_entities("a&#39;b"), "a'b");
        assert_eq!(decode_entities("a &amp; b"), "a & b");
        assert_eq!(decode_entities("Café &amp; crème"), "Café & crème");
        assert_eq!(decode_entities("GitHub · Change"), "GitHub · Change");
    }

    #[test]
    fn decodes_hex_and_decimal_codepoints() {
        assert_eq!(decode_entities("a&#x27;b"), "a'b");
        assert_eq!(decode_entities("a&#X27;b"), "a'b");
        assert_eq!(decode_entities("&#65;&#66;"), "AB");
    }

    #[test]
    fn leaves_malformed_entities_intact() {
        assert_eq!(decode_entities("&foo;"), "&foo;");
        assert_eq!(decode_entities("&amp"), "&amp");
        assert_eq!(decode_entities("a & b"), "a & b");
        assert_eq!(decode_entities("&abcdefghijk;"), "&abcdefghijk;");
    }
}
