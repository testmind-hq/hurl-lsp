use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tower_lsp::lsp_types::Url;

const VARIABLE_FILES: &[&str] = &[".hurl-vars", "vars.env", "hurl.env", ".env"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableDef {
    pub name: String,
    pub value: String,
    pub uri: Url,
    pub line: u32,
    pub start: u32,
    pub end: u32,
}

pub fn load_workspace_variables(document_uri: &Url) -> Vec<VariableDef> {
    let Some(path) = file_path_from_uri(document_uri) else {
        return Vec::new();
    };
    let Some(base_dir) = path.parent() else {
        return Vec::new();
    };

    let mut dirs = Vec::new();
    let mut current = Some(base_dir.to_path_buf());
    while let Some(dir) = current {
        dirs.push(dir.clone());
        current = dir.parent().map(Path::to_path_buf);
    }
    dirs.reverse();

    let mut vars = BTreeMap::<String, VariableDef>::new();
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

    vars.into_values().collect()
}

fn file_path_from_uri(uri: &Url) -> Option<PathBuf> {
    if uri.scheme() != "file" {
        return None;
    }
    uri.to_file_path().ok()
}

fn parse_variable_file(path: &Path) -> Vec<VariableDef> {
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
        .map(|(idx, name, value, start, end)| VariableDef {
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

        let vars = load_workspace_variables(&uri);
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

        let vars = load_workspace_variables(&uri);
        let host = vars.iter().find(|var| var.name == "host").expect("host");
        assert_eq!(host.value, "local.example.com");

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
