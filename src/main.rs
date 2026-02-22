use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use aigent::builder::template::SkillTemplate;
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
enum Commands {
    /// Validate skill directories (spec conformance)
    Validate {
        /// Paths to skill directories or SKILL.md files
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
        /// Paths to skill directories or SKILL.md files
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
        /// Path to skill directory or SKILL.md file
        #[arg(name = "skill-dir")]
        skill_dir: PathBuf,
    },
    /// Generate prompt from skill directories
    #[command(alias = "to-prompt")]
    Prompt {
        /// Paths to skill directories
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
    },
    /// Score a skill against best-practices checklist
    Score {
        /// Path to skill directory or SKILL.md file
        #[arg(name = "skill-dir")]
        skill_dir: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Generate a markdown skill catalog
    Doc {
        /// Paths to skill directories
        skill_dirs: Vec<PathBuf>,
        /// Write output to file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Probe a skill's activation surface with a sample query
    Probe {
        /// Path to skill directory or SKILL.md file
        #[arg(name = "skill-dir")]
        skill_dir: PathBuf,
        /// Sample user query to test activation against
        #[arg(name = "query")]
        query: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Assemble skills into a Claude Code plugin
    Build {
        /// Paths to skill directories
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
        /// Paths to skill directories
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
        /// Path to skill directory or SKILL.md file
        #[arg(name = "skill-dir")]
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
    #[command(alias = "format")]
    Fmt {
        /// Paths to skill directories or SKILL.md files
        skill_dirs: Vec<PathBuf>,
        /// Check formatting without modifying files (exit 1 if unformatted)
        #[arg(long)]
        check: bool,
        /// Discover skills recursively
        #[arg(long)]
        recursive: bool,
    },
    /// Initialize a skill directory with a template SKILL.md
    Init {
        /// Target directory
        dir: Option<PathBuf>,
        /// Template variant for skill structure
        #[arg(long, value_enum, default_value_t = SkillTemplate::Minimal)]
        template: SkillTemplate,
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
            structure,
            recursive,
            apply_fixes,
            watch,
        }) => {
            // Watch mode: re-run validation on filesystem changes.
            #[cfg(feature = "watch")]
            if watch {
                run_watch_mode(
                    &skill_dirs,
                    format,
                    target,
                    structure,
                    recursive,
                    apply_fixes,
                );
                return;
            }
            #[cfg(not(feature = "watch"))]
            if watch {
                eprintln!("Watch mode requires the 'watch' feature. Rebuild with: cargo build --features watch");
                std::process::exit(1);
            }

            // Resolve directories: expand --recursive, resolve file paths.
            let (dirs, disc_warnings) = resolve_dirs(&skill_dirs, recursive);
            for w in &disc_warnings {
                eprintln!("warning: {}: {}", w.path.display(), w.message);
            }
            if dirs.is_empty() {
                if recursive {
                    eprintln!("No SKILL.md files found under the specified path(s).");
                } else {
                    eprintln!("Usage: aigent validate <skill-dir> [<skill-dir>...]");
                }
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

                // Append structure checks if requested.
                if structure {
                    diags.extend(aigent::validate_structure(dir));
                }

                all_diags.push((dir.clone(), diags));
            }

            // Run cross-skill conflict detection for multi-dir validation.
            let conflict_diags = if all_diags.len() > 1 {
                let skill_dirs_refs: Vec<&std::path::Path> =
                    dirs.iter().map(|p| p.as_path()).collect();
                let (entries, coll_warnings) = aigent::collect_skills_verbose(&skill_dirs_refs);
                for w in &coll_warnings {
                    eprintln!("warning: {}: {}", w.path.display(), w.message);
                }
                aigent::detect_conflicts(&entries)
            } else {
                vec![]
            };

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
                    // Print cross-skill conflict warnings.
                    if !conflict_diags.is_empty() {
                        eprintln!("\nCross-skill conflicts:");
                        for d in &conflict_diags {
                            eprintln!("  {d}");
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
                            "\n{total} skills: {ok} ok, {errors} errors, {warnings} warnings only"
                        );
                    }
                }
                Format::Json => {
                    // Always emit consistent array-of-objects format.
                    let mut entries: Vec<serde_json::Value> = all_diags
                        .iter()
                        .map(|(dir, diags)| {
                            serde_json::json!({
                                "path": dir.display().to_string(),
                                "diagnostics": diags,
                            })
                        })
                        .collect();
                    // Append cross-skill conflict diagnostics.
                    if !conflict_diags.is_empty() {
                        entries.push(serde_json::json!({
                            "path": "<cross-skill>",
                            "diagnostics": conflict_diags,
                        }));
                    }
                    let json = serde_json::to_string_pretty(&entries).unwrap();
                    println!("{json}");
                }
            }

            if has_errors {
                std::process::exit(1);
            }
        }
        Some(Commands::Check {
            skill_dirs,
            format,
            target,
            no_validate,
            structure,
            recursive,
            apply_fixes,
        }) => {
            let (dirs, disc_warnings) = resolve_dirs(&skill_dirs, recursive);
            for w in &disc_warnings {
                eprintln!("warning: {}: {}", w.path.display(), w.message);
            }
            if dirs.is_empty() {
                if recursive {
                    eprintln!("No SKILL.md files found under the specified path(s).");
                } else {
                    eprintln!("Usage: aigent check <skill-dir> [<skill-dir>...]");
                }
                std::process::exit(1);
            }

            let mut all_diags: Vec<(PathBuf, Vec<Diagnostic>)> = Vec::new();
            let target_val: ValidationTarget = target.into();

            for dir in &dirs {
                let mut diags = Vec::new();

                // Run spec conformance checks unless --no-validate.
                if !no_validate {
                    diags.extend(aigent::validate_with_target(dir, target_val));

                    // Apply fixes if requested.
                    if apply_fixes {
                        match aigent::apply_fixes(dir, &diags) {
                            Ok(count) if count > 0 => {
                                eprintln!("Applied {count} fix(es) to {}", dir.display());
                                diags = aigent::validate_with_target(dir, target_val);
                            }
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!(
                                    "warning: could not apply fixes to {}: {e}",
                                    dir.display()
                                );
                            }
                        }
                    }
                }

                // Always run semantic lint checks (the core of `check`).
                match aigent::read_properties(dir) {
                    Ok(props) => {
                        let body = aigent::read_body(dir).unwrap_or_default();
                        diags.extend(aigent::lint(&props, &body));
                    }
                    Err(e) => {
                        // Report parse failures as diagnostics rather than silently skipping.
                        diags.push(Diagnostic::new(
                            aigent::Severity::Error,
                            "E000",
                            format!("cannot read properties: {e}"),
                        ));
                    }
                }

                // Append structure checks if requested.
                if structure {
                    diags.extend(aigent::validate_structure(dir));
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
                            "\n{total} skills: {ok} ok, {errors} errors, {warnings} warnings only"
                        );
                    }
                }
                Format::Json => {
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

            if has_errors {
                std::process::exit(1);
            }
        }
        Some(Commands::Properties { skill_dir }) => {
            let dir = resolve_skill_dir(&skill_dir);
            match aigent::read_properties(&dir) {
                Ok(props) => {
                    println!("{}", serde_json::to_string_pretty(&props).unwrap());
                }
                Err(e) => {
                    eprintln!("aigent properties: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Prompt {
            skill_dirs,
            format,
            budget,
            output,
        }) => {
            let dirs: Vec<&std::path::Path> = skill_dirs.iter().map(|p| p.as_path()).collect();
            let prompt_format: aigent::prompt::PromptFormat = format.into();
            let (entries, warnings) = aigent::prompt::collect_skills_verbose(&dirs);
            for w in &warnings {
                eprintln!("warning: {}: {}", w.path.display(), w.message);
            }
            let content = aigent::prompt::format_entries(&entries, prompt_format);

            if let Some(output_path) = output {
                // Diff-aware file output: compare with existing, only write on change.
                let changed = if output_path.exists() {
                    let existing = std::fs::read_to_string(&output_path).unwrap_or_default();
                    existing != content
                } else {
                    true
                };

                if changed {
                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                            eprintln!(
                                "aigent to-prompt: failed to create directory {}: {e}",
                                parent.display()
                            );
                            std::process::exit(1);
                        });
                    }
                    std::fs::write(&output_path, &content).unwrap_or_else(|e| {
                        eprintln!(
                            "aigent to-prompt: failed to write {}: {e}",
                            output_path.display()
                        );
                        std::process::exit(1);
                    });
                    eprintln!("Updated {}", output_path.display());
                    if budget {
                        eprint!("{}", aigent::prompt::format_budget(&entries));
                    }
                    std::process::exit(1);
                } else {
                    eprintln!("Unchanged {}", output_path.display());
                }
            } else {
                println!("{content}");
                if budget {
                    eprint!("{}", aigent::prompt::format_budget(&entries));
                }
            }
        }
        Some(Commands::Score { skill_dir, format }) => {
            let dir = resolve_skill_dir(&skill_dir);
            let result = aigent::score(&dir);

            match format {
                Format::Text => {
                    eprint!("{}", aigent::scorer::format_text(&result));
                }
                Format::Json => {
                    let json = serde_json::to_string_pretty(&result).unwrap();
                    println!("{json}");
                }
            }

            // Exit with non-zero if score is below 100 (not perfect).
            if result.total < result.max {
                std::process::exit(1);
            }
        }
        Some(Commands::New {
            purpose,
            name,
            dir,
            no_llm,
            interactive,
        }) => {
            let spec = aigent::SkillSpec {
                purpose,
                name,
                output_dir: dir,
                no_llm,
                ..Default::default()
            };
            let result = if interactive {
                let mut stdin = std::io::stdin().lock();
                aigent::interactive_build(&spec, &mut stdin)
            } else {
                aigent::build_skill(&spec)
            };
            match result {
                Ok(result) => {
                    for w in &result.warnings {
                        eprintln!("warning: {w}");
                    }
                    println!(
                        "Created skill '{}' at {}",
                        result.properties.name,
                        result.output_dir.display()
                    );
                }
                Err(e) => {
                    eprintln!("aigent new: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Doc {
            skill_dirs,
            output,
            recursive,
        }) => {
            let (dirs, disc_warnings) = resolve_dirs(&skill_dirs, recursive);
            for w in &disc_warnings {
                eprintln!("warning: {}: {}", w.path.display(), w.message);
            }
            if dirs.is_empty() {
                if recursive {
                    eprintln!("No SKILL.md files found under the specified path(s).");
                } else {
                    eprintln!("Usage: aigent doc <skill-dir> [<skill-dir>...]");
                }
                std::process::exit(1);
            }

            let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();
            let (entries, warnings) = aigent::collect_skills_verbose(&dir_refs);
            for w in &warnings {
                eprintln!("warning: {}: {}", w.path.display(), w.message);
            }
            let content = format_doc_catalog(&entries);

            if let Some(output_path) = output {
                // Diff-aware output: only write on change.
                let changed = if output_path.exists() {
                    let existing = std::fs::read_to_string(&output_path).unwrap_or_default();
                    existing != content
                } else {
                    true
                };

                if changed {
                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                            eprintln!(
                                "aigent doc: failed to create directory {}: {e}",
                                parent.display()
                            );
                            std::process::exit(1);
                        });
                    }
                    std::fs::write(&output_path, &content).unwrap_or_else(|e| {
                        eprintln!("aigent doc: failed to write {}: {e}", output_path.display());
                        std::process::exit(1);
                    });
                    eprintln!("Updated {}", output_path.display());
                } else {
                    eprintln!("Unchanged {}", output_path.display());
                }
            } else {
                println!("{content}");
            }
        }
        Some(Commands::Probe {
            skill_dir,
            query,
            format,
        }) => {
            let dir = resolve_skill_dir(&skill_dir);
            match aigent::test_skill(&dir, &query) {
                Ok(result) => match format {
                    Format::Text => {
                        print!("{}", aigent::tester::format_test_result(&result));
                    }
                    Format::Json => {
                        let json = serde_json::json!({
                            "name": result.name,
                            "query": result.query,
                            "description": result.description,
                            "activation": format!("{:?}", result.query_match),
                            "estimated_tokens": result.estimated_tokens,
                            "validation_errors": result.diagnostics.iter()
                                .filter(|d| d.is_error()).count(),
                            "validation_warnings": result.diagnostics.iter()
                                .filter(|d| d.is_warning()).count(),
                            "structure_issues": result.structure_diagnostics.len(),
                        });
                        println!("{}", serde_json::to_string_pretty(&json).unwrap());
                    }
                },
                Err(e) => {
                    eprintln!("aigent probe: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Build {
            skill_dirs,
            output,
            name,
            validate,
        }) => {
            let dirs: Vec<&std::path::Path> = skill_dirs.iter().map(|p| p.as_path()).collect();
            let opts = aigent::AssembleOptions {
                output_dir: output,
                name,
                validate,
            };
            match aigent::assemble_plugin(&dirs, &opts) {
                Ok(result) => {
                    for w in &result.warnings {
                        eprintln!("warning: {}: {}", w.dir.display(), w.message);
                    }
                    println!(
                        "Assembled {} skill(s) into {}",
                        result.skills_count,
                        result.plugin_dir.display()
                    );
                }
                Err(e) => {
                    eprintln!("aigent build: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Test {
            skill_dirs,
            format,
            recursive,
            generate,
        }) => {
            let (dirs, disc_warnings) = resolve_dirs(&skill_dirs, recursive);
            for w in &disc_warnings {
                eprintln!("warning: {}: {}", w.path.display(), w.message);
            }
            if dirs.is_empty() {
                if recursive {
                    eprintln!("No SKILL.md files found under the specified path(s).");
                } else {
                    eprintln!("Usage: aigent test <skill-dir> [<skill-dir>...]");
                }
                std::process::exit(1);
            }

            if generate {
                let mut any_error = false;
                for dir in &dirs {
                    match aigent::generate_fixture(dir) {
                        Ok(yaml) => {
                            let fixture_path = dir.join("tests.yml");
                            if fixture_path.exists() {
                                eprintln!("Skipping {} — tests.yml already exists", dir.display());
                            } else {
                                std::fs::write(&fixture_path, &yaml).unwrap_or_else(|e| {
                                    eprintln!(
                                        "aigent test: failed to write {}: {e}",
                                        fixture_path.display()
                                    );
                                    std::process::exit(1);
                                });
                                eprintln!("Generated {}", fixture_path.display());
                            }
                        }
                        Err(e) => {
                            eprintln!("aigent test: {}: {e}", dir.display());
                            any_error = true;
                        }
                    }
                }
                if any_error {
                    std::process::exit(1);
                }
                return;
            }

            let mut total_passed = 0;
            let mut total_failed = 0;
            let mut any_error = false;

            for dir in &dirs {
                match aigent::run_test_suite(dir) {
                    Ok(result) => {
                        match format {
                            Format::Text => {
                                if dirs.len() > 1 {
                                    eprintln!("{}:", dir.display());
                                }
                                eprint!("{}", aigent::format_test_suite(&result));
                            }
                            Format::Json => {
                                let json = serde_json::to_string_pretty(&result).unwrap();
                                println!("{json}");
                            }
                        }
                        total_passed += result.passed;
                        total_failed += result.failed;
                    }
                    Err(e) => {
                        eprintln!("aigent test: {}: {e}", dir.display());
                        any_error = true;
                    }
                }
            }

            if dirs.len() > 1 {
                eprintln!(
                    "\nTotal: {total_passed} passed, {total_failed} failed, {} total",
                    total_passed + total_failed
                );
            }

            if total_failed > 0 || any_error {
                std::process::exit(1);
            }
        }
        Some(Commands::Upgrade {
            skill_dir,
            apply,
            full,
            format,
        }) => {
            let dir = resolve_skill_dir(&skill_dir);
            match run_upgrade(&dir, apply, full) {
                Ok(suggestions) => {
                    if suggestions.is_empty() {
                        eprintln!("No upgrade suggestions — skill follows current best practices.");
                    } else {
                        match format {
                            Format::Text => {
                                for s in &suggestions {
                                    eprintln!("{s}");
                                }
                                if !apply {
                                    eprintln!(
                                        "\nRun with --apply to apply {} suggestion(s).",
                                        suggestions.len()
                                    );
                                }
                            }
                            Format::Json => {
                                let json = serde_json::json!({
                                    "suggestions": suggestions,
                                    "applied": apply,
                                });
                                println!("{}", serde_json::to_string_pretty(&json).unwrap());
                            }
                        }
                        if !apply {
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("aigent upgrade: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Fmt {
            skill_dirs,
            check,
            recursive,
        }) => {
            let (dirs, disc_warnings) = resolve_dirs(&skill_dirs, recursive);
            for w in &disc_warnings {
                eprintln!("warning: {}: {}", w.path.display(), w.message);
            }
            if dirs.is_empty() {
                if recursive {
                    eprintln!("No SKILL.md files found under the specified path(s).");
                } else {
                    eprintln!("Usage: aigent fmt <skill-dir> [<skill-dir>...]");
                }
                std::process::exit(1);
            }

            let mut any_changed = false;
            let mut any_error = false;
            for dir in &dirs {
                match aigent::format_skill(dir) {
                    Ok(result) => {
                        if result.changed {
                            any_changed = true;
                            if check {
                                eprintln!("Would reformat: {}", dir.display());
                                let diff = aigent::diff_skill(&result, &dir.display().to_string());
                                eprint!("{diff}");
                            } else {
                                let path = aigent::find_skill_md(dir).unwrap();
                                std::fs::write(&path, &result.content).unwrap_or_else(|e| {
                                    eprintln!(
                                        "aigent fmt: failed to write {}: {e}",
                                        path.display()
                                    );
                                    std::process::exit(1);
                                });
                                eprintln!("Formatted {}", dir.display());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("aigent fmt: {}: {e}", dir.display());
                        any_error = true;
                    }
                }
            }

            if any_error || (check && any_changed) {
                std::process::exit(1);
            }
        }
        Some(Commands::Init { dir, template }) => {
            let target = dir.unwrap_or_else(|| PathBuf::from("."));
            match aigent::init_skill(&target, template) {
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
         ├─ developer:  mailto:waclaw.kusnierczyk@gmail.com\n\
         ├─ source:     {}\n\
         └─ licence:    {} https://opensource.org/licenses/{}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_REPOSITORY"),
        env!("CARGO_PKG_LICENSE"),
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

/// Run watch mode: re-validate on filesystem changes (requires `watch` feature).
#[cfg(feature = "watch")]
fn run_watch_mode(
    skill_dirs: &[PathBuf],
    _format: Format,
    target: Target,
    structure: bool,
    recursive: bool,
    apply_fixes: bool,
) {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let (dirs, disc_warnings) = resolve_dirs(skill_dirs, recursive);
    for w in &disc_warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    if dirs.is_empty() {
        eprintln!("No SKILL.md files found.");
        std::process::exit(1);
    }

    let target_val: aigent::diagnostics::ValidationTarget = target.into();

    // Run initial validation.
    run_validation_pass(&dirs, target_val, structure, apply_fixes);

    // Set up file watcher.
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap_or_else(|e| {
        eprintln!("aigent watch: failed to create watcher: {e}");
        std::process::exit(1);
    });

    // Watch all parent directories of skill dirs.
    let mut watch_paths: Vec<PathBuf> = Vec::new();
    for dir in &dirs {
        let watch_dir = if recursive {
            dir.parent().unwrap_or(dir).to_path_buf()
        } else {
            dir.clone()
        };
        if !watch_paths.contains(&watch_dir) {
            watch_paths.push(watch_dir);
        }
    }
    for path in &watch_paths {
        if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
            eprintln!("aigent watch: failed to watch {}: {e}", path.display());
        }
    }

    eprintln!("Watching for changes... (press Ctrl+C to stop)");

    let debounce = Duration::from_millis(500);
    let mut last_run = Instant::now();

    loop {
        match rx.recv() {
            Ok(_event) => {
                // Debounce: skip if we ran too recently.
                if last_run.elapsed() < debounce {
                    // Drain pending events.
                    while rx.try_recv().is_ok() {}
                    continue;
                }

                // Clear terminal.
                eprint!("\x1b[2J\x1b[H");

                // Re-resolve dirs in case new skills appeared.
                let (dirs, disc_warnings) = resolve_dirs(skill_dirs, recursive);
                for w in &disc_warnings {
                    eprintln!("warning: {}: {}", w.path.display(), w.message);
                }
                run_validation_pass(&dirs, target_val, structure, apply_fixes);

                last_run = Instant::now();

                // Drain any queued events during validation.
                while rx.try_recv().is_ok() {}
            }
            Err(e) => {
                eprintln!("aigent watch: watcher error: {e}");
                break;
            }
        }
    }
}

/// Run a single validation pass (used by watch mode).
#[cfg(feature = "watch")]
fn run_validation_pass(
    dirs: &[PathBuf],
    target: aigent::diagnostics::ValidationTarget,
    structure: bool,
    apply_fixes: bool,
) {
    let mut total_errors = 0;
    let mut total_warnings = 0;

    for dir in dirs {
        let mut diags = aigent::validate_with_target(dir, target);

        if apply_fixes {
            if let Ok(count) = aigent::apply_fixes(dir, &diags) {
                if count > 0 {
                    eprintln!("Applied {count} fix(es) to {}", dir.display());
                    diags = aigent::validate_with_target(dir, target);
                }
            }
        }

        if structure {
            diags.extend(aigent::validate_structure(dir));
        }

        let has_errors = diags.iter().any(|d| d.is_error());
        let has_warnings = diags.iter().any(|d| d.is_warning());

        if has_errors {
            total_errors += 1;
        } else if has_warnings {
            total_warnings += 1;
        }

        if !diags.is_empty() {
            if dirs.len() > 1 {
                eprintln!("{}:", dir.display());
            }
            for d in &diags {
                if dirs.len() > 1 {
                    eprintln!("  {d}");
                } else {
                    eprintln!("{d}");
                }
            }
        }
    }

    let total = dirs.len();
    let ok = total - total_errors - total_warnings;
    eprintln!("\n{total} skills: {ok} ok, {total_errors} errors, {total_warnings} warnings only");
}

/// Format a skill catalog as markdown documentation.
///
/// Generates a markdown document listing all skills sorted alphabetically,
/// with name, description, and location. Missing fields are omitted.
fn format_doc_catalog(entries: &[aigent::SkillEntry]) -> String {
    let mut out = String::from("# Skill Catalog\n");

    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    for entry in sorted {
        out.push_str(&format!("\n## {}\n", entry.name));
        out.push_str(&format!("> {}\n", entry.description));

        // Read full properties for optional fields.
        // entry.location is a file path to SKILL.md; read_properties expects the parent directory.
        let loc_path = std::path::Path::new(&entry.location);
        let skill_dir = loc_path.parent().unwrap_or(loc_path);
        if let Ok(props) = aigent::read_properties(skill_dir) {
            if let Some(compat) = &props.compatibility {
                out.push_str(&format!("\n**Compatibility**: {compat}\n"));
            }
            if let Some(license) = &props.license {
                out.push_str(&format!("**License**: {license}\n"));
            }
        }

        out.push_str(&format!("**Location**: `{}`\n", entry.location));
        out.push_str("\n---\n");
    }

    out
}

/// Extract frontmatter lines from SKILL.md content (between the `---` delimiters).
///
/// Returns the lines without the delimiters.
fn extract_frontmatter_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .skip(1) // skip opening ---
        .take_while(|l| l.trim_end() != "---")
        .map(|l| l.to_string())
        .collect()
}

/// Detect the indentation style used in frontmatter lines.
///
/// Scans for the first indented line and returns its leading spaces.
/// Defaults to "  " (2 spaces) if no indented lines are found.
/// Note: YAML forbids tabs for indentation, so only spaces are considered.
fn detect_indent(lines: &[String]) -> String {
    for line in lines {
        if line.starts_with(' ') {
            let indent: String = line.chars().take_while(|c| *c == ' ').collect();
            return indent;
        }
    }
    "  ".to_string() // default: 2 spaces
}

/// Find the insertion position for new metadata keys.
///
/// Locates the `metadata:` line (not in a comment) and returns the index
/// after the last indented child key beneath it.
fn find_metadata_insert_position(lines: &[String]) -> usize {
    let mut meta_line_idx = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // Skip comment lines.
        if trimmed.starts_with('#') {
            continue;
        }
        // Match `metadata:` as a top-level key (not indented).
        if trimmed.starts_with("metadata:") && !line.starts_with(' ') {
            meta_line_idx = Some(i);
            break;
        }
    }

    if let Some(idx) = meta_line_idx {
        // Walk forward from metadata: to find the last indented child line.
        let mut last_child = idx;
        for (i, line) in lines.iter().enumerate().skip(idx + 1) {
            if line.is_empty() || line.starts_with(' ') {
                // Still inside the metadata block (indented or blank).
                if !line.trim().is_empty() {
                    last_child = i;
                }
            } else {
                // Hit a non-indented line — end of metadata block.
                break;
            }
        }
        last_child + 1
    } else {
        // Fallback: insert at the end.
        lines.len()
    }
}

/// Run upgrade analysis on a skill directory.
///
/// Checks for missing best-practice fields and returns a list of human-readable
/// suggestions. With `apply = true`, attempts to add missing optional fields.
/// With `full = true`, also runs validate + lint first (and applies fixes if
/// `apply` is also true).
fn run_upgrade(
    dir: &std::path::Path,
    apply: bool,
    full: bool,
) -> std::result::Result<Vec<String>, aigent::AigentError> {
    let mut suggestions = Vec::new();

    // Full mode: run validate + lint before upgrade analysis.
    if full {
        let mut diags = aigent::validate(dir);
        if let Ok(props) = aigent::read_properties(dir) {
            let body = aigent::read_body(dir).unwrap_or_default();
            diags.extend(aigent::lint(&props, &body));
        }

        if apply {
            let fixable: Vec<_> = diags
                .iter()
                .filter(|d| d.suggestion.is_some())
                .cloned()
                .collect();
            if !fixable.is_empty() {
                let fix_count = aigent::apply_fixes(dir, &fixable)?;
                if fix_count > 0 {
                    suggestions.push(format!(
                        "[full] Applied {fix_count} validation/lint fix(es)"
                    ));
                }
                // Re-run diagnostics after fixes to get fresh state.
                diags = aigent::validate(dir);
                if let Ok(props) = aigent::read_properties(dir) {
                    let body = aigent::read_body(dir).unwrap_or_default();
                    diags.extend(aigent::lint(&props, &body));
                }
            }
        }

        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        if !errors.is_empty() || !warnings.is_empty() {
            for d in &errors {
                suggestions.push(format!("[full] error: {d}"));
            }
            for d in &warnings {
                suggestions.push(format!("[full] warning: {d}"));
            }
        }
    }

    let props = aigent::read_properties(dir)?;

    // Check for missing compatibility field.
    if props.compatibility.is_none() {
        suggestions.push(
            "Missing 'compatibility' field — recommended for multi-platform skills.".to_string(),
        );
    }

    // The parser stores the YAML `metadata:` block as a key named "metadata"
    // inside the extra-fields HashMap. So metadata.version becomes
    // props.metadata["metadata"]["version"].
    let meta_block = props
        .metadata
        .as_ref()
        .and_then(|m| m.get("metadata"))
        .and_then(|v| {
            if let serde_yaml_ng::Value::Mapping(map) = v {
                Some(map.clone())
            } else {
                None
            }
        });

    // Check for missing metadata.version.
    let has_version = meta_block
        .as_ref()
        .and_then(|m| m.get(serde_yaml_ng::Value::String("version".to_string())))
        .is_some();
    if !has_version {
        suggestions.push(
            "Missing 'metadata.version' — recommended for tracking skill versions.".to_string(),
        );
    }

    // Check for missing metadata.author.
    let has_author = meta_block
        .as_ref()
        .and_then(|m| m.get(serde_yaml_ng::Value::String("author".to_string())))
        .is_some();
    if !has_author {
        suggestions.push("Missing 'metadata.author' — recommended for attribution.".to_string());
    }

    // Check for missing trigger phrase in description.
    let desc_lower = props.description.to_lowercase();
    if !desc_lower.contains("use when") && !desc_lower.contains("use this when") {
        suggestions.push(
            "Description lacks 'Use when...' trigger phrase — helps Claude activate the skill."
                .to_string(),
        );
    }

    // Check body length.
    let body = aigent::read_body(dir)?;
    let line_count = body.lines().count();
    if line_count > 500 {
        suggestions.push(format!(
            "Body is {line_count} lines — consider splitting into reference files (recommended < 500)."
        ));
    }

    // Apply upgrades if requested.
    if apply && !suggestions.is_empty() {
        if let Some(path) = aigent::find_skill_md(dir) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok((raw_map, body)) = aigent::parse_frontmatter(&content) {
                    let front_lines = extract_frontmatter_lines(&content);
                    let mut updated_lines = front_lines.clone();

                    // Append compatibility if missing.
                    if props.compatibility.is_none() && !raw_map.contains_key("compatibility") {
                        updated_lines.push("compatibility: claude-code".to_string());
                    }

                    // Insert metadata keys.
                    if !has_version || !has_author {
                        if meta_block.is_none() {
                            // No metadata block — append entire block.
                            updated_lines.push("metadata:".to_string());
                            let indent = detect_indent(&front_lines);
                            if !has_version {
                                updated_lines.push(format!("{indent}version: '0.1.0'"));
                            }
                            if !has_author {
                                updated_lines.push(format!("{indent}author: unknown"));
                            }
                        } else {
                            // Partial metadata — find the metadata: line and insert after
                            // the last existing child key under it.
                            let indent = detect_indent(&front_lines);
                            let insert_pos = find_metadata_insert_position(&updated_lines);
                            let mut to_insert = Vec::new();
                            if !has_version {
                                to_insert.push(format!("{indent}version: '0.1.0'"));
                            }
                            if !has_author {
                                to_insert.push(format!("{indent}author: unknown"));
                            }
                            // Insert after the last metadata child, in reverse to maintain order.
                            for (i, line) in to_insert.into_iter().enumerate() {
                                updated_lines.insert(insert_pos + i, line);
                            }
                        }
                    }

                    let updated_yaml = updated_lines.join("\n");
                    let new_content = format!("---\n{updated_yaml}\n---\n{body}");
                    if new_content != content {
                        std::fs::write(&path, &new_content)?;
                        eprintln!("Applied upgrades to {}", path.display());
                    }
                }
            }
        }
    }

    Ok(suggestions)
}
