use crate::variables::{resolve_variable_files, resolve_workspace_variables, ResolvedVariable};
use std::{collections::BTreeMap, path::PathBuf};
use tower_lsp::lsp_types::Url;

pub const AUTO_PROFILE: &str = "Auto";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileConfig {
    pub name: String,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ResolvedProfile {
    pub name: String,
    pub variables: BTreeMap<String, ResolvedVariable>,
    pub source_files: Vec<String>,
}

pub fn resolve_profile(
    document_uri: &Url,
    workspace_roots: &[PathBuf],
    selected: Option<&ProfileConfig>,
) -> Result<ResolvedProfile, String> {
    let Some(config) = selected.filter(|profile| profile.name != AUTO_PROFILE) else {
        let variables = resolve_workspace_variables(document_uri, workspace_roots);
        let source_files = variable_sources(&variables, workspace_roots);
        return Ok(ResolvedProfile {
            name: AUTO_PROFILE.into(),
            variables,
            source_files,
        });
    };
    let document_path = document_uri
        .to_file_path()
        .map_err(|_| "Environment profiles require a local file document.".to_string())?;
    let root = owning_workspace_root(&document_path, workspace_roots)
        .ok_or_else(|| "Document is not inside a workspace folder.".to_string())?;
    let canonical_root = root.canonicalize().unwrap_or(root);
    let mut paths = Vec::new();
    for configured in &config.files {
        let candidate = canonical_root.join(configured);
        let canonical = candidate
            .canonicalize()
            .map_err(|error| format!("Unable to read profile file `{configured}`: {error}"))?;
        if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
            return Err(format!(
                "Profile file `{configured}` must be a file inside the workspace folder."
            ));
        }
        paths.push(canonical);
    }
    let variables = resolve_variable_files(&paths);
    let source_files = paths
        .iter()
        .filter_map(|path| {
            path.strip_prefix(&canonical_root)
                .ok()
                .map(|relative| relative.to_string_lossy().into_owned())
        })
        .collect();
    Ok(ResolvedProfile {
        name: config.name.clone(),
        variables,
        source_files,
    })
}

fn owning_workspace_root(document: &std::path::Path, roots: &[PathBuf]) -> Option<PathBuf> {
    let document = document
        .canonicalize()
        .unwrap_or_else(|_| document.to_path_buf());
    roots
        .iter()
        .map(|root| root.canonicalize().unwrap_or_else(|_| root.clone()))
        .filter(|root| document.starts_with(root))
        .max_by_key(|root| root.components().count())
}

fn variable_sources(
    variables: &BTreeMap<String, ResolvedVariable>,
    roots: &[PathBuf],
) -> Vec<String> {
    let mut sources = variables
        .values()
        .filter_map(|variable| variable.uri.to_file_path().ok())
        .map(|path| {
            roots
                .iter()
                .filter_map(|root| path.strip_prefix(root).ok())
                .min_by_key(|relative| relative.components().count())
                .map(|relative| relative.to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.to_string_lossy().into_owned())
                })
        })
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn selected_profile_merges_files_in_declared_order() {
        let root = tmp_dir("hurl-lsp-profile");
        fs::create_dir_all(&root).expect("mkdir");
        fs::write(root.join("base.env"), "host=base\nbase_only=yes\n").expect("base");
        fs::write(root.join("local.env"), "host=local\n").expect("local");
        fs::write(root.join("request.hurl"), "GET https://example.com\n").expect("request");
        let uri = Url::from_file_path(root.join("request.hurl")).expect("uri");
        let profile = resolve_profile(
            &uri,
            std::slice::from_ref(&root),
            Some(&ProfileConfig {
                name: "Local".into(),
                files: vec!["base.env".into(), "local.env".into()],
            }),
        )
        .expect("profile");
        assert_eq!(profile.name, "Local");
        assert_eq!(profile.variables["host"].value, "local");
        assert_eq!(profile.variables["base_only"].value, "yes");
        assert_eq!(profile.source_files, vec!["base.env", "local.env"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn profile_rejects_files_outside_workspace() {
        let parent = tmp_dir("hurl-lsp-profile-bounded");
        let root = parent.join("workspace");
        fs::create_dir_all(&root).expect("mkdir");
        fs::write(parent.join("secret.env"), "token=secret\n").expect("secret");
        fs::write(root.join("request.hurl"), "GET https://example.com\n").expect("request");
        let uri = Url::from_file_path(root.join("request.hurl")).expect("uri");
        let error = resolve_profile(
            &uri,
            std::slice::from_ref(&root),
            Some(&ProfileConfig {
                name: "Invalid".into(),
                files: vec!["../secret.env".into()],
            }),
        )
        .expect_err("outside file must fail");
        assert!(error.contains("inside the workspace"));
        let _ = fs::remove_dir_all(parent);
    }

    fn tmp_dir(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{now}"))
    }
}
