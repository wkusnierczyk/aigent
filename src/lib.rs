pub mod builder;
pub mod errors;
pub mod models;
pub mod parser;
pub mod prompt;
pub mod validator;

// Re-export key types at crate root for convenience.
pub use errors::{AigentError, Result};
pub use models::SkillProperties;
pub use parser::{find_skill_md, parse_frontmatter, read_properties};
pub use prompt::to_prompt;
pub use validator::{validate, validate_metadata};

pub use builder::{
    assess_clarity, build_skill, derive_name, BuildResult, ClarityAssessment, SkillSpec,
};
