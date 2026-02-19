use thiserror::Error;

/// Errors that can occur during skill operations.
#[derive(Error, Debug)]
pub enum AigentError {
    /// SKILL.md parsing failed.
    #[error("parse error: {message}")]
    Parse { message: String },

    /// Skill validation found problems.
    #[error("validation failed")]
    Validation { errors: Vec<String> },

    /// Filesystem I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML deserialization error.
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Skill build error.
    #[error("build error: {message}")]
    Build { message: String },
}

/// Convenience alias for `Result<T, AigentError>`.
pub type Result<T> = std::result::Result<T, AigentError>;
