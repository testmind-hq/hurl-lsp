use crate::{
    syntax::{variable_placeholders, visible_variables_before_line, BUILTIN_VARIABLES},
    variables::ResolvedVariable,
};
use std::collections::BTreeMap;
use tower_lsp::lsp_types::{
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintTooltip, MarkupContent, MarkupKind,
    Position, Range,
};

pub fn variable_inlay_hints(
    text: &str,
    range: Range,
    external: &BTreeMap<String, ResolvedVariable>,
    max_length: usize,
) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let max_length = max_length.clamp(8, 500);
    for (line_index, line) in text.lines().enumerate() {
        let line_no = line_index as u32;
        if line_no < range.start.line || line_no > range.end.line {
            continue;
        }
        let visible_runtime = visible_variables_before_line(text, line_index);
        for (_, end, name) in variable_placeholders(line) {
            let (label, tooltip) = if let Some(variable) = external.get(name) {
                let value = if variable.sensitive {
                    "••••••".to_string()
                } else {
                    truncate(&variable.value, max_length)
                };
                let source = variable
                    .uri
                    .to_file_path()
                    .ok()
                    .and_then(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| variable.uri.to_string());
                (
                    format!("= {value}"),
                    format!("**{name}**\n\nSource: `{source}:{}`", variable.line + 1),
                )
            } else if visible_runtime.contains(name) || BUILTIN_VARIABLES.contains(&name) {
                (
                    "= runtime value".to_string(),
                    format!("**{name}**\n\nAvailable only at runtime."),
                )
            } else {
                continue;
            };
            hints.push(InlayHint {
                position: Position::new(line_no, end as u32),
                label: InlayHintLabel::String(label),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(InlayHintTooltip::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: tooltip,
                })),
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }
    }
    hints
}

fn truncate(value: &str, max_length: usize) -> String {
    if value.chars().count() <= max_length {
        return value.to_string();
    }
    let keep = max_length.saturating_sub(1);
    format!("{}…", value.chars().take(keep).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    fn variable(name: &str, value: &str, sensitive: bool) -> ResolvedVariable {
        ResolvedVariable {
            name: name.into(),
            value: value.into(),
            uri: Url::parse("file:///tmp/vars.env").expect("uri"),
            line: 1,
            start: 0,
            end: name.len() as u32,
            sensitive,
        }
    }

    fn labels(hints: &[InlayHint]) -> Vec<String> {
        hints
            .iter()
            .map(|hint| match &hint.label {
                InlayHintLabel::String(value) => value.clone(),
                _ => String::new(),
            })
            .collect()
    }

    #[test]
    fn previews_external_sensitive_runtime_and_skips_missing() {
        let text = "GET /seed\nHTTP 200\n[Captures]\ncaptured: jsonpath \"$.id\"\n\nGET {{base_url}}/{{token}}/{{captured}}/{{missing}}/{{newUuid}}\nHTTP 200\n";
        let external = BTreeMap::from([
            (
                "base_url".into(),
                variable("base_url", "https://example.com", false),
            ),
            ("token".into(), variable("token", "real-secret", true)),
        ]);
        let hints = variable_inlay_hints(
            text,
            Range::new(Position::new(5, 0), Position::new(5, 99)),
            &external,
            12,
        );
        assert_eq!(
            labels(&hints),
            vec![
                "= https://exa…",
                "= ••••••",
                "= runtime value",
                "= runtime value"
            ]
        );
    }
}
