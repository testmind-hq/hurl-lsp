pub fn format_document(text: &str) -> Option<String> {
    let hurl_file = hurl_core::parser::parse_hurl_file(text).ok()?;
    Some(hurlfmt::linter::lint_hurl_file(&hurl_file))
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

    #[test]
    fn normalizes_form_whitespace() {
        let source = "POST https://example.org/login\n[Form]\n user: toto\npassword:1234\ntoken: {{csrf_token}}\nHTTP 302\n";
        let formatted = format_document(source).expect("expected formatter to return output");
        assert!(formatted.contains("\nuser: toto\n"));
        assert!(formatted.contains("\npassword: 1234\n"));
        assert_ne!(formatted, source);
    }
}
