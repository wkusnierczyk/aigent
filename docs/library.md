# Library usage

> Back to [README](../README.md)

```rust
use std::path::Path;

// Validate a skill directory
let errors = aigent::validate(Path::new("my-skill"));

// Read skill properties
let props = aigent::read_properties(Path::new("my-skill")).unwrap();

// Generate prompt XML
let xml = aigent::to_prompt(&[Path::new("skill-a"), Path::new("skill-b")]);

// Format a SKILL.md
let result = aigent::format_skill(Path::new("my-skill")).unwrap();

// Assemble skills into a plugin
let opts = aigent::AssembleOptions {
    output_dir: std::path::PathBuf::from("./dist"),
    name: None,
    validate: true,
};
let result = aigent::assemble_plugin(
    &[Path::new("skill-a"), Path::new("skill-b")], &opts,
).unwrap();

// Build a skill
let spec = aigent::SkillSpec {
    purpose: "Process PDF files".to_string(),
    no_llm: true,
    ..Default::default()
};
let result = aigent::build_skill(&spec).unwrap();

// Validate a plugin directory (manifest, hooks, agents, commands, skills)
let manifest_diags = aigent::validate_manifest(Path::new("my-plugin/plugin.json"));
let hooks_diags = aigent::validate_hooks(Path::new("my-plugin/hooks.json"));
let cross_diags = aigent::validate_cross_component(Path::new("my-plugin"));
```
