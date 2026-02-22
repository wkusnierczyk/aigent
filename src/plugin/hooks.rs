//! Hook configuration (`hooks.json`) validation.

use std::path::Path;

use serde::Deserialize;

use crate::diagnostics::{
    Diagnostic, Severity, H001, H002, H003, H004, H005, H006, H007, H008, H009, H010, H011,
};

/// Valid Claude Code hook event names.
const VALID_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "SubagentStop",
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PreCompact",
    "Notification",
];

/// Valid hook types.
const VALID_HOOK_TYPES: &[&str] = &["command", "prompt"];

/// Events where prompt hooks are most useful.
const OPTIMAL_PROMPT_EVENTS: &[&str] = &["Stop", "SubagentStop", "UserPromptSubmit", "PreToolUse"];

/// A single hook definition within an event entry.
#[derive(Debug, Deserialize)]
pub struct HookDefinition {
    /// Hook type: `command` or `prompt`.
    #[serde(rename = "type")]
    pub hook_type: Option<String>,
    /// Shell command to execute (for `command` hooks).
    pub command: Option<String>,
    /// Prompt text to inject (for `prompt` hooks).
    pub prompt: Option<String>,
    /// Timeout in seconds.
    pub timeout: Option<f64>,
}

/// An event entry containing a matcher and a list of hooks.
#[derive(Debug, Deserialize)]
pub struct HookEntry {
    /// Optional matcher pattern (e.g., tool name glob).
    pub matcher: Option<String>,
    /// List of hook definitions.
    pub hooks: Option<Vec<HookDefinition>>,
}

/// Validate a `hooks.json` file at the given path.
///
/// Returns a list of diagnostics. Never panics — parse errors are reported
/// as H001/H002 diagnostics.
#[must_use]
pub fn validate_hooks(path: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Read file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                H001,
                format!("cannot read hooks.json: {e}"),
            ));
            return diags;
        }
    };

    // H001: JSON syntax check (phase 1)
    let raw: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                H001,
                format!("invalid JSON syntax: {e}"),
            ));
            return diags;
        }
    };

    // H002: Structure check — must be an object of event arrays (phase 2)
    let events: std::collections::HashMap<String, Vec<HookEntry>> =
        match serde_json::from_value(raw) {
            Ok(m) => m,
            Err(e) => {
                diags.push(Diagnostic::new(
                    Severity::Error,
                    H002,
                    format!("invalid hooks structure: {e}"),
                ));
                return diags;
            }
        };

    for (event_name, entries) in &events {
        // H003: Unknown event name
        if !VALID_EVENTS.contains(&event_name.as_str()) {
            diags.push(
                Diagnostic::new(
                    Severity::Error,
                    H003,
                    format!("unknown event name: \"{event_name}\""),
                )
                .with_suggestion(format!("Valid events: {}", VALID_EVENTS.join(", "))),
            );
        }

        for entry in entries {
            // H004: Missing hooks array
            let hooks = match &entry.hooks {
                Some(h) => h,
                None => {
                    diags.push(Diagnostic::new(
                        Severity::Error,
                        H004,
                        format!("hook entry for \"{event_name}\" missing `hooks` array"),
                    ));
                    continue;
                }
            };

            for hook in hooks {
                // H005: Missing type field
                let hook_type = match &hook.hook_type {
                    Some(t) => t.as_str(),
                    None => {
                        diags.push(Diagnostic::new(
                            Severity::Error,
                            H005,
                            format!("hook in \"{event_name}\" missing `type` field"),
                        ));
                        continue;
                    }
                };

                // H006: Unknown hook type
                if !VALID_HOOK_TYPES.contains(&hook_type) {
                    diags.push(
                        Diagnostic::new(
                            Severity::Error,
                            H006,
                            format!("unknown hook type: \"{hook_type}\""),
                        )
                        .with_suggestion("Valid types: command, prompt"),
                    );
                    continue;
                }

                // H007: Command hook missing command field
                if hook_type == "command" && hook.command.is_none() {
                    diags.push(Diagnostic::new(
                        Severity::Error,
                        H007,
                        format!("command hook in \"{event_name}\" missing `command` field"),
                    ));
                }

                // H008: Prompt hook missing prompt field
                if hook_type == "prompt" && hook.prompt.is_none() {
                    diags.push(Diagnostic::new(
                        Severity::Error,
                        H008,
                        format!("prompt hook in \"{event_name}\" missing `prompt` field"),
                    ));
                }

                // H009: Timeout outside recommended range
                if let Some(timeout) = hook.timeout {
                    if !(5.0..=600.0).contains(&timeout) {
                        diags.push(
                            Diagnostic::new(
                                Severity::Warning,
                                H009,
                                format!("timeout {timeout}s is outside recommended range (5–600s)"),
                            )
                            .with_suggestion("Use a timeout between 5 and 600 seconds"),
                        );
                    }
                }

                // H010: Hardcoded absolute path in command
                if hook_type == "command" {
                    if let Some(cmd) = &hook.command {
                        if cmd.starts_with('/') {
                            diags.push(
                                Diagnostic::new(
                                    Severity::Warning,
                                    H010,
                                    format!("absolute path in command: \"{cmd}\""),
                                )
                                .with_suggestion("Use ${CLAUDE_PLUGIN_ROOT} for portable paths"),
                            );
                        }
                    }
                }

                // H011: Prompt hook on suboptimal event
                if hook_type == "prompt" && !OPTIMAL_PROMPT_EVENTS.contains(&event_name.as_str()) {
                    diags.push(Diagnostic::new(
                        Severity::Info,
                        H011,
                        format!(
                            "prompt hook on \"{event_name}\" — prompt hooks work best on {}",
                            OPTIMAL_PROMPT_EVENTS.join(", ")
                        ),
                    ));
                }
            }
        }
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_hooks(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        fs::write(&path, content).unwrap();
        (dir, path)
    }

    #[test]
    fn valid_hooks_no_errors() {
        let (_dir, path) = write_hooks(
            r#"{
                "PreToolUse": [{
                    "matcher": "Bash",
                    "hooks": [{ "type": "command", "command": "echo test", "timeout": 10 }]
                }]
            }"#,
        );
        let diags = validate_hooks(&path);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn invalid_json_h001() {
        let (_dir, path) = write_hooks("{ not json }");
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H001));
    }

    #[test]
    fn invalid_structure_h002() {
        // Valid JSON but not an object of event arrays
        let (_dir, path) = write_hooks(r#"["not", "an", "object"]"#);
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H002));
    }

    #[test]
    fn unknown_event_h003() {
        let (_dir, path) = write_hooks(
            r#"{ "OnSave": [{ "hooks": [{ "type": "command", "command": "echo" }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H003));
    }

    #[test]
    fn missing_hooks_array_h004() {
        let (_dir, path) = write_hooks(r#"{ "PreToolUse": [{ "matcher": "Bash" }] }"#);
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H004));
    }

    #[test]
    fn missing_type_h005() {
        let (_dir, path) =
            write_hooks(r#"{ "PreToolUse": [{ "hooks": [{ "command": "echo test" }] }] }"#);
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H005));
    }

    #[test]
    fn unknown_hook_type_h006() {
        let (_dir, path) = write_hooks(
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "script", "command": "echo" }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H006));
    }

    #[test]
    fn command_missing_command_h007() {
        let (_dir, path) =
            write_hooks(r#"{ "PreToolUse": [{ "hooks": [{ "type": "command" }] }] }"#);
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H007));
    }

    #[test]
    fn prompt_missing_prompt_h008() {
        let (_dir, path) = write_hooks(r#"{ "Stop": [{ "hooks": [{ "type": "prompt" }] }] }"#);
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H008));
    }

    #[test]
    fn timeout_out_of_range_h009() {
        let (_dir, path) = write_hooks(
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "echo", "timeout": 1 }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H009));
    }

    #[test]
    fn timeout_too_high_h009() {
        let (_dir, path) = write_hooks(
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "echo", "timeout": 700 }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H009));
    }

    #[test]
    fn timeout_in_range_no_h009() {
        let (_dir, path) = write_hooks(
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "echo", "timeout": 30 }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(!diags.iter().any(|d| d.code == H009));
    }

    #[test]
    fn absolute_path_h010() {
        let (_dir, path) = write_hooks(
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "/usr/bin/test" }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H010));
    }

    #[test]
    fn relative_path_no_h010() {
        let (_dir, path) = write_hooks(
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "./scripts/test.sh" }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(!diags.iter().any(|d| d.code == H010));
    }

    #[test]
    fn prompt_on_suboptimal_event_h011() {
        let (_dir, path) = write_hooks(
            r#"{ "SessionStart": [{ "hooks": [{ "type": "prompt", "prompt": "Be careful" }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(diags.iter().any(|d| d.code == H011));
    }

    #[test]
    fn prompt_on_optimal_event_no_h011() {
        let (_dir, path) = write_hooks(
            r#"{ "Stop": [{ "hooks": [{ "type": "prompt", "prompt": "Review output" }] }] }"#,
        );
        let diags = validate_hooks(&path);
        assert!(!diags.iter().any(|d| d.code == H011));
    }

    #[test]
    fn nonexistent_file_returns_h001() {
        let diags = validate_hooks(Path::new("/nonexistent/hooks.json"));
        assert!(diags.iter().any(|d| d.code == H001));
    }

    #[test]
    fn all_valid_events_accepted() {
        for event in VALID_EVENTS {
            let json = format!(
                r#"{{ "{event}": [{{ "hooks": [{{ "type": "command", "command": "echo" }}] }}] }}"#
            );
            let (_dir, path) = write_hooks(&json);
            let diags = validate_hooks(&path);
            assert!(
                !diags.iter().any(|d| d.code == H003),
                "event {event} should be valid"
            );
        }
    }
}
