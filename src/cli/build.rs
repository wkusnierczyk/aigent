use std::path::PathBuf;

pub(crate) fn run(skill_dirs: Vec<PathBuf>, output: PathBuf, name: Option<String>, validate: bool) {
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
