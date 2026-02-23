use std::path::PathBuf;

use aigent::diagnostics::Diagnostic;

pub(crate) fn run(plugin_dir: PathBuf, format: super::Format) {
    let mut all_diags: Vec<(String, Vec<Diagnostic>)> = Vec::new();

    // Validate manifest
    let manifest_path = plugin_dir.join("plugin.json");
    let manifest_diags = aigent::validate_manifest(&manifest_path);
    all_diags.push(("plugin.json".to_string(), manifest_diags));

    // Validate hooks if hooks.json exists
    let hooks_path = plugin_dir.join("hooks.json");
    if hooks_path.exists() {
        let hooks_diags = aigent::validate_hooks(&hooks_path);
        all_diags.push(("hooks.json".to_string(), hooks_diags));
    }

    // Validate agent files
    let agents_dir = plugin_dir.join("agents");
    if agents_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    let label = format!("agents/{}", path.file_name().unwrap().to_string_lossy());
                    let agent_diags = aigent::validate_agent(&path);
                    all_diags.push((label, agent_diags));
                }
            }
        }
    }

    // Validate command files
    let commands_dir = plugin_dir.join("commands");
    if commands_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&commands_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    let label = format!("commands/{}", path.file_name().unwrap().to_string_lossy());
                    let cmd_diags = aigent::validate_command(&path);
                    all_diags.push((label, cmd_diags));
                }
            }
        }
    }

    // Validate skill directories (each subdirectory under skills/)
    let skills_dir = plugin_dir.join("skills");
    if skills_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("SKILL.md").exists() {
                    let label = format!("skills/{}", path.file_name().unwrap().to_string_lossy());
                    let skill_diags = aigent::validate(&path);
                    all_diags.push((label, skill_diags));
                }
            }
        }
    }

    // Cross-component consistency checks (X-series)
    let cross_diags = aigent::validate_cross_component(&plugin_dir);
    if !cross_diags.is_empty() {
        all_diags.push(("<cross-component>".to_string(), cross_diags));
    }

    let has_errors = all_diags
        .iter()
        .any(|(_, d)| d.iter().any(|d| d.is_error()));

    match format {
        super::Format::Text => {
            let total_diags: usize = all_diags.iter().map(|(_, d)| d.len()).sum();
            for (label, diags) in &all_diags {
                if !diags.is_empty() {
                    eprintln!("{label}:");
                    for d in diags {
                        eprintln!("  {d}");
                    }
                }
            }
            if total_diags == 0 {
                eprintln!("Plugin validation passed.");
            }
        }
        super::Format::Json => {
            let entries: Vec<serde_json::Value> = all_diags
                .iter()
                .map(|(label, diags)| {
                    serde_json::json!({
                        "path": label,
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
