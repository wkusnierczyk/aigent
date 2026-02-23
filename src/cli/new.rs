use std::path::PathBuf;

pub(crate) fn run(
    purpose: String,
    name: Option<String>,
    dir: Option<PathBuf>,
    no_llm: bool,
    interactive: bool,
    minimal: bool,
) {
    let spec = aigent::SkillSpec {
        purpose,
        name,
        output_dir: dir,
        no_llm,
        minimal,
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
