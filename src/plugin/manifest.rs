//! Plugin manifest (`plugin.json`) validation.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use crate::diagnostics::{
    Diagnostic, Severity, P001, P002, P003, P004, P005, P006, P007, P008, P009, P010,
};

/// Regex for valid kebab-case names: lowercase letters, digits, hyphens.
static KEBAB_CASE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$").expect("kebab-case regex"));

/// Regex for semver: x.y.z (no pre-release/build metadata).
static SEMVER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").expect("semver regex"));

/// Regex for detecting hardcoded credentials in string values.
static CREDENTIAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(api[_-]?key|token|secret|password|credential)\s*[:=]\s*["'][^"']+["']"#)
        .expect("credential regex")
});

/// Recommended metadata fields that improve discoverability.
const RECOMMENDED_FIELDS: &[(&str, &str)] = &[
    ("author", "Add an author field for attribution"),
    ("homepage", "Add a homepage URL for documentation"),
    ("license", "Add a license field for legal clarity"),
];

/// Author field: either a simple string or a detailed object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AuthorField {
    /// Simple string author (e.g., `"Jane Doe"`).
    Simple(String),
    /// Detailed author with name and optional URL.
    Detailed {
        /// Author name.
        name: String,
        /// Author URL.
        url: Option<String>,
    },
}

/// Parsed plugin manifest from `plugin.json`.
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (required).
    pub name: Option<String>,
    /// Plugin description.
    pub description: Option<String>,
    /// Plugin version (semver).
    pub version: Option<String>,
    /// Plugin author.
    pub author: Option<AuthorField>,
    /// Project homepage URL.
    pub homepage: Option<String>,
    /// Source repository URL.
    pub repository: Option<String>,
    /// License identifier.
    pub license: Option<String>,
    /// Search keywords.
    pub keywords: Option<Vec<String>>,
    /// Custom commands directory path.
    pub commands: Option<String>,
    /// Custom agents directory path.
    pub agents: Option<String>,
    /// Custom skills directory path.
    pub skills: Option<String>,
    /// Custom hooks configuration path.
    pub hooks: Option<String>,
    /// MCP server configuration (path string or inline object).
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<serde_json::Value>,
    /// Custom output styles path.
    #[serde(rename = "outputStyles")]
    pub output_styles: Option<String>,
    /// LSP server configuration path.
    #[serde(rename = "lspServers")]
    pub lsp_servers: Option<String>,
}

impl PluginManifest {
    /// Return path overrides as (field_name, path_value) pairs.
    fn path_overrides(&self) -> Vec<(&'static str, &str)> {
        let string_fields: [(&str, &Option<String>); 6] = [
            ("commands", &self.commands),
            ("agents", &self.agents),
            ("skills", &self.skills),
            ("hooks", &self.hooks),
            ("outputStyles", &self.output_styles),
            ("lspServers", &self.lsp_servers),
        ];
        let mut result: Vec<(&'static str, &str)> = string_fields
            .into_iter()
            .filter_map(|(name, val)| val.as_deref().map(|v| (name, v)))
            .collect();
        // mcpServers can be a string (path) or object (inline config); only
        // treat it as a path override when it's a string.
        if let Some(serde_json::Value::String(s)) = &self.mcp_servers {
            result.push(("mcpServers", s.as_str()));
        }
        result
    }
}

/// Validate a `plugin.json` file at the given path.
///
/// Returns a list of diagnostics (errors, warnings, info). Never panics or
/// fails — parse errors are reported as P001 diagnostics.
#[must_use]
pub fn validate_manifest(path: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Read file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                P001,
                format!("cannot read plugin.json: {e}"),
            ));
            return diags;
        }
    };

    // P001: JSON syntax check
    let raw: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                P001,
                format!("invalid JSON syntax: {e}"),
            ));
            return diags;
        }
    };

    // Deserialize into typed struct
    let manifest: PluginManifest = match serde_json::from_value(raw.clone()) {
        Ok(m) => m,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                P001,
                format!("invalid manifest structure: {e}"),
            ));
            return diags;
        }
    };

    // P002: name field missing
    let name = match &manifest.name {
        Some(n) if !n.is_empty() => n.as_str(),
        Some(_) => {
            diags.push(
                Diagnostic::new(Severity::Error, P002, "`name` must not be empty")
                    .with_field("name"),
            );
            ""
        }
        None => {
            diags.push(
                Diagnostic::new(Severity::Error, P002, "missing required field `name`")
                    .with_field("name"),
            );
            ""
        }
    };

    // P003: name not kebab-case
    if !name.is_empty() && !KEBAB_CASE_RE.is_match(name) {
        diags.push(
            Diagnostic::new(
                Severity::Error,
                P003,
                format!("`name` is not valid kebab-case: \"{name}\""),
            )
            .with_field("name")
            .with_suggestion("Use lowercase letters, digits, and hyphens (e.g., \"my-plugin\")"),
        );
    }

    // P004: version not semver
    if let Some(version) = &manifest.version {
        if !SEMVER_RE.is_match(version) {
            diags.push(
                Diagnostic::new(
                    Severity::Warning,
                    P004,
                    format!("`version` is not valid semver: \"{version}\""),
                )
                .with_field("version")
                .with_suggestion("Use x.y.z format (e.g., \"1.0.0\")"),
            );
        }
    }

    // P005: description empty or missing
    match &manifest.description {
        Some(d) if d.trim().is_empty() => {
            diags.push(
                Diagnostic::new(Severity::Warning, P005, "`description` is empty")
                    .with_field("description"),
            );
        }
        None => {
            diags.push(
                Diagnostic::new(Severity::Warning, P005, "missing `description` field")
                    .with_field("description"),
            );
        }
        Some(_) => {}
    }

    // P006/P007: path override checks
    let plugin_dir = path.parent().unwrap_or(Path::new("."));
    for (field, value) in manifest.path_overrides() {
        // P006: absolute path
        if Path::new(value).is_absolute() {
            diags.push(
                Diagnostic::new(
                    Severity::Error,
                    P006,
                    format!("`{field}` uses absolute path: \"{value}\""),
                )
                .with_field(field)
                .with_suggestion("Use a relative path (e.g., \"./my-commands\")"),
            );
        }

        // P007: path does not exist
        let resolved = plugin_dir.join(value);
        if !resolved.exists() {
            diags.push(
                Diagnostic::new(
                    Severity::Error,
                    P007,
                    format!("`{field}` path does not exist: \"{value}\""),
                )
                .with_field(field),
            );
        }
    }

    // P008: credential scanning (scan all string values in the JSON)
    scan_credentials(&raw, &mut diags, &mut Vec::new());

    // P009: insecure MCP server URLs
    if let Some(obj) = raw.get("mcpServers").and_then(|v| v.as_object()) {
        for (server_name, config) in obj {
            if let Some(url) = config.get("url").and_then(|u| u.as_str()) {
                let url_lower = url.to_ascii_lowercase();
                if url_lower.starts_with("http://") || url_lower.starts_with("ws://") {
                    diags.push(
                        Diagnostic::new(
                            Severity::Warning,
                            P009,
                            format!("MCP server \"{server_name}\" uses insecure URL: \"{url}\""),
                        )
                        .with_field("mcpServers")
                        .with_suggestion("Use HTTPS or WSS for secure communication"),
                    );
                }
            }
        }
    }

    // P010: recommended fields
    for (field, suggestion) in RECOMMENDED_FIELDS {
        if raw.get(field).is_none() {
            diags.push(
                Diagnostic::new(
                    Severity::Info,
                    P010,
                    format!("missing recommended field `{field}`"),
                )
                .with_field(field)
                .with_suggestion(*suggestion),
            );
        }
    }

    diags
}

/// Recursively scan all string values in a JSON tree for credential patterns.
///
/// Tracks the JSON path for actionable diagnostic messages.
fn scan_credentials(
    value: &serde_json::Value,
    diags: &mut Vec<Diagnostic>,
    path: &mut Vec<String>,
) {
    match value {
        serde_json::Value::String(s) => {
            if CREDENTIAL_RE.is_match(s) {
                let location = if path.is_empty() {
                    String::new()
                } else {
                    format!(" at `{}`", path.join(""))
                };
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        P008,
                        format!("possible hardcoded credential detected{location}"),
                    )
                    .with_suggestion(
                        "Use environment variables or a secrets manager instead of inline credentials",
                    ),
                );
            }
        }
        serde_json::Value::Object(map) => {
            for (key, v) in map {
                let segment = if path.is_empty() {
                    key.clone()
                } else {
                    format!(".{key}")
                };
                path.push(segment);
                scan_credentials(v, diags, path);
                path.pop();
            }
        }
        serde_json::Value::Array(arr) => {
            for (idx, v) in arr.iter().enumerate() {
                path.push(format!("[{idx}]"));
                scan_credentials(v, diags, path);
                path.pop();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Write a plugin.json to a temp dir and return (dir, path).
    fn write_manifest(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("plugin.json");
        fs::write(&path, content).unwrap();
        (dir, path)
    }

    #[test]
    fn valid_manifest_no_errors() {
        let (_dir, path) = write_manifest(
            r#"{
                "name": "my-plugin",
                "description": "A test plugin",
                "version": "1.0.0",
                "author": { "name": "Test", "url": "https://example.com" },
                "homepage": "https://example.com",
                "license": "MIT"
            }"#,
        );
        let diags = validate_manifest(&path);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn invalid_json_p001() {
        let (_dir, path) = write_manifest("{ not json }");
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P001));
    }

    #[test]
    fn missing_name_p002() {
        let (_dir, path) = write_manifest(r#"{ "description": "test" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P002));
    }

    #[test]
    fn empty_name_p002() {
        let (_dir, path) = write_manifest(r#"{ "name": "" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P002));
    }

    #[test]
    fn invalid_name_p003() {
        let (_dir, path) = write_manifest(r#"{ "name": "My Plugin" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P003));
    }

    #[test]
    fn uppercase_name_p003() {
        let (_dir, path) = write_manifest(r#"{ "name": "MyPlugin" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P003));
    }

    #[test]
    fn valid_kebab_name_no_p003() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "my-plugin", "description": "test", "author": "x", "homepage": "x", "license": "MIT" }"#,
        );
        let diags = validate_manifest(&path);
        assert!(!diags.iter().any(|d| d.code == P003));
    }

    #[test]
    fn invalid_version_p004() {
        let (_dir, path) = write_manifest(r#"{ "name": "test", "version": "1.0" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P004));
    }

    #[test]
    fn valid_version_no_p004() {
        let (_dir, path) = write_manifest(r#"{ "name": "test", "version": "1.2.3" }"#);
        let diags = validate_manifest(&path);
        assert!(!diags.iter().any(|d| d.code == P004));
    }

    #[test]
    fn missing_description_p005() {
        let (_dir, path) = write_manifest(r#"{ "name": "test" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P005));
    }

    #[test]
    fn empty_description_p005() {
        let (_dir, path) = write_manifest(r#"{ "name": "test", "description": "  " }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P005));
    }

    #[test]
    fn absolute_path_p006() {
        let (_dir, path) = write_manifest(r#"{ "name": "test", "commands": "/usr/local/cmds" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P006));
    }

    #[test]
    fn nonexistent_path_p007() {
        let (_dir, path) = write_manifest(r#"{ "name": "test", "commands": "./nonexistent-dir" }"#);
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P007));
    }

    #[test]
    fn existing_path_no_p007() {
        let dir = tempdir().unwrap();
        let cmds_dir = dir.path().join("my-commands");
        fs::create_dir(&cmds_dir).unwrap();
        let path = dir.path().join("plugin.json");
        fs::write(&path, r#"{ "name": "test", "commands": "./my-commands" }"#).unwrap();
        let diags = validate_manifest(&path);
        assert!(!diags.iter().any(|d| d.code == P007));
    }

    #[test]
    fn credential_detection_p008() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "config": { "value": "api_key: 'sk-1234abcd'" } }"#,
        );
        let diags = validate_manifest(&path);
        let p008 = diags.iter().find(|d| d.code == P008);
        assert!(p008.is_some());
        assert!(
            p008.unwrap().message.contains("config.value"),
            "P008 should include JSON path: {}",
            p008.unwrap().message
        );
    }

    #[test]
    fn no_credential_false_positive() {
        let (_dir, path) =
            write_manifest(r#"{ "name": "test", "description": "Uses API key rotation" }"#);
        let diags = validate_manifest(&path);
        assert!(!diags.iter().any(|d| d.code == P008));
    }

    #[test]
    fn insecure_mcp_url_p009() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "mcpServers": { "local": { "url": "http://localhost:3000" } } }"#,
        );
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P009));
    }

    #[test]
    fn insecure_ws_url_p009() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "mcpServers": { "ws-server": { "url": "ws://localhost:8080" } } }"#,
        );
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P009));
    }

    #[test]
    fn insecure_url_case_insensitive_p009() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "mcpServers": { "mixed": { "url": "HTTP://localhost:3000" } } }"#,
        );
        let diags = validate_manifest(&path);
        assert!(diags.iter().any(|d| d.code == P009));
    }

    #[test]
    fn secure_mcp_url_no_p009() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "mcpServers": { "remote": { "url": "https://api.example.com" } } }"#,
        );
        let diags = validate_manifest(&path);
        assert!(!diags.iter().any(|d| d.code == P009));
    }

    #[test]
    fn missing_recommended_fields_p010() {
        let (_dir, path) = write_manifest(r#"{ "name": "test" }"#);
        let diags = validate_manifest(&path);
        let p010s: Vec<_> = diags.iter().filter(|d| d.code == P010).collect();
        assert_eq!(p010s.len(), 3); // author, homepage, license
    }

    #[test]
    fn all_recommended_fields_no_p010() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "author": "x", "homepage": "x", "license": "MIT" }"#,
        );
        let diags = validate_manifest(&path);
        assert!(!diags.iter().any(|d| d.code == P010));
    }

    #[test]
    fn nonexistent_file_returns_p001() {
        let diags = validate_manifest(Path::new("/nonexistent/plugin.json"));
        assert!(diags.iter().any(|d| d.code == P001));
    }

    #[test]
    fn author_simple_string_accepted() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "author": "Jane Doe", "homepage": "x", "license": "MIT" }"#,
        );
        let diags = validate_manifest(&path);
        assert!(!diags
            .iter()
            .any(|d| d.code == P010 && d.field == Some("author")));
    }

    #[test]
    fn author_detailed_accepted() {
        let (_dir, path) = write_manifest(
            r#"{ "name": "test", "author": { "name": "Jane", "url": "https://x.com" }, "homepage": "x", "license": "MIT" }"#,
        );
        let diags = validate_manifest(&path);
        assert!(!diags
            .iter()
            .any(|d| d.code == P010 && d.field == Some("author")));
    }
}
