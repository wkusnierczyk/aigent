use std::path::PathBuf;

pub(crate) fn run(skill_dir: PathBuf) {
    let dir = super::resolve_skill_dir(&skill_dir);
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
