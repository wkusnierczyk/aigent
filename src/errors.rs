use thiserror::Error;

/// Errors that can occur during skill operations.
#[derive(Error, Debug)]
pub enum AigentError {
    /// SKILL.md parsing failed.
    #[error("parse error: {message}")]
    Parse {
        /// Description of the parse failure.
        message: String,
    },

    /// Skill validation found problems.
    #[error("{}", format_validation_errors(errors))]
    Validation {
        /// List of validation error messages.
        errors: Vec<String>,
    },

    /// Filesystem I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML deserialization error.
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    /// Skill build error.
    #[error("build error: {message}")]
    Build {
        /// Description of the build failure.
        message: String,
    },
}

/// Format validation errors for display.
///
/// - Empty list → `"Validation failed: no details"`
/// - Single error → the error message itself
/// - Multiple errors → bullet list prefixed with `"Validation failed:"`
fn format_validation_errors(errors: &[String]) -> String {
    match errors.len() {
        0 => "Validation failed: no details".to_string(),
        1 => errors[0].clone(),
        _ => {
            let bullets: String = errors.iter().map(|e| format!("\n  - {e}")).collect();
            format!("Validation failed:{bullets}")
        }
    }
}

/// Convenience alias for `Result<T, AigentError>`.
pub type Result<T> = std::result::Result<T, AigentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_display() {
        let err = AigentError::Parse {
            message: "bad yaml".to_string(),
        };
        assert_eq!(err.to_string(), "parse error: bad yaml");
    }

    #[test]
    fn validation_single_error_display() {
        let err = AigentError::Validation {
            errors: vec!["name too long".to_string()],
        };
        assert_eq!(err.to_string(), "name too long");
    }

    #[test]
    fn validation_multiple_errors_display() {
        let err = AigentError::Validation {
            errors: vec!["bad name".to_string(), "too long".to_string()],
        };
        assert_eq!(
            err.to_string(),
            "Validation failed:\n  - bad name\n  - too long"
        );
    }

    #[test]
    fn validation_empty_errors_display() {
        let err = AigentError::Validation { errors: vec![] };
        assert_eq!(err.to_string(), "Validation failed: no details");
    }

    #[test]
    fn build_display() {
        let err = AigentError::Build {
            message: "LLM unavailable".to_string(),
        };
        assert_eq!(err.to_string(), "build error: LLM unavailable");
    }

    #[test]
    fn io_error_converts_via_from() {
        fn trigger() -> Result<()> {
            let _f = std::fs::File::open("/nonexistent/path/that/does/not/exist")?;
            Ok(())
        }
        let err = trigger().unwrap_err();
        assert!(matches!(err, AigentError::Io(_)));
        assert!(err.to_string().starts_with("IO error:"));
    }

    #[test]
    fn yaml_error_converts_via_from() {
        fn trigger() -> Result<()> {
            let _: serde_yaml_ng::Value = serde_yaml_ng::from_str(":\n  :\n   :")?;
            Ok(())
        }
        let err = trigger().unwrap_err();
        assert!(matches!(err, AigentError::Yaml(_)));
        assert!(err.to_string().starts_with("YAML error:"));
    }

    #[test]
    fn validation_errors_accessible_via_match() {
        let err = AigentError::Validation {
            errors: vec!["err1".to_string(), "err2".to_string(), "err3".to_string()],
        };
        match err {
            AigentError::Validation { errors } => {
                assert_eq!(errors.len(), 3);
                assert_eq!(errors[0], "err1");
                assert_eq!(errors[2], "err3");
            }
            _ => panic!("expected Validation variant"),
        }
    }

    #[test]
    fn parse_message_accessible_via_match() {
        let err = AigentError::Parse {
            message: "unexpected EOF".to_string(),
        };
        match err {
            AigentError::Parse { message } => assert_eq!(message, "unexpected EOF"),
            _ => panic!("expected Parse variant"),
        }
    }

    #[test]
    fn aigent_error_implements_std_error() {
        let err = AigentError::Parse {
            message: "test".to_string(),
        };
        // std::error::Error requires Debug + Display
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn result_alias_works_with_question_mark() {
        fn ok_path() -> Result<i32> {
            Ok(42)
        }
        fn err_path() -> Result<i32> {
            Err(AigentError::Build {
                message: "fail".to_string(),
            })
        }
        assert_eq!(ok_path().unwrap(), 42);
        assert!(err_path().is_err());
    }
}
