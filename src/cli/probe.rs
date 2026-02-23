use std::path::PathBuf;

pub(crate) fn run(skill_dirs: Vec<PathBuf>, query: String, format: super::Format) {
    let dirs: Vec<PathBuf> = skill_dirs
        .iter()
        .map(|p| super::resolve_skill_dir(p))
        .collect();
    let mut results = Vec::new();
    let mut had_errors = false;
    for dir in &dirs {
        match aigent::test_skill(dir, &query) {
            Ok(result) => results.push(result),
            Err(e) => {
                eprintln!("aigent probe: {}: {e}", dir.display());
                had_errors = true;
            }
        }
    }
    // Sort by score descending (best match first)
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    match format {
        super::Format::Text => {
            for (i, result) in results.iter().enumerate() {
                if i > 0 {
                    println!();
                }
                print!("{}", aigent::tester::format_test_result(result));
            }
        }
        super::Format::Json => {
            let json: Vec<_> = results
                .iter()
                .map(|result| {
                    serde_json::json!({
                        "name": result.name,
                        "query": result.query,
                        "description": result.description,
                        "activation": format!("{:?}", result.query_match),
                        "score": result.score,
                        "estimated_tokens": result.estimated_tokens,
                        "validation_errors": result.diagnostics.iter()
                            .filter(|d| d.is_error()).count(),
                        "validation_warnings": result.diagnostics.iter()
                            .filter(|d| d.is_warning()).count(),
                        "structure_issues": result.structure_diagnostics.len(),
                    })
                })
                .collect();
            if json.len() == 1 {
                println!("{}", serde_json::to_string_pretty(&json[0]).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            }
        }
    }
    if had_errors && results.is_empty() {
        std::process::exit(1);
    }
}
