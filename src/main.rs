use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use aigent::diagnostics::{Diagnostic, ValidationTarget};

#[derive(Parser)]
#[command(
    name = "aigent",
    version,
    about = "AI agent skill builder and validator"
)]
struct Cli {
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

#[derive(Subcommand)]
enum Commands {
    /// Validate skill directories
    Validate {
        /// Paths to skill directories or SKILL.md files
        skill_dirs: Vec<PathBuf>,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Validation target profile
        #[arg(long, value_enum, default_value_t = Target::Standard)]
        target: Target,
        /// Run semantic lint checks
        #[arg(long)]
        lint: bool,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
        /// Apply automatic fixes for fixable issues
        #[arg(long)]
        apply_fixes: bool,
    },
    /// Run semantic lint checks on a skill directory
    Lint {
        /// Path to skill directory or SKILL.md file
        skill_dir: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Read skill properties as JSON
    ReadProperties {
        /// Path to skill directory or SKILL.md file
        #[arg(name = "skill-dir")]
        skill_dir: PathBuf,
    },
    /// Generate XML prompt from skill directories
    ToPrompt {
        /// Paths to skill directories
        skill_dirs: Vec<PathBuf>,
    },
    /// Build a skill from a natural language description
    Build {
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
    },
    /// Initialize a skill directory with a template SKILL.md
    Init {
        /// Target directory
        dir: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.about {
        print_about();
        return;
    }

    match cli.command {
        Some(Commands::Validate {
            skill_dirs,
            format,
            target,
            lint,
            recursive,
            apply_fixes,
        }) => {
            // Resolve directories: expand --recursive, resolve file paths.
            let dirs = resolve_dirs(&skill_dirs, recursive);
            if dirs.is_empty() {
                eprintln!("Usage: aigent validate <skill-dir> [<skill-dir>...]");
                std::process::exit(1);
            }

            let mut all_diags: Vec<(PathBuf, Vec<Diagnostic>)> = Vec::new();
            let target_val: ValidationTarget = target.into();

            for dir in &dirs {
                let mut diags = aigent::validate_with_target(dir, target_val);

                // Apply fixes if requested.
                if apply_fixes {
                    match aigent::apply_fixes(dir, &diags) {
                        Ok(count) if count > 0 => {
                            eprintln!("Applied {count} fix(es) to {}", dir.display());
                            // Re-validate after fixes.
                            diags = aigent::validate_with_target(dir, target_val);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("warning: could not apply fixes to {}: {e}", dir.display());
                        }
                    }
                }

                // Append lint results if requested.
                if lint {
                    if let Ok(props) = aigent::read_properties(dir) {
                        let body = read_body(dir);
                        diags.extend(aigent::lint(&props, &body));
                    }
                }

                all_diags.push((dir.clone(), diags));
            }

            let has_errors = all_diags
                .iter()
                .any(|(_, d)| d.iter().any(|d| d.is_error()));

            match format {
                Format::Text => {
                    let multi = all_diags.len() > 1;
                    for (dir, diags) in &all_diags {
                        if multi && !diags.is_empty() {
                            eprintln!("{}:", dir.display());
                        }
                        for d in diags {
                            if multi {
                                eprintln!("  {d}");
                            } else {
                                eprintln!("{d}");
                            }
                        }
                    }
                    // Print summary for multi-dir.
                    if multi {
                        let total = all_diags.len();
                        let errors = all_diags
                            .iter()
                            .filter(|(_, d)| d.iter().any(|d| d.is_error()))
                            .count();
                        let warnings = all_diags
                            .iter()
                            .filter(|(_, d)| {
                                d.iter().any(|d| d.is_warning()) && !d.iter().any(|d| d.is_error())
                            })
                            .count();
                        let ok = total - errors - warnings;
                        eprintln!(
                            "\n{total} skills: {ok} ok, {errors} errors, {warnings} warnings"
                        );
                    }
                }
                Format::Json => {
                    // For single dir: flat array. For multi: array of objects.
                    if all_diags.len() == 1 {
                        let json = serde_json::to_string_pretty(&all_diags[0].1).unwrap();
                        println!("{json}");
                    } else {
                        let entries: Vec<serde_json::Value> = all_diags
                            .iter()
                            .map(|(dir, diags)| {
                                serde_json::json!({
                                    "path": dir.display().to_string(),
                                    "diagnostics": diags,
                                })
                            })
                            .collect();
                        let json = serde_json::to_string_pretty(&entries).unwrap();
                        println!("{json}");
                    }
                }
            }

            if has_errors {
                std::process::exit(1);
            }
        }
        Some(Commands::Lint { skill_dir, format }) => {
            let dir = resolve_skill_dir(&skill_dir);
            let diags = match aigent::read_properties(&dir) {
                Ok(props) => {
                    let body = read_body(&dir);
                    aigent::lint(&props, &body)
                }
                Err(e) => {
                    eprintln!("aigent lint: {e}");
                    std::process::exit(1);
                }
            };

            match format {
                Format::Text => {
                    for d in &diags {
                        eprintln!("{d}");
                    }
                }
                Format::Json => {
                    let json = serde_json::to_string_pretty(&diags).unwrap();
                    println!("{json}");
                }
            }

            // Lint never causes failure — all diagnostics are Info.
        }
        Some(Commands::ReadProperties { skill_dir }) => {
            let dir = resolve_skill_dir(&skill_dir);
            match aigent::read_properties(&dir) {
                Ok(props) => {
                    println!("{}", serde_json::to_string_pretty(&props).unwrap());
                }
                Err(e) => {
                    eprintln!("aigent read-properties: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::ToPrompt { skill_dirs }) => {
            let dirs: Vec<&std::path::Path> = skill_dirs.iter().map(|p| p.as_path()).collect();
            println!("{}", aigent::to_prompt(&dirs));
        }
        Some(Commands::Build {
            purpose,
            name,
            dir,
            no_llm,
        }) => {
            let spec = aigent::SkillSpec {
                purpose,
                name,
                output_dir: dir,
                no_llm,
                ..Default::default()
            };
            match aigent::build_skill(&spec) {
                Ok(result) => {
                    println!(
                        "Created skill '{}' at {}",
                        result.properties.name,
                        result.output_dir.display()
                    );
                }
                Err(e) => {
                    eprintln!("aigent build: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Init { dir }) => {
            let target = dir.unwrap_or_else(|| PathBuf::from("."));
            match aigent::init_skill(&target) {
                Ok(path) => {
                    println!("Created {}", path.display());
                }
                Err(e) => {
                    eprintln!("aigent init: {e}");
                    std::process::exit(1);
                }
            }
        }
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
         ├─ source:     {}\n\
         └─ license:    {}",
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

/// Resolve a list of input paths into skill directories.
///
/// When `recursive` is true, discovers skills under each path recursively.
/// When false, treats each path as a direct skill directory (resolving
/// SKILL.md file paths to their parent).
fn resolve_dirs(paths: &[PathBuf], recursive: bool) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for path in paths {
        if recursive {
            dirs.extend(aigent::discover_skills(path));
        } else {
            dirs.push(resolve_skill_dir(path));
        }
    }
    dirs
}

/// Read the SKILL.md body (post-frontmatter) for linting.
///
/// Returns an empty string if the file can't be read or parsed.
fn read_body(dir: &std::path::Path) -> String {
    let path = aigent::find_skill_md(dir);
    let path = match path {
        Some(p) => p,
        None => return String::new(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    match aigent::parse_frontmatter(&content) {
        Ok((_, body)) => body,
        Err(_) => String::new(),
    }
}
