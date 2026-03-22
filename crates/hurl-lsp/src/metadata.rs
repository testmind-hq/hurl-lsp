#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseKind {
    Single,
    Chain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Priority {
    P0,
    P1,
    P2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepType {
    Setup,
    Test,
    Teardown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryMeta {
    pub case_id: Option<String>,
    pub case_kind: Option<CaseKind>,
    pub priority: Option<Priority>,
    pub step_id: Option<String>,
    pub step_type: Option<StepType>,
    pub title: Option<String>,
    pub technique: Option<String>,
    pub depends_on: Vec<String>,
    pub line: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HurlFileMeta {
    pub entries: Vec<EntryMeta>,
}

pub struct HurlMetaParser;

impl HurlMetaParser {
    pub fn parse(source: &str) -> HurlFileMeta {
        let mut entries = Vec::new();
        let mut current = EntryMeta::default();

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            if let Some((key, value)) = parse_meta_comment(trimmed) {
                match key.as_str() {
                    "case_id" => current.case_id = Some(value),
                    "case_kind" => current.case_kind = parse_case_kind(&value),
                    "priority" => current.priority = parse_priority(&value),
                    "step_id" => current.step_id = Some(value),
                    "step_type" => current.step_type = parse_step_type(&value),
                    "title" => current.title = Some(value),
                    "technique" => current.technique = Some(value),
                    "depends_on" => {
                        current.depends_on = value
                            .split(',')
                            .map(str::trim)
                            .filter(|part| !part.is_empty())
                            .map(ToString::to_string)
                            .collect()
                    }
                    _ => {}
                }
                continue;
            }

            if is_http_method_line(trimmed) {
                current.line = line_num as u32;
                entries.push(current.clone());
                current = EntryMeta::default();
            }
        }

        HurlFileMeta { entries }
    }
}

fn parse_meta_comment(line: &str) -> Option<(String, String)> {
    if !line.starts_with('#') {
        return None;
    }
    let rest = line[1..].trim();
    let (key, value) = rest.split_once('=')?;
    let key = key.trim();
    if !matches!(
        key,
        "case_id"
            | "case_kind"
            | "priority"
            | "step_id"
            | "step_type"
            | "title"
            | "technique"
            | "depends_on"
    ) {
        return None;
    }
    Some((key.to_string(), value.trim().to_string()))
}

fn parse_case_kind(value: &str) -> Option<CaseKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "single" => Some(CaseKind::Single),
        "chain" => Some(CaseKind::Chain),
        _ => None,
    }
}

fn parse_priority(value: &str) -> Option<Priority> {
    match value.trim() {
        "P0" => Some(Priority::P0),
        "P1" => Some(Priority::P1),
        "P2" => Some(Priority::P2),
        _ => None,
    }
}

fn parse_step_type(value: &str) -> Option<StepType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "setup" => Some(StepType::Setup),
        "test" => Some(StepType::Test),
        "teardown" => Some(StepType::Teardown),
        _ => None,
    }
}

fn is_http_method_line(line: &str) -> bool {
    let method = line.split_whitespace().next().unwrap_or_default();
    matches!(
        method,
        "GET" | "HEAD" | "POST" | "PUT" | "DELETE" | "CONNECT" | "OPTIONS" | "TRACE" | "PATCH"
    ) && line.split_whitespace().count() >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entry_metadata_by_request_order() {
        let source = r#"
# case_id=TC-CHAIN-001
# case_kind=chain
# priority=P0
# step_id=setup-step
# step_type=setup
# title=Create user
POST https://example.com/users
HTTP 201

# case_id=TC-0042
# case_kind=single
# priority=P1
# title=Invalid email
POST https://example.com/users
HTTP 422
"#;

        let file_meta = HurlMetaParser::parse(source);
        assert_eq!(file_meta.entries.len(), 2);
        assert_eq!(
            file_meta.entries[0].case_id.as_deref(),
            Some("TC-CHAIN-001")
        );
        assert_eq!(file_meta.entries[0].case_kind, Some(CaseKind::Chain));
        assert_eq!(file_meta.entries[0].step_type, Some(StepType::Setup));
        assert_eq!(file_meta.entries[1].priority, Some(Priority::P1));
        assert_eq!(file_meta.entries[1].title.as_deref(), Some("Invalid email"));
    }
}
