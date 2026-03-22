use tower_lsp::lsp_types::{
    GotoDefinitionResponse, Location, Position, Range, TextDocumentPositionParams, Url,
};

pub fn definition(
    uri: &Url,
    text: &str,
    params: &TextDocumentPositionParams,
) -> Option<GotoDefinitionResponse> {
    let variable = variable_at_position(text, params.position)?;
    let (line, start, end) = capture_definition_span(text, variable)?;

    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: Range::new(
            Position::new(line as u32, start as u32),
            Position::new(line as u32, end as u32),
        ),
    }))
}

fn variable_at_position(text: &str, position: Position) -> Option<&str> {
    let line = text.lines().nth(position.line as usize)?;
    let ch = position.character as usize;
    for (start, end, name) in variable_placeholders(line) {
        if ch >= start && ch <= end {
            return Some(name);
        }
    }
    None
}

fn capture_definition_span(text: &str, variable: &str) -> Option<(usize, usize, usize)> {
    let mut in_captures = false;

    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_captures = trimmed == "[Captures]";
            continue;
        }
        if !in_captures || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name != variable {
            continue;
        }

        let leading_ws = line.chars().take_while(|c| c.is_whitespace()).count();
        let start = leading_ws;
        let end = start + name.len();
        return Some((line_idx, start, end));
    }

    None
}

fn variable_placeholders(line: &str) -> Vec<(usize, usize, &str)> {
    let mut result = Vec::new();
    let mut offset = 0;

    while let Some(start) = line[offset..].find("{{") {
        let abs_start = offset + start;
        let content_start = abs_start + 2;
        let Some(end_rel) = line[content_start..].find("}}") else {
            break;
        };
        let content_end = content_start + end_rel;
        let variable = line[content_start..content_end].trim();
        if !variable.is_empty() {
            result.push((abs_start, content_end + 2, variable));
        }
        offset = content_end + 2;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_capture_variable_definition() {
        let text = "GET /users\nHTTP 200\n[Captures]\nuser_id: jsonpath \"$.id\"\n\nGET /users/{{user_id}}\nHTTP 200\n";
        let uri = Url::parse("file:///tmp/test.hurl").expect("valid uri");
        let position = Position::new(5, 15);
        let params = TextDocumentPositionParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position,
        };

        let result = definition(&uri, text, &params).expect("definition should resolve");
        match result {
            GotoDefinitionResponse::Scalar(location) => {
                assert_eq!(location.range.start.line, 3);
                assert_eq!(location.range.start.character, 0);
            }
            _ => panic!("unexpected definition response"),
        }
    }
}
