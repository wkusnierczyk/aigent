use std::path::PathBuf;

use aigent::builder::template::SkillTemplate;

pub(crate) fn run(dir: Option<PathBuf>, template: SkillTemplate, minimal: bool) {
    let target = dir.unwrap_or_else(|| PathBuf::from("."));
    match aigent::init_skill(&target, template, minimal) {
        Ok(path) => {
            println!("Created {}", path.display());
        }
        Err(e) => {
            eprintln!("aigent init: {e}");
            std::process::exit(1);
        }
    }
}
