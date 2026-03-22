use tower_lsp::lsp_types::{Hover, HoverContents, MarkedString, Position};

const SECTION_DOCS: &[(&str, &str)] = &[
    ("[Headers]", "HTTP request headers."),
    ("[Asserts]", "Assertions evaluated against the response."),
    (
        "[Captures]",
        "Values extracted from the response for later reuse.",
    ),
    ("[Options]", "Request execution options."),
];

const ASSERT_DOCS: &[(&str, &str)] = &[
    (
        "jsonpath",
        "Evaluate a JSONPath expression against the response body.",
    ),
    (
        "xpath",
        "Evaluate an XPath expression against the response body.",
    ),
    ("regex", "Assert with a regular expression."),
    ("status", "Assert against the HTTP status code."),
    (
        "duration",
        "Assert against total request duration in milliseconds.",
    ),
];

const METHOD_DOCS: &[(&str, &str)] = &[
    ("GET", "Retrieve a representation of a resource."),
    ("POST", "Submit data to create or trigger a resource."),
    ("PUT", "Replace a resource with the provided payload."),
    ("PATCH", "Partially update a resource."),
    ("DELETE", "Delete a resource."),
];

pub fn hover(text: &str, position: Position) -> Option<Hover> {
    let line = text.lines().nth(position.line as usize)?;
    let token = token_at(line, position.character as usize)?;
    let docs = SECTION_DOCS
        .iter()
        .chain(ASSERT_DOCS.iter())
        .chain(METHOD_DOCS.iter())
        .find(|(label, _)| *label == token)?;

    Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(format!(
            "**{}**\n\n{}",
            docs.0, docs.1
        ))),
        range: None,
    })
}

fn token_at(line: &str, character: usize) -> Option<&str> {
    if line.trim_start().starts_with('[') {
        return Some(line.trim());
    }

    let idx = character.min(line.len());
    let bytes = line.as_bytes();
    let mut start = idx;
    while start > 0 && !bytes[start - 1].is_ascii_whitespace() {
        start -= 1;
    }
    let mut end = idx;
    while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
        end += 1;
    }
    if start == end {
        None
    } else {
        line.get(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_hover_for_method() {
        let value = hover("GET https://example.com", Position::new(0, 1));
        assert!(value.is_some());
    }
}
