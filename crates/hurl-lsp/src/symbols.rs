use crate::diagnostics::ParsedDocument;
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

#[allow(deprecated)]
pub fn document_symbols(parsed: &ParsedDocument) -> Vec<DocumentSymbol> {
    parsed
        .entries
        .iter()
        .map(|entry| {
            let name = format!("{} {}", entry.method, entry.path);
            let range = Range::new(
                Position::new(entry.line, 0),
                Position::new(entry.line, name.len() as u32),
            );
            DocumentSymbol {
                name,
                detail: None,
                kind: SymbolKind::OBJECT,
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
                children: None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Entry, ParsedDocument};

    #[test]
    fn creates_request_symbols() {
        let parsed = ParsedDocument {
            entries: vec![Entry {
                method: "GET".into(),
                path: "/users".into(),
                line: 3,
            }],
        };

        let symbols = document_symbols(&parsed);
        assert_eq!(symbols[0].name, "GET /users");
        assert_eq!(symbols[0].range.start.line, 3);
    }
}
