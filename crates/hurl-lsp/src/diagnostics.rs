use hurl_core::error::DisplaySourceError;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub method: String,
    pub path: String,
    pub line: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedDocument {
    pub entries: Vec<Entry>,
}

const HTTP_METHODS: &[&str] = &[
    "GET", "HEAD", "POST", "PUT", "DELETE", "CONNECT", "OPTIONS", "TRACE", "PATCH",
];
const SECTIONS: &[&str] = &[
    "[Asserts]",
    "[BasicAuth]",
    "[Captures]",
    "[Cookies]",
    "[FormParams]",
    "[Headers]",
    "[MultipartFormData]",
    "[Options]",
    "[QueryStringParams]",
    "[QueryStringParameters]",
];

pub fn parse_document(text: &str) -> ParsedDocument {
    let mut entries = Vec::new();

    for (line_idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with('[')
            || line.starts_with("HTTP ")
        {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(first) = parts.next() else {
            continue;
        };
        let Some(second) = parts.next() else {
            continue;
        };

        if HTTP_METHODS.contains(&first) {
            entries.push(Entry {
                method: first.to_string(),
                path: second.to_string(),
                line: line_idx as u32,
            });
        }
    }

    ParsedDocument { entries }
}

pub fn collect_diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if let Err(error) = hurl_core::parser::parse_hurl_file(text) {
        let line = error.pos.line.saturating_sub(1) as u32;
        let character = error.pos.column.saturating_sub(1) as u32;
        diagnostics.push(Diagnostic {
            range: Range::new(
                Position::new(line, character),
                Position::new(line, character.saturating_add(1)),
            ),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("hurl_core".into()),
            message: error.description(),
            ..Default::default()
        });
    }

    for (line_idx, raw_line) in text.lines().enumerate() {
        let trimmed = raw_line.trim();

        if trimmed.starts_with('[') && !SECTIONS.contains(&trimmed) {
            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(line_idx as u32, 0),
                    Position::new(line_idx as u32, raw_line.len() as u32),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("hurl-lsp".into()),
                message: format!("Unknown section `{trimmed}`"),
                ..Default::default()
            });
        }

        if is_probable_method_line(trimmed) {
            let method = trimmed.split_whitespace().next().unwrap_or_default();
            if !HTTP_METHODS.contains(&method) {
                diagnostics.push(Diagnostic {
                    range: Range::new(
                        Position::new(line_idx as u32, 0),
                        Position::new(line_idx as u32, method.len() as u32),
                    ),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("hurl-lsp".into()),
                    message: format!("Unknown HTTP method `{method}`"),
                    ..Default::default()
                });
            }
        }
    }

    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.range.start.line,
            diagnostic.range.start.character,
            diagnostic.message.clone(),
        )
    });
    diagnostics.dedup_by(|left, right| {
        left.range == right.range
            && left.message == right.message
            && left.severity == right.severity
    });

    diagnostics
}

fn is_probable_method_line(line: &str) -> bool {
    if line.is_empty()
        || line.starts_with('#')
        || line.starts_with('[')
        || line.starts_with('{')
        || line.starts_with("HTTP ")
    {
        return false;
    }

    let token = line.split_whitespace().next().unwrap_or_default();
    token.chars().all(|ch| ch.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let parsed = parse_document("GET https://example.com\nHTTP 200\n\nPOST /users\nHTTP 201\n");
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].method, "GET");
        assert_eq!(parsed.entries[1].path, "/users");
    }

    #[test]
    fn flags_unknown_section() {
        let diagnostics = collect_diagnostics("[Headerz]\nfoo: bar\n");
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Unknown section")));
    }
}
