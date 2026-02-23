use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use aigent::builder::template::SkillTemplate;
use aigent::diagnostics::ValidationTarget;

mod build;
mod check;
mod doc;
mod format;
mod init;
mod new;
mod probe;
mod prompt;
mod properties;
mod score;
mod test;
mod upgrade;
mod validate;
mod validate_plugin;
#[cfg(feature = "watch")]
mod watch;

#[derive(Parser)]
#[command(
    name = "aigent",
    version,
    about = "AI agent skill builder and validator"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Show project information
    #[arg(long)]
    about: bool,
}

/// Output format for validation results.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Format {
    /// Human-readable text output (default)
    #[default]
    Text,
    /// JSON array of diagnostic objects
    Json,
}

/// Validation target profile for controlling known-field detection.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Target {
    /// Anthropic specification fields only (default)
    #[default]
    Standard,
    /// Specification fields plus Claude Code extension fields
    ClaudeCode,
    /// No unknown-field warnings (all fields accepted)
    Permissive,
}

impl From<Target> for ValidationTarget {
    fn from(t: Target) -> Self {
        match t {
            Target::Standard => ValidationTarget::Standard,
            Target::ClaudeCode => ValidationTarget::ClaudeCode,
            Target::Permissive => ValidationTarget::Permissive,
        }
    }
}

/// Output format for prompt generation.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum PromptOutputFormat {
    /// XML format (default, matches Anthropic spec)
    #[default]
    Xml,
    /// JSON array
    Json,
    /// YAML document
    Yaml,
    /// Markdown document
    Markdown,
}

impl From<PromptOutputFormat> for aigent::prompt::PromptFormat {
    fn from(f: PromptOutputFormat) -> Self {
        match f {
            PromptOutputFormat::Xml => aigent::prompt::PromptFormat::Xml,
            PromptOutputFormat::Json => aigent::prompt::PromptFormat::Json,
            PromptOutputFormat::Yaml => aigent::prompt::PromptFormat::Yaml,
            PromptOutputFormat::Markdown => aigent::prompt::PromptFormat::Markdown,
        }
    }
}

#[derive(Subcommand)]
#[command(next_display_order = None)]
enum Commands {
    /// Validate skill directories (spec conformance)
    Validate {
        /// Paths to skill directories or SKILL.md files [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Validation target profile
        #[arg(long, value_enum, default_value_t = Target::Standard)]
        target: Target,
        /// Run directory structure checks
        #[arg(long)]
        structure: bool,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
        /// Apply automatic fixes for fixable issues
        #[arg(long)]
        apply_fixes: bool,
        /// Watch for changes and re-validate (requires 'watch' feature)
        #[arg(long)]
        watch: bool,
    },
    /// Run validate + semantic quality checks (superset of validate)
    #[command(alias = "lint")]
    Check {
        /// Paths to skill directories or SKILL.md files [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Validation target profile
        #[arg(long, value_enum, default_value_t = Target::Standard)]
        target: Target,
        /// Skip spec conformance checks (semantic quality only)
        #[arg(long)]
        no_validate: bool,
        /// Run directory structure checks
        #[arg(long)]
        structure: bool,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
        /// Apply automatic fixes for fixable issues
        #[arg(long)]
        apply_fixes: bool,
    },
    /// Read skill properties as JSON
    #[command(alias = "read-properties")]
    Properties {
        /// Path to skill directory or SKILL.md file [default: .]
        #[arg(name = "skill-dir", default_value = ".")]
        skill_dir: PathBuf,
    },
    /// Generate prompt from skill directories
    #[command(alias = "to-prompt")]
    Prompt {
        /// Paths to skill directories [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Output format
        #[arg(long, value_enum, default_value_t = PromptOutputFormat::Xml)]
        format: PromptOutputFormat,
        /// Show estimated token budget
        #[arg(long)]
        budget: bool,
        /// Write output to file instead of stdout (exit 0 = unchanged, 1 = changed)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Create a new skill from a natural language description
    #[command(alias = "create")]
    New {
        /// What the skill should do
        purpose: String,
        /// Override the derived skill name
        #[arg(long)]
        name: Option<String>,
        /// Output directory
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Force deterministic mode (no LLM)
        #[arg(long)]
        no_llm: bool,
        /// Interactive mode with step-by-step confirmation
        #[arg(long, short = 'i')]
        interactive: bool,
        /// Skip scaffolding of examples/ and scripts/ directories
        #[arg(long)]
        minimal: bool,
    },
    /// Score a skill against best-practices checklist
    Score {
        /// Path to skill directory or SKILL.md file [default: .]
        #[arg(name = "skill-dir", default_value = ".")]
        skill_dir: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Generate a markdown skill catalog
    Doc {
        /// Paths to skill directories [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Write output to file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Probe skill activation against a sample query
    Probe {
        /// Paths to skill directories or SKILL.md files [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Sample user query to test activation against
        #[arg(long, short)]
        query: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Assemble skills into a Claude Code plugin
    Build {
        /// Paths to skill directories [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Output directory for the assembled plugin
        #[arg(long, default_value = "./dist")]
        output: PathBuf,
        /// Override plugin name
        #[arg(long)]
        name: Option<String>,
        /// Run validation on assembled skills
        #[arg(long)]
        validate: bool,
    },
    /// Run fixture-based test suite from tests.yml
    Test {
        /// Paths to skill directories [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
        /// Generate a starter tests.yml for skills that lack one
        #[arg(long)]
        generate: bool,
    },
    /// Check a skill for upgrade opportunities
    Upgrade {
        /// Path to skill directory or SKILL.md file [default: .]
        #[arg(name = "skill-dir", default_value = ".")]
        skill_dir: PathBuf,
        /// Apply automatic upgrades
        #[arg(long)]
        apply: bool,
        /// Run validate + lint before upgrade (with --apply, also fix errors first)
        #[arg(long)]
        full: bool,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Format SKILL.md files (canonical key order, clean whitespace)
    #[command(alias = "fmt")]
    Format {
        /// Paths to skill directories or SKILL.md files [default: .]
        #[arg(default_value = ".")]
        skill_dirs: Vec<PathBuf>,
        /// Check formatting without modifying files (exit 1 if unformatted)
        #[arg(long)]
        check: bool,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Validate a Claude Code plugin directory
    ValidatePlugin {
        /// Path to plugin root directory [default: .]
        #[arg(name = "plugin-dir", default_value = ".")]
        plugin_dir: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Initialize a skill directory with a template SKILL.md
    Init {
        /// Target directory
        dir: Option<PathBuf>,
        /// Template variant for skill structure
        #[arg(long, value_enum, default_value_t = SkillTemplate::Minimal)]
        template: SkillTemplate,
        /// Skip scaffolding of examples/ and scripts/ directories
        #[arg(long)]
        minimal: bool,
    },
}

pub fn run(cli: Cli) {
    if cli.about {
        print_about();
        return;
    }

    match cli.command {
        Some(Commands::Validate {
            skill_dirs,
            format,
            target,
            structure,
            recursive,
            apply_fixes,
            watch,
        }) => validate::run(
            skill_dirs,
            format,
            target,
            structure,
            recursive,
            apply_fixes,
            watch,
        ),
        Some(Commands::Check {
            skill_dirs,
            format,
            target,
            no_validate,
            structure,
            recursive,
            apply_fixes,
        }) => check::run(
            skill_dirs,
            format,
            target,
            no_validate,
            structure,
            recursive,
            apply_fixes,
        ),
        Some(Commands::Properties { skill_dir }) => properties::run(skill_dir),
        Some(Commands::Prompt {
            skill_dirs,
            format,
            budget,
            output,
        }) => prompt::run(skill_dirs, format, budget, output),
        Some(Commands::Score { skill_dir, format }) => score::run(skill_dir, format),
        Some(Commands::New {
            purpose,
            name,
            dir,
            no_llm,
            interactive,
            minimal,
        }) => new::run(purpose, name, dir, no_llm, interactive, minimal),
        Some(Commands::Doc {
            skill_dirs,
            output,
            recursive,
        }) => doc::run(skill_dirs, output, recursive),
        Some(Commands::Probe {
            skill_dirs,
            query,
            format,
        }) => probe::run(skill_dirs, query, format),
        Some(Commands::Build {
            skill_dirs,
            output,
            name,
            validate,
        }) => build::run(skill_dirs, output, name, validate),
        Some(Commands::Test {
            skill_dirs,
            format,
            recursive,
            generate,
        }) => test::run(skill_dirs, format, recursive, generate),
        Some(Commands::Upgrade {
            skill_dir,
            apply,
            full,
            format,
        }) => upgrade::run(skill_dir, apply, full, format),
        Some(Commands::Format {
            skill_dirs,
            check,
            recursive,
        }) => format::run(skill_dirs, check, recursive),
        Some(Commands::ValidatePlugin { plugin_dir, format }) => {
            validate_plugin::run(plugin_dir, format)
        }
        Some(Commands::Init {
            dir,
            template,
            minimal,
        }) => init::run(dir, template, minimal),
        None => {
            eprintln!("Usage: aigent <command> [args]");
            eprintln!("Run `aigent --help` for details.");
            std::process::exit(1);
        }
    }
}

fn print_about() {
    println!(
        "aigent: Rust AI Agent Skills Tool\n\
         ├─ version:    {}\n\
         ├─ author:     {}\n\
         ├─ developer:  mailto:waclaw.kusnierczyk@gmail.com\n\
         ├─ source:     {}\n\
         └─ licence:    {} https://www.apache.org/licenses/LICENSE-2.0",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_REPOSITORY"),
        env!("CARGO_PKG_LICENSE"),
    );
}

/// If path points to a SKILL.md file, resolve to its parent directory.
fn resolve_skill_dir(path: &std::path::Path) -> PathBuf {
    if path.is_file() {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        path.to_path_buf()
    }
}

/// Resolve a list of input paths into skill directories, collecting discovery warnings.
///
/// When `recursive` is true, discovers skills under each path recursively.
/// File paths (e.g., `path/to/SKILL.md`) are resolved to their parent
/// directory before recursive discovery.
/// When false, treats each path as a direct skill directory (resolving
/// SKILL.md file paths to their parent).
fn resolve_dirs(
    paths: &[PathBuf],
    recursive: bool,
) -> (Vec<PathBuf>, Vec<aigent::DiscoveryWarning>) {
    let mut dirs = Vec::new();
    let mut warnings = Vec::new();
    for path in paths {
        if recursive {
            // If the user passes a SKILL.md file path, resolve to its parent
            // before running recursive discovery.
            let resolved = resolve_skill_dir(path);
            let (found, warns) = aigent::discover_skills_verbose(&resolved);
            dirs.extend(found);
            warnings.extend(warns);
        } else {
            dirs.push(resolve_skill_dir(path));
        }
    }
    (dirs, warnings)
}
