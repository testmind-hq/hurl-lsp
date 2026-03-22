use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const SECTIONS: &[&str] = &[
    "[Asserts]",
    "[Captures]",
    "[Cookies]",
    "[FormParams]",
    "[Headers]",
    "[Options]",
    "[QueryStringParameters]",
];
const ASSERTS: &[(&str, &str)] = &[
    ("jsonpath", "jsonpath \"$.field\" == value"),
    ("xpath", "xpath \"//node\" exists"),
    ("regex", "regex \"pattern\""),
    ("header", "header \"content-type\" == \"application/json\""),
    ("status", "status == 200"),
    ("duration", "duration < 1000"),
];
const CONTENT_TYPES: &[&str] = &[
    "application/json",
    "application/xml",
    "text/plain",
    "application/x-www-form-urlencoded",
];

pub fn completions(text: &str, position: Position) -> Vec<CompletionItem> {
    let line = text.lines().nth(position.line as usize).unwrap_or_default();
    let prefix = &line[..(position.character as usize).min(line.len())];
    let trimmed = prefix.trim_start();

    if trimmed.starts_with('[') {
        return SECTIONS
            .iter()
            .map(|section| keyword_item(section))
            .collect();
    }

    if in_asserts_block(text, position.line as usize) {
        return ASSERTS
            .iter()
            .map(|(label, detail)| CompletionItem {
                label: (*label).into(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some((*detail).into()),
                insert_text: Some(format!("{label} ")),
                ..Default::default()
            })
            .collect();
    }

    if prefix.contains("Content-Type:") {
        return CONTENT_TYPES
            .iter()
            .map(|item| keyword_item(item))
            .collect();
    }

    METHODS.iter().map(|method| keyword_item(method)).collect()
}

fn keyword_item(value: &str) -> CompletionItem {
    CompletionItem {
        label: value.into(),
        kind: Some(CompletionItemKind::KEYWORD),
        insert_text: Some(value.into()),
        ..Default::default()
    }
}

fn in_asserts_block(text: &str, line_idx: usize) -> bool {
    let mut current_section = "";

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed;
        }
        if idx == line_idx {
            break;
        }
    }

    current_section == "[Asserts]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_method_completions_by_default() {
        let items = completions("", Position::new(0, 0));
        assert!(items.iter().any(|item| item.label == "GET"));
    }

    #[test]
    fn returns_assert_completions_inside_asserts_block() {
        let items = completions("[Asserts]\njs", Position::new(1, 2));
        assert!(items.iter().any(|item| item.label == "jsonpath"));
    }
}
