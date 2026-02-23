use std::path::PathBuf;

pub(crate) fn run(skill_dir: PathBuf, format: super::Format) {
    let dir = super::resolve_skill_dir(&skill_dir);
    let result = aigent::score(&dir);

    match format {
        super::Format::Text => {
            eprint!("{}", aigent::scorer::format_text(&result));
        }
        super::Format::Json => {
            let json = serde_json::to_string_pretty(&result).unwrap();
            println!("{json}");
        }
    }

    // Exit with non-zero if score is below 100 (not perfect).
    if result.total < result.max {
        std::process::exit(1);
    }
}
