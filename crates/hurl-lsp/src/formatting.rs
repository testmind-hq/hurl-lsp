pub fn format_document(text: &str) -> Option<String> {
    let hurl_file = hurl_core::parser::parse_hurl_file(text).ok()?;
    Some(hurlfmt::format::format_text(&hurl_file, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_official_formatter() {
        let formatted = format_document("GET https://example.com   \nHTTP 200   ")
            .expect("expected formatter to return output");
        assert!(!formatted.contains('\u{1b}'));
    }

    #[test]
    fn returns_none_when_parse_fails() {
        let formatted =
            format_document("GET https://example.com\nHTTP 200\n[Asserts]\njsonpath \"$.id == 1\n");
        assert_eq!(formatted, None);
    }
}
