use std::path::PathBuf;

use clap::{Parser, Subcommand};

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

#[derive(Subcommand)]
enum Commands {
    /// Validate a skill directory
    Validate {
        /// Path to skill directory or SKILL.md file
        skill_dir: PathBuf,
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
        Some(Commands::Validate { skill_dir }) => {
            let dir = resolve_skill_dir(&skill_dir);
            let messages = aigent::validate(&dir);
            let has_errors = messages.iter().any(|m| !m.starts_with("warning: "));
            for m in &messages {
                eprintln!("{m}");
            }
            if has_errors {
                std::process::exit(1);
            }
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
        Some(Commands::Build { .. }) => {
            eprintln!("aigent build: not yet implemented");
            std::process::exit(1);
        }
        Some(Commands::Init { .. }) => {
            eprintln!("aigent init: not yet implemented");
            std::process::exit(1);
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
         ├─ authors:    {}\n\
         ├─ source:     {}\n\
         └─ license:    {} https://opensource.org/licenses/{}",
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
