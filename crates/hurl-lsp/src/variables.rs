use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::{Builder, NamedTempFile};
use tower_lsp::lsp_types::Url;

const VARIABLE_FILES: &[&str] = &[".hurl-vars", "vars.env", "hurl.env", ".env"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedVariable {
    pub name: String,
    pub value: String,
    pub uri: Url,
    pub line: u32,
    pub start: u32,
    pub end: u32,
    pub sensitive: bool,
}

pub type VariableDef = ResolvedVariable;

pub fn load_workspace_variables_with_roots(
    document_uri: &Url,
    workspace_roots: &[PathBuf],
) -> Vec<ResolvedVariable> {
    resolve_workspace_variables(document_uri, workspace_roots)
        .into_values()
        .collect()
}

pub fn resolve_workspace_variables(
    document_uri: &Url,
    workspace_roots: &[PathBuf],
) -> BTreeMap<String, ResolvedVariable> {
    let Some(base_dir) = base_dir_from_uri(document_uri) else {
        return BTreeMap::new();
    };
    let mut dirs = bounded_ancestor_dirs(base_dir, workspace_roots);
    dirs.reverse();

    let mut vars = BTreeMap::<String, ResolvedVariable>::new();
    for dir in dirs {
        for file_name in VARIABLE_FILES {
            let file_path = dir.join(file_name);
            if !file_path.exists() || !file_path.is_file() {
                continue;
            }
            for var in parse_variable_file(&file_path) {
                vars.insert(var.name.clone(), var);
            }
        }
    }

    vars
}

pub fn write_merged_variables_file(
    vars: &BTreeMap<String, ResolvedVariable>,
    directory: Option<&Path>,
) -> std::io::Result<NamedTempFile> {
    let mut builder = Builder::new();
    builder.prefix("hurl-lsp-vars-").suffix(".env");
    let mut file = match directory {
        Some(dir) => builder
            .tempfile_in(dir)
            .or_else(|_| Builder::new().suffix(".env").tempfile())?,
        None => builder.tempfile()?,
    };
    for (name, variable) in vars {
        writeln!(file, "{name}={}", variable.value)?;
    }
    file.flush()?;
    Ok(file)
}

pub fn is_sensitive_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "passwd",
        "authorization",
        "api_key",
        "apikey",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

fn file_path_from_uri(uri: &Url) -> Option<PathBuf> {
    if uri.scheme() != "file" {
        return None;
    }
    uri.to_file_path().ok()
}

fn base_dir_from_uri(uri: &Url) -> Option<PathBuf> {
    let path = file_path_from_uri(uri)?;
    path.parent().map(Path::to_path_buf)
}

fn bounded_ancestor_dirs(base_dir: PathBuf, workspace_roots: &[PathBuf]) -> Vec<PathBuf> {
    let normalized_roots: Vec<PathBuf> = workspace_roots
        .iter()
        .map(|root| root.canonicalize().unwrap_or_else(|_| root.clone()))
        .collect();
    let normalized_base = base_dir.canonicalize().unwrap_or_else(|_| base_dir.clone());

    let selected_root = normalized_roots
        .iter()
        .filter(|root| normalized_base.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned();

    let mut dirs = Vec::new();
    let mut current = Some(normalized_base);
    while let Some(dir) = current {
        if let Some(root) = &selected_root {
            if !dir.starts_with(root) {
                break;
            }
        }
        dirs.push(dir.clone());
        if let Some(root) = &selected_root {
            if dir == *root {
                break;
            }
        } else {
            break;
        }
        current = dir.parent().map(Path::to_path_buf);
    }
    dirs
}

fn parse_variable_file(path: &Path) -> Vec<ResolvedVariable> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(uri) = Url::from_file_path(path) else {
        return Vec::new();
    };

    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            parse_variable_line(line)
                .map(|(name, value, start, end)| (idx, name, value, start, end))
        })
        .map(|(idx, name, value, start, end)| ResolvedVariable {
            sensitive: is_sensitive_name(&name),
            name,
            value,
            uri: uri.clone(),
            line: idx as u32,
            start: start as u32,
            end: end as u32,
        })
        .collect()
}

fn parse_variable_line(line: &str) -> Option<(String, String, usize, usize)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (left, right) = line.split_once('=')?;
    let name = left.trim();
    if !is_identifier(name) {
        return None;
    }
    let value = right.trim().to_string();
    let start = line.find(name)?;
    Some((name.to_string(), value, start, start + name.len()))
}

fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loads_variables_from_detected_files() {
        let base = tmp_dir("hurl-lsp-vars");
        fs::create_dir_all(&base).expect("mkdir");
        fs::write(base.join(".env"), "host=example.com\n# note\nport=443\n").expect("write env");
        let nested = base.join("api");
        fs::create_dir_all(&nested).expect("mkdir nested");
        let uri = Url::from_file_path(nested.join("test.hurl")).expect("uri");

        let vars = load_workspace_variables_with_roots(&uri, std::slice::from_ref(&base));
        assert!(vars
            .iter()
            .any(|var| var.name == "host" && var.value == "example.com"));
        assert!(vars
            .iter()
            .any(|var| var.name == "port" && var.value == "443"));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn overrides_with_nearest_file() {
        let base = tmp_dir("hurl-lsp-vars-override");
        let nested = base.join("project");
        fs::create_dir_all(&nested).expect("mkdir");
        fs::write(base.join(".env"), "host=global.example.com\n").expect("write root");
        fs::write(nested.join(".env"), "host=local.example.com\n").expect("write nested");
        let uri = Url::from_file_path(nested.join("case.hurl")).expect("uri");

        let vars = load_workspace_variables_with_roots(&uri, std::slice::from_ref(&base));
        let host = vars.iter().find(|var| var.name == "host").expect("host");
        assert_eq!(host.value, "local.example.com");

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn does_not_read_variables_outside_workspace_root() {
        let base = tmp_dir("hurl-lsp-vars-bounded");
        let workspace = base.join("workspace");
        let nested = workspace.join("api");
        fs::create_dir_all(&nested).expect("mkdir nested");
        fs::write(base.join(".env"), "outer_only=1\n").expect("write outer");
        fs::write(workspace.join(".env"), "inner_only=1\n").expect("write inner");
        let uri = Url::from_file_path(nested.join("test.hurl")).expect("uri");

        let vars = load_workspace_variables_with_roots(&uri, std::slice::from_ref(&workspace));
        assert!(vars.iter().any(|var| var.name == "inner_only"));
        assert!(!vars.iter().any(|var| var.name == "outer_only"));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn keeps_full_value_when_contains_equal_sign() {
        let value = parse_variable_line("token=abc=def").expect("parsed").1;
        assert_eq!(value, "abc=def");
    }

    #[test]
    fn resolves_precedence_source_and_sensitivity() {
        let base = tmp_dir("hurl-lsp-vars-resolved");
        let nested = base.join("project");
        fs::create_dir_all(&nested).expect("mkdir");
        fs::write(base.join(".env"), "host=root.example.com\n").expect("root env");
        fs::write(
            nested.join("vars.env"),
            "host=nested-vars.example.com\napi_token=secret\n",
        )
        .expect("vars");
        fs::write(
            nested.join(".env"),
            "host=nested-dotenv.example.com\naccount_email=user@example.com\n",
        )
        .expect("env");
        let uri = Url::from_file_path(nested.join("case.hurl")).expect("uri");

        let vars = resolve_workspace_variables(&uri, std::slice::from_ref(&base));
        let host = vars.get("host").expect("host");
        assert_eq!(host.value, "nested-dotenv.example.com");
        assert!(host.uri.path().ends_with("project/.env"));
        assert_eq!(host.line, 0);
        assert!(vars.get("api_token").expect("token").sensitive);
        assert!(!vars.get("account_email").expect("email").sensitive);
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn writes_deterministic_merged_file() {
        let base = tmp_dir("hurl-lsp-vars-merged");
        fs::create_dir_all(&base).expect("mkdir");
        let uri = Url::from_file_path(base.join("vars.env")).expect("uri");
        let vars = BTreeMap::from([
            (
                "token".into(),
                ResolvedVariable {
                    name: "token".into(),
                    value: "abc=def".into(),
                    uri: uri.clone(),
                    line: 1,
                    start: 0,
                    end: 5,
                    sensitive: true,
                },
            ),
            (
                "account_email".into(),
                ResolvedVariable {
                    name: "account_email".into(),
                    value: "user@example.com".into(),
                    uri,
                    line: 0,
                    start: 0,
                    end: 13,
                    sensitive: false,
                },
            ),
        ]);
        let temp = write_merged_variables_file(&vars, Some(&base)).expect("merged file");
        assert_eq!(
            fs::read_to_string(temp.path()).expect("read"),
            "account_email=user@example.com\ntoken=abc=def\n"
        );
        assert_eq!(temp.path().parent(), Some(base.as_path()));
        drop(temp);
        let _ = fs::remove_dir_all(base);
    }

    fn tmp_dir(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{now}"))
    }
}
