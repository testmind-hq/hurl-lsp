use crate::syntax::{capture_definitions_before_line, variable_placeholders};
use tower_lsp::lsp_types::{
    GotoDefinitionResponse, Location, Position, Range, TextDocumentPositionParams, Url,
};

pub fn definition(
    uri: &Url,
    text: &str,
    params: &TextDocumentPositionParams,
) -> Option<GotoDefinitionResponse> {
    let variable = variable_at_position(text, params.position)?;
    let (line, start, end) =
        capture_definition_span(text, variable, params.position.line as usize)?;

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

fn capture_definition_span(
    text: &str,
    variable: &str,
    target_line: usize,
) -> Option<(usize, usize, usize)> {
    capture_definitions_before_line(text, target_line)
        .into_iter()
        .rev()
        .find(|(_, _, _, name)| name == variable)
        .map(|(line, start, end, _)| (line, start, end))
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

    #[test]
    fn resolves_nearest_previous_definition() {
        let text = "GET /u1\nHTTP 200\n[Captures]\nuser_id: jsonpath \"$.id\"\n\nGET /u2\nHTTP 200\n[Captures]\nuser_id: jsonpath \"$.other\"\n\nGET /u3/{{user_id}}\nHTTP 200\n";
        let uri = Url::parse("file:///tmp/test.hurl").expect("valid uri");
        let params = TextDocumentPositionParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(10, 12),
        };

        let result = definition(&uri, text, &params).expect("definition should resolve");
        match result {
            GotoDefinitionResponse::Scalar(location) => {
                assert_eq!(location.range.start.line, 8);
            }
            _ => panic!("unexpected definition response"),
        }
    }
}
