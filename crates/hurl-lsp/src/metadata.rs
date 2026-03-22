use crate::syntax::{method_from_line, section_name_from_line, variable_placeholders};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryDependency {
    pub from_line: u32,
    pub to_line: u32,
    pub from_step: String,
    pub to_step: String,
    pub variables: Vec<String>,
    pub inferred: bool,
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

pub fn infer_entry_dependencies(source: &str, file_meta: &HurlFileMeta) -> Vec<EntryDependency> {
    let mut out = Vec::new();
    if file_meta.entries.is_empty() {
        return out;
    }
    let entry_lines: Vec<u32> = file_meta.entries.iter().map(|entry| entry.line).collect();
    let lines: Vec<&str> = source.lines().collect();
    let mut edges: BTreeMap<(usize, usize), BTreeSet<String>> = BTreeMap::new();

    let mut step_to_index = BTreeMap::<String, usize>::new();
    for (idx, entry) in file_meta.entries.iter().enumerate() {
        if let Some(step_id) = &entry.step_id {
            step_to_index.insert(step_id.clone(), idx);
        }
    }

    for (to_idx, entry) in file_meta.entries.iter().enumerate() {
        for dependency in &entry.depends_on {
            if let Some(from_idx) = step_to_index.get(dependency) {
                edges.entry((*from_idx, to_idx)).or_default();
            }
        }
    }

    let capture_vars = capture_variables_by_entry(&lines, &entry_lines);
    let used_vars = used_variables_by_entry(&lines, &entry_lines);

    for (to_idx, vars) in used_vars.iter().enumerate() {
        for var in vars {
            let producer = (0..to_idx)
                .rev()
                .find(|from_idx| capture_vars[*from_idx].contains(var));
            if let Some(from_idx) = producer {
                edges
                    .entry((from_idx, to_idx))
                    .or_default()
                    .insert(var.clone());
            }
        }
    }

    for ((from_idx, to_idx), vars) in edges {
        let from = &file_meta.entries[from_idx];
        let to = &file_meta.entries[to_idx];
        let explicit = to
            .depends_on
            .iter()
            .any(|dependency| from.step_id.as_deref() == Some(dependency.as_str()));
        out.push(EntryDependency {
            from_line: from.line,
            to_line: to.line,
            from_step: step_label(from),
            to_step: step_label(to),
            variables: vars.into_iter().collect(),
            inferred: !explicit,
        });
    }

    out
}

fn step_label(entry: &EntryMeta) -> String {
    entry
        .step_id
        .clone()
        .unwrap_or_else(|| format!("line-{}", entry.line + 1))
}

fn capture_variables_by_entry(lines: &[&str], entry_lines: &[u32]) -> Vec<BTreeSet<String>> {
    let mut out = Vec::new();
    for idx in 0..entry_lines.len() {
        let start = entry_lines[idx] as usize;
        let end = entry_lines
            .get(idx + 1)
            .copied()
            .unwrap_or(lines.len() as u32) as usize;
        let mut captures = BTreeSet::new();
        let mut in_captures = false;
        for raw in &lines[start..end] {
            let trimmed = raw.trim();
            if method_from_line(trimmed).is_some() {
                in_captures = false;
                continue;
            }
            if let Some(section) = section_name_from_line(trimmed) {
                in_captures = section == "Captures";
                continue;
            }
            if !in_captures || trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((name, _)) = trimmed.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if !name.is_empty() {
                captures.insert(name.to_string());
            }
        }
        out.push(captures);
    }
    out
}

fn used_variables_by_entry(lines: &[&str], entry_lines: &[u32]) -> Vec<BTreeSet<String>> {
    let mut out = Vec::new();
    for idx in 0..entry_lines.len() {
        let start = entry_lines[idx] as usize;
        let end = entry_lines
            .get(idx + 1)
            .copied()
            .unwrap_or(lines.len() as u32) as usize;
        let mut vars = BTreeSet::new();
        for raw in &lines[start..end] {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            for (_, _, variable) in variable_placeholders(raw) {
                vars.insert(variable.to_string());
            }
        }
        out.push(vars);
    }
    out
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

    #[test]
    fn infers_dependency_from_capture_usage_when_no_depends_on_meta() {
        let source = r#"
# step_id=step-setup-user
POST /users
HTTP 201
[Captures]
user_id: jsonpath "$.id"

# step_id=step-test-get
GET /users/{{user_id}}
HTTP 200
"#;
        let file_meta = HurlMetaParser::parse(source);
        let deps = infer_entry_dependencies(source, &file_meta);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].from_step, "step-setup-user");
        assert_eq!(deps[0].to_step, "step-test-get");
        assert_eq!(deps[0].variables, vec!["user_id".to_string()]);
        assert!(deps[0].inferred);
    }
}
