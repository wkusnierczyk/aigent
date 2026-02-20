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
    /// Generate prompt from skill directories
    ToPrompt {
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
    /// Test a skill against a sample user query
    Test {
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
    /// Check a skill for upgrade opportunities
    Upgrade {
        /// Path to skill directory or SKILL.md file
        #[arg(name = "skill-dir")]
        skill_dir: PathBuf,
        /// Apply automatic upgrades
        #[arg(long)]
        apply: bool,
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
                    lint,
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
            let dirs = resolve_dirs(&skill_dirs, recursive);
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

                // Append lint results if requested.
                if lint {
                    if let Ok(props) = aigent::read_properties(dir) {
                        let body = aigent::read_body(dir);
                        diags.extend(aigent::lint(&props, &body));
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
                let entries = aigent::collect_skills(&skill_dirs_refs);
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
        Some(Commands::Lint { skill_dir, format }) => {
            let dir = resolve_skill_dir(&skill_dir);
            let diags = match aigent::read_properties(&dir) {
                Ok(props) => {
                    let body = aigent::read_body(&dir);
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
        Some(Commands::ToPrompt {
            skill_dirs,
            format,
            budget,
            output,
        }) => {
            let dirs: Vec<&std::path::Path> = skill_dirs.iter().map(|p| p.as_path()).collect();
            let prompt_format: aigent::prompt::PromptFormat = format.into();
            let content = aigent::prompt::to_prompt_format(&dirs, prompt_format);

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
                        let entries = aigent::prompt::collect_skills(&dirs);
                        eprint!("{}", aigent::prompt::format_budget(&entries));
                    }
                    std::process::exit(1);
                } else {
                    eprintln!("Unchanged {}", output_path.display());
                }
            } else {
                println!("{content}");
                if budget {
                    let entries = aigent::prompt::collect_skills(&dirs);
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
        Some(Commands::Build {
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
        Some(Commands::Doc {
            skill_dirs,
            output,
            recursive,
        }) => {
            let dirs = resolve_dirs(&skill_dirs, recursive);
            if dirs.is_empty() {
                if recursive {
                    eprintln!("No SKILL.md files found under the specified path(s).");
                } else {
                    eprintln!("Usage: aigent doc <skill-dir> [<skill-dir>...]");
                }
                std::process::exit(1);
            }

            let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();
            let entries = aigent::collect_skills(&dir_refs);
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
        Some(Commands::Test {
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
                    eprintln!("aigent test: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Upgrade {
            skill_dir,
            apply,
            format,
        }) => {
            let dir = resolve_skill_dir(&skill_dir);
            match run_upgrade(&dir, apply) {
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

/// Resolve a list of input paths into skill directories.
///
/// When `recursive` is true, discovers skills under each path recursively.
/// File paths (e.g., `path/to/SKILL.md`) are resolved to their parent
/// directory before recursive discovery.
/// When false, treats each path as a direct skill directory (resolving
/// SKILL.md file paths to their parent).
fn resolve_dirs(paths: &[PathBuf], recursive: bool) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for path in paths {
        if recursive {
            // If the user passes a SKILL.md file path, resolve to its parent
            // before running recursive discovery.
            let resolved = resolve_skill_dir(path);
            dirs.extend(aigent::discover_skills(&resolved));
        } else {
            dirs.push(resolve_skill_dir(path));
        }
    }
    dirs
}

/// Run watch mode: re-validate on filesystem changes (requires `watch` feature).
#[cfg(feature = "watch")]
fn run_watch_mode(
    skill_dirs: &[PathBuf],
    _format: Format,
    target: Target,
    lint: bool,
    structure: bool,
    recursive: bool,
    apply_fixes: bool,
) {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let dirs = resolve_dirs(skill_dirs, recursive);
    if dirs.is_empty() {
        eprintln!("No SKILL.md files found.");
        std::process::exit(1);
    }

    let target_val: aigent::diagnostics::ValidationTarget = target.into();

    // Run initial validation.
    run_validation_pass(&dirs, target_val, lint, structure, apply_fixes);

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
                let dirs = resolve_dirs(skill_dirs, recursive);
                run_validation_pass(&dirs, target_val, lint, structure, apply_fixes);

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
    lint: bool,
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

        if lint {
            if let Ok(props) = aigent::read_properties(dir) {
                let body = aigent::read_body(dir);
                diags.extend(aigent::lint(&props, &body));
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

/// Run upgrade analysis on a skill directory.
///
/// Checks for missing best-practice fields and returns a list of human-readable
/// suggestions. With `apply = true`, attempts to add missing optional fields.
fn run_upgrade(
    dir: &std::path::Path,
    apply: bool,
) -> std::result::Result<Vec<String>, aigent::AigentError> {
    let props = aigent::read_properties(dir)?;
    let mut suggestions = Vec::new();

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
    let body = aigent::read_body(dir);
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
                    let mut updated_yaml = String::new();
                    // Re-serialize frontmatter with additions.
                    // We simply append missing fields to existing frontmatter.
                    let existing_front = content
                        .lines()
                        .skip(1) // skip opening ---
                        .take_while(|l| *l != "---")
                        .collect::<Vec<_>>()
                        .join("\n");

                    updated_yaml.push_str(&existing_front);

                    if props.compatibility.is_none() && !raw_map.contains_key("compatibility") {
                        updated_yaml.push_str("\ncompatibility: claude-code");
                    }

                    if !has_version || !has_author {
                        if meta_block.is_none() {
                            // No metadata block at all — add the entire block.
                            updated_yaml.push_str("\nmetadata:");
                            if !has_version {
                                updated_yaml.push_str("\n  version: '0.1.0'");
                            }
                            if !has_author {
                                updated_yaml.push_str("\n  author: unknown");
                            }
                        } else {
                            // Partial metadata exists — append missing keys under it.
                            // Find the metadata: line and insert after it.
                            let lines: Vec<&str> = updated_yaml.lines().collect();
                            let mut new_yaml = String::new();
                            for line in &lines {
                                new_yaml.push_str(line);
                                new_yaml.push('\n');
                                if line.trim_start().starts_with("metadata:") {
                                    if !has_version {
                                        new_yaml.push_str("  version: '0.1.0'\n");
                                    }
                                    if !has_author {
                                        new_yaml.push_str("  author: unknown\n");
                                    }
                                }
                            }
                            // Remove trailing newline to match expected format.
                            updated_yaml = new_yaml.trim_end_matches('\n').to_string();
                        }
                    }

                    let new_content = format!("---\n{updated_yaml}\n---\n{body}");
                    if new_content != content {
                        std::fs::write(&path, new_content).unwrap_or_else(|e| {
                            eprintln!("aigent upgrade: failed to write {}: {e}", path.display());
                        });
                        eprintln!("Applied upgrades to {}", path.display());
                    }
                }
            }
        }
    }

    Ok(suggestions)
}
