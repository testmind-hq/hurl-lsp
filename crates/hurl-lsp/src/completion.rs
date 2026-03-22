use crate::syntax::{
    is_identifier, method_from_line, section_label, section_name_from_line,
    visible_variables_before_line, SECTION_NAMES,
};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
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
    let line_idx = position.line as usize;
    let line = text.lines().nth(position.line as usize).unwrap_or_default();
    let prefix = &line[..(position.character as usize).min(line.len())];
    let trimmed = prefix.trim_start();

    if let Some(var_prefix) = variable_prefix(prefix) {
        let vars = known_variables(text, line_idx);
        return vars
            .into_iter()
            .filter(|name| name.starts_with(var_prefix))
            .map(|name| CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("Captured variable".into()),
                insert_text: Some([name.as_str(), "}}"].concat()),
                ..Default::default()
            })
            .collect();
    }

    if trimmed.starts_with('[') {
        return SECTION_NAMES
            .iter()
            .map(|section| keyword_item(&section_label(section)))
            .collect();
    }

    if in_asserts_block(text, line_idx) {
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
    let mut current_section = None;

    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if method_from_line(trimmed).is_some() {
            current_section = None;
        } else if let Some(name) = section_name_from_line(trimmed) {
            current_section = Some(name);
        }
        if idx == line_idx {
            break;
        }
    }

    current_section == Some("Asserts")
}

fn variable_prefix(prefix: &str) -> Option<&str> {
    let idx = prefix.rfind("{{")?;
    let content = &prefix[(idx + 2)..];
    if content.contains("}}") {
        return None;
    }
    Some(content)
}

fn known_variables(text: &str, line_idx: usize) -> Vec<String> {
    visible_variables_before_line(text, line_idx)
        .into_iter()
        .filter(|name| is_identifier(name))
        .collect()
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

    #[test]
    fn returns_variable_completions_from_captures() {
        let text = "[Captures]\nuser_id: jsonpath \"$.id\"\n\nGET /users/{{u";
        let items = completions(text, Position::new(3, 13));
        assert!(items.iter().any(|item| item.label == "user_id"));
    }

    #[test]
    fn resets_assert_context_on_next_request() {
        let text = "GET /a\nHTTP 200\n[Asserts]\njsonpath \"$.id\" == 1\n\nGET /b\n";
        let items = completions(text, Position::new(5, 2));
        assert!(!items.iter().any(|item| item.label == "jsonpath"));
    }

    #[test]
    fn supports_short_section_completion() {
        let items = completions("[Q", Position::new(0, 2));
        assert!(items.iter().any(|item| item.label == "[Query]"));
        assert!(items.iter().any(|item| item.label == "[Form]"));
    }

    #[test]
    fn does_not_suggest_future_capture_variable() {
        let text = "GET /users/{{u}}\nHTTP 200\n\nGET /a\nHTTP 200\n[Captures]\nuser_id: jsonpath \"$.id\"\n";
        let items = completions(text, Position::new(0, 14));
        assert!(!items.iter().any(|item| item.label == "user_id"));
    }
}
