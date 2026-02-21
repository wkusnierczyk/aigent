//! Structured diagnostics for validation, linting, and error reporting.
//!
//! Replaces the ad-hoc `Vec<String>` pattern with typed diagnostics carrying
//! stable error codes, severity levels, and optional fix suggestions.

use std::fmt;

use serde::Serialize;

/// Severity of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A rule violation that causes validation failure.
    Error,
    /// A potential issue that does not cause failure.
    Warning,
    /// An informational suggestion for improvement.
    Info,
}

/// A structured diagnostic message from validation or linting.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: Severity,
    /// Stable error code (e.g., `"E001"`, `"W001"`, `"I001"`).
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Field that caused the diagnostic (e.g., `"name"`, `"description"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<&'static str>,
    /// Suggested fix (actionable text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic with the given severity, code, and message.
    #[must_use]
    pub fn new(severity: Severity, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            field: None,
            suggestion: None,
        }
    }

    /// Set the field that caused this diagnostic.
    #[must_use]
    pub fn with_field(mut self, field: &'static str) -> Self {
        self.field = Some(field);
        self
    }

    /// Set a suggested fix for this diagnostic.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Returns `true` if this diagnostic is an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Returns `true` if this diagnostic is a warning.
    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.severity == Severity::Warning
    }

    /// Returns `true` if this diagnostic is informational.
    #[must_use]
    pub fn is_info(&self) -> bool {
        self.severity == Severity::Info
    }
}

/// Display format preserves backward compatibility:
/// - Errors: `"message"` (no prefix)
/// - Warnings: `"warning: message"`
/// - Info: `"info: message"`
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.severity {
            Severity::Error => write!(f, "{}", self.message),
            Severity::Warning => write!(f, "warning: {}", self.message),
            Severity::Info => write!(f, "info: {}", self.message),
        }
    }
}

// ── Error code constants ────────────────────────────────────────────────

// Infrastructure errors (E000)

/// Infrastructure error (file not found, IO error, parse failure).
pub const E000: &str = "E000";

// Name validation errors (E001–E009)

/// Name must not be empty.
pub const E001: &str = "E001";
/// Name exceeds 64 characters.
pub const E002: &str = "E002";
/// Name contains invalid character.
pub const E003: &str = "E003";
/// Name starts with hyphen.
pub const E004: &str = "E004";
/// Name ends with hyphen.
pub const E005: &str = "E005";
/// Name contains consecutive hyphens.
pub const E006: &str = "E006";
/// Name contains reserved word.
pub const E007: &str = "E007";
/// Name contains XML/HTML tags (reserved; currently caught by E003 character validation).
pub const E008: &str = "E008";
/// Name does not match directory name.
pub const E009: &str = "E009";

// Description validation errors (E010–E012)

/// Description must not be empty.
pub const E010: &str = "E010";
/// Description exceeds 1024 characters.
pub const E011: &str = "E011";
/// Description contains XML/HTML tags.
pub const E012: &str = "E012";

// Compatibility validation errors (E013)

/// Compatibility exceeds 500 characters.
pub const E013: &str = "E013";

// Field type errors (E014–E016)

/// `name` field is not a string.
pub const E014: &str = "E014";
/// `description` field is not a string.
pub const E015: &str = "E015";
/// `compatibility` field is not a string.
pub const E016: &str = "E016";

// Missing field errors (E017–E018)

/// Missing required field `name`.
pub const E017: &str = "E017";
/// Missing required field `description`.
pub const E018: &str = "E018";

// Warning codes (W001–W002)

/// Unexpected metadata field.
pub const W001: &str = "W001";
/// Body exceeds 500 lines.
pub const W002: &str = "W002";

// Structure validation codes (S001–S006)

/// Referenced file does not exist.
pub const S001: &str = "S001";
/// Script missing execute permission (Unix only).
pub const S002: &str = "S002";
/// Reference depth exceeds 1 level.
pub const S003: &str = "S003";
/// Excessive directory nesting depth.
pub const S004: &str = "S004";
/// Symlink detected in skill directory.
pub const S005: &str = "S005";
/// Path traversal in reference link.
pub const S006: &str = "S006";

// Conflict detection codes (C001–C003)

/// Name collision across skill directories.
pub const C001: &str = "C001";
/// Description overlap between skills.
pub const C002: &str = "C002";
/// Total token budget exceeded.
pub const C003: &str = "C003";

/// Validation target profile for controlling which fields are considered known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationTarget {
    /// Standard Anthropic specification fields only.
    #[default]
    Standard,
    /// Standard fields plus Claude Code extension fields.
    ClaudeCode,
    /// No unknown-field warnings (all fields accepted).
    Permissive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_no_prefix() {
        let d = Diagnostic::new(Severity::Error, E001, "name must not be empty");
        assert_eq!(d.to_string(), "name must not be empty");
    }

    #[test]
    fn warning_display_with_prefix() {
        let d = Diagnostic::new(Severity::Warning, W001, "unexpected metadata field: 'foo'");
        assert_eq!(d.to_string(), "warning: unexpected metadata field: 'foo'");
    }

    #[test]
    fn info_display_with_prefix() {
        let d = Diagnostic::new(Severity::Info, "I001", "description uses first person");
        assert_eq!(d.to_string(), "info: description uses first person");
    }

    #[test]
    fn is_error_true_for_errors() {
        let d = Diagnostic::new(Severity::Error, E001, "test");
        assert!(d.is_error());
        assert!(!d.is_warning());
        assert!(!d.is_info());
    }

    #[test]
    fn is_warning_true_for_warnings() {
        let d = Diagnostic::new(Severity::Warning, W001, "test");
        assert!(!d.is_error());
        assert!(d.is_warning());
        assert!(!d.is_info());
    }

    #[test]
    fn is_info_true_for_info() {
        let d = Diagnostic::new(Severity::Info, "I001", "test");
        assert!(!d.is_error());
        assert!(!d.is_warning());
        assert!(d.is_info());
    }

    #[test]
    fn with_field_sets_field() {
        let d = Diagnostic::new(Severity::Error, E001, "test").with_field("name");
        assert_eq!(d.field, Some("name"));
    }

    #[test]
    fn with_suggestion_sets_suggestion() {
        let d = Diagnostic::new(Severity::Error, E003, "invalid character")
            .with_suggestion("Use lowercase letters only");
        assert_eq!(d.suggestion.as_deref(), Some("Use lowercase letters only"));
    }

    #[test]
    fn new_has_no_field_or_suggestion() {
        let d = Diagnostic::new(Severity::Error, E001, "test");
        assert!(d.field.is_none());
        assert!(d.suggestion.is_none());
    }

    #[test]
    fn builder_pattern_chains() {
        let d = Diagnostic::new(Severity::Error, E003, "invalid character: 'X'")
            .with_field("name")
            .with_suggestion("Use lowercase: 'x'");
        assert_eq!(d.code, E003);
        assert_eq!(d.field, Some("name"));
        assert!(d.suggestion.is_some());
    }

    #[test]
    fn serialize_json_error() {
        let d = Diagnostic::new(Severity::Error, E001, "name must not be empty").with_field("name");
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["severity"], "error");
        assert_eq!(json["code"], "E001");
        assert_eq!(json["message"], "name must not be empty");
        assert_eq!(json["field"], "name");
        assert!(json.get("suggestion").is_none());
    }

    #[test]
    fn serialize_json_warning_with_suggestion() {
        let d = Diagnostic::new(Severity::Warning, W001, "unexpected field: 'foo'")
            .with_field("metadata")
            .with_suggestion("Remove the field");
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["suggestion"], "Remove the field");
    }

    #[test]
    fn serialize_json_omits_none_fields() {
        let d = Diagnostic::new(Severity::Error, E001, "test");
        let json = serde_json::to_value(&d).unwrap();
        assert!(json.get("field").is_none());
        assert!(json.get("suggestion").is_none());
    }

    #[test]
    fn error_codes_are_unique() {
        let codes = [
            E000, E001, E002, E003, E004, E005, E006, E007, E008, E009, E010, E011, E012, E013,
            E014, E015, E016, E017, E018, W001, W002, S001, S002, S003, S004, S005, S006, C001,
            C002, C003,
        ];
        let mut seen = std::collections::HashSet::new();
        for code in &codes {
            assert!(seen.insert(code), "duplicate error code: {code}");
        }
    }

    #[test]
    fn validation_target_default_is_standard() {
        let target = ValidationTarget::default();
        assert_eq!(target, ValidationTarget::Standard);
    }
}
