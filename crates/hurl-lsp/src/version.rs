pub fn display_version() -> String {
    resolve_display_version(option_env!("HURL_LSP_VERSION"), env!("CARGO_PKG_VERSION"))
}

fn resolve_display_version(injected: Option<&str>, fallback: &str) -> String {
    injected
        .and_then(normalize_tag_version)
        .unwrap_or_else(|| fallback.to_string())
}

fn normalize_tag_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let stripped = trimmed.strip_prefix('v').unwrap_or(trimmed);
    if is_semver_like(stripped) {
        return Some(stripped.to_string());
    }
    None
}

fn is_semver_like(value: &str) -> bool {
    let (core, suffix) = split_semver_suffix(value);
    if core.is_empty() {
        return false;
    }
    let mut parts = core.split('.');
    let (Some(major), Some(minor), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if !(is_numeric_component(major) && is_numeric_component(minor) && is_numeric_component(patch))
    {
        return false;
    }
    if let Some(rest) = suffix {
        if rest.len() < 2 {
            return false;
        }
        let marker = rest.as_bytes()[0];
        if marker != b'-' && marker != b'+' {
            return false;
        }
        if !rest[1..]
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
        {
            return false;
        }
    }
    true
}

fn split_semver_suffix(value: &str) -> (&str, Option<&str>) {
    let mut boundary = value.len();
    if let Some(pos) = value.find('-') {
        boundary = boundary.min(pos);
    }
    if let Some(pos) = value.find('+') {
        boundary = boundary.min(pos);
    }
    if boundary == value.len() {
        return (value, None);
    }
    (&value[..boundary], Some(&value[boundary..]))
}

fn is_numeric_component(component: &str) -> bool {
    !component.is_empty() && component.chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{normalize_tag_version, resolve_display_version};

    #[test]
    fn normalize_tag_version_supports_prefixed_version() {
        assert_eq!(normalize_tag_version("v0.1.9"), Some("0.1.9".to_string()));
    }

    #[test]
    fn normalize_tag_version_supports_plain_version() {
        assert_eq!(normalize_tag_version("0.1.9"), Some("0.1.9".to_string()));
    }

    #[test]
    fn normalize_tag_version_rejects_non_version_like_values() {
        assert_eq!(normalize_tag_version("main"), None);
        assert_eq!(normalize_tag_version("v1foo"), None);
        assert_eq!(normalize_tag_version("1.2"), None);
        assert_eq!(normalize_tag_version("1.2.3.4"), None);
        assert_eq!(normalize_tag_version("1.2.x"), None);
        assert_eq!(normalize_tag_version(""), None);
    }

    #[test]
    fn normalize_tag_version_supports_prerelease_and_build_suffixes() {
        assert_eq!(
            normalize_tag_version("v1.2.3-rc.1"),
            Some("1.2.3-rc.1".to_string())
        );
        assert_eq!(
            normalize_tag_version("1.2.3+build.7"),
            Some("1.2.3+build.7".to_string())
        );
    }

    #[test]
    fn resolve_display_version_prefers_injected_when_valid() {
        assert_eq!(
            resolve_display_version(Some("v2.0.0"), "0.1.8"),
            "2.0.0".to_string()
        );
    }

    #[test]
    fn resolve_display_version_falls_back_for_invalid_injected() {
        assert_eq!(
            resolve_display_version(Some("release"), "0.1.8"),
            "0.1.8".to_string()
        );
        assert_eq!(resolve_display_version(None, "0.1.8"), "0.1.8".to_string());
    }
}
