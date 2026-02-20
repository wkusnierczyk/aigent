use std::collections::HashMap;

use clap::ValueEnum;

use super::util::to_title_case;

/// Skill template variant for `init` and `build`.
///
/// Each variant generates a different directory structure and SKILL.md
/// content pattern, following the
/// [Anthropic skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices).
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum SkillTemplate {
    /// Basic SKILL.md only (default)
    #[default]
    Minimal,
    /// SKILL.md + REFERENCE.md + EXAMPLES.md
    ReferenceGuide,
    /// SKILL.md + reference/domain.md
    DomainSpecific,
    /// SKILL.md with checklist/workflow pattern
    Workflow,
    /// SKILL.md + scripts/run.sh
    CodeSkill,
    /// SKILL.md with Claude Code extension fields
    ClaudeCode,
}

/// Generate template files for a given template variant and skill name.
///
/// Returns a map of relative path → content. The skill name should be
/// kebab-case. Every template includes at least a `SKILL.md` entry.
#[must_use]
pub fn template_files(template: SkillTemplate, dir_name: &str) -> HashMap<String, String> {
    let name = to_kebab_case(dir_name);
    let name = if name.is_empty() {
        "my-skill".to_string()
    } else {
        name
    };
    let title = to_title_case(&name);

    let mut files = HashMap::new();

    match template {
        SkillTemplate::Minimal => {
            files.insert("SKILL.md".to_string(), minimal_skill_md(&name, &title));
        }
        SkillTemplate::ReferenceGuide => {
            files.insert(
                "SKILL.md".to_string(),
                reference_guide_skill_md(&name, &title),
            );
            files.insert("REFERENCE.md".to_string(), reference_md(&title));
            files.insert("EXAMPLES.md".to_string(), examples_md(&title));
        }
        SkillTemplate::DomainSpecific => {
            files.insert(
                "SKILL.md".to_string(),
                domain_specific_skill_md(&name, &title),
            );
            files.insert(
                "reference/domain.md".to_string(),
                domain_reference_md(&title),
            );
        }
        SkillTemplate::Workflow => {
            files.insert("SKILL.md".to_string(), workflow_skill_md(&name, &title));
        }
        SkillTemplate::CodeSkill => {
            files.insert("SKILL.md".to_string(), code_skill_md(&name, &title));
            files.insert("scripts/run.sh".to_string(), run_script(&name));
        }
        SkillTemplate::ClaudeCode => {
            files.insert("SKILL.md".to_string(), claude_code_skill_md(&name, &title));
        }
    }

    files
}

/// Generate a SKILL.md template for `init` (backward-compatible wrapper).
///
/// The `dir_name` is used as the `name` field (kebab-cased) and the heading
/// (title-cased). Uses the `Minimal` template.
#[must_use]
pub fn skill_template(dir_name: &str) -> String {
    let files = template_files(SkillTemplate::Minimal, dir_name);
    files.get("SKILL.md").cloned().unwrap_or_default()
}

// ── Template content generators ────────────────────────────────────────

fn minimal_skill_md(name: &str, title: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         description: Describe what this skill does and when to use it\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Quick start\n\
         \n\
         [Add quick start instructions here]\n\
         \n\
         ## Usage\n\
         \n\
         [Add detailed usage instructions here]\n"
    )
}

fn reference_guide_skill_md(name: &str, title: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         description: Describe what this skill does and when to use it\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Quick start\n\
         \n\
         [Add quick start instructions here]\n\
         \n\
         ## Usage\n\
         \n\
         [Add detailed usage instructions here]\n\
         \n\
         ## Reference\n\
         \n\
         See [REFERENCE.md](REFERENCE.md) for detailed reference documentation.\n\
         \n\
         ## Examples\n\
         \n\
         See [EXAMPLES.md](EXAMPLES.md) for usage examples.\n"
    )
}

fn reference_md(title: &str) -> String {
    format!(
        "# {title} Reference\n\
         \n\
         ## API\n\
         \n\
         [Document the API or interface here]\n\
         \n\
         ## Configuration\n\
         \n\
         [Document configuration options here]\n\
         \n\
         ## Options\n\
         \n\
         [Document available options here]\n"
    )
}

fn examples_md(title: &str) -> String {
    format!(
        "# {title} Examples\n\
         \n\
         ## Basic usage\n\
         \n\
         ```\n\
         [Add a basic usage example here]\n\
         ```\n\
         \n\
         ## Advanced usage\n\
         \n\
         ```\n\
         [Add an advanced usage example here]\n\
         ```\n"
    )
}

fn domain_specific_skill_md(name: &str, title: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         description: Describe what this skill does and when to use it\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Quick start\n\
         \n\
         [Add quick start instructions here]\n\
         \n\
         ## Domain knowledge\n\
         \n\
         See [reference/domain.md](reference/domain.md) for domain-specific reference.\n\
         \n\
         ## Usage\n\
         \n\
         [Add detailed usage instructions here]\n"
    )
}

fn domain_reference_md(title: &str) -> String {
    format!(
        "# {title} Domain Reference\n\
         \n\
         ## Terminology\n\
         \n\
         [Define domain-specific terms here]\n\
         \n\
         ## Rules\n\
         \n\
         [Document domain rules and constraints here]\n\
         \n\
         ## Patterns\n\
         \n\
         [Document common patterns here]\n"
    )
}

fn workflow_skill_md(name: &str, title: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         description: Describe what this skill does and when to use it\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Workflow\n\
         \n\
         Follow these steps in order:\n\
         \n\
         1. [ ] **Step 1**: [Describe first step]\n\
         2. [ ] **Step 2**: [Describe second step]\n\
         3. [ ] **Step 3**: [Describe third step]\n\
         \n\
         ## Checklist\n\
         \n\
         Before completing, verify:\n\
         \n\
         - [ ] [First verification item]\n\
         - [ ] [Second verification item]\n\
         - [ ] [Third verification item]\n"
    )
}

fn code_skill_md(name: &str, title: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         description: Describe what this skill does and when to use it\n\
         allowed-tools: Bash(./scripts/run.sh *)\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Quick start\n\
         \n\
         Run the script:\n\
         \n\
         ```bash\n\
         ./scripts/run.sh [arguments]\n\
         ```\n\
         \n\
         ## Usage\n\
         \n\
         [Add detailed usage instructions here]\n"
    )
}

fn run_script(name: &str) -> String {
    format!(
        "#!/usr/bin/env bash\n\
         set -euo pipefail\n\
         \n\
         # {name} — main script\n\
         # Usage: ./scripts/run.sh [arguments]\n\
         \n\
         main() {{\n\
         \x20\x20\x20\x20echo \"{name}: not yet implemented\"\n\
         \x20\x20\x20\x20exit 1\n\
         }}\n\
         \n\
         main \"$@\"\n"
    )
}

fn claude_code_skill_md(name: &str, title: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         description: Describe what this skill does and when to use it\n\
         allowed-tools: Bash(*), Read, Write, Glob\n\
         user-invocable: true\n\
         argument-hint: \"[arguments]\"\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Quick start\n\
         \n\
         [Add quick start instructions here]\n\
         \n\
         ## Usage\n\
         \n\
         [Add detailed usage instructions here]\n"
    )
}

// ── Utility ────────────────────────────────────────────────────────────

/// Convert a string to kebab-case: lowercase, replace non-alphanumeric with
/// hyphens, collapse consecutive hyphens, trim leading/trailing hyphens.
fn to_kebab_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut result = String::with_capacity(lower.len());
    let mut prev_hyphen = false;

    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            prev_hyphen = false;
        } else if !prev_hyphen && !result.is_empty() {
            result.push('-');
            prev_hyphen = true;
        }
    }

    result.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_template_matches_legacy_output() {
        let legacy = skill_template("my-skill");
        let files = template_files(SkillTemplate::Minimal, "my-skill");
        assert_eq!(files.get("SKILL.md").unwrap(), &legacy);
    }

    #[test]
    fn minimal_template_produces_only_skill_md() {
        let files = template_files(SkillTemplate::Minimal, "test-skill");
        assert_eq!(files.len(), 1);
        assert!(files.contains_key("SKILL.md"));
    }

    #[test]
    fn reference_guide_template_produces_three_files() {
        let files = template_files(SkillTemplate::ReferenceGuide, "test-skill");
        assert_eq!(files.len(), 3);
        assert!(files.contains_key("SKILL.md"));
        assert!(files.contains_key("REFERENCE.md"));
        assert!(files.contains_key("EXAMPLES.md"));
    }

    #[test]
    fn domain_specific_template_produces_two_files() {
        let files = template_files(SkillTemplate::DomainSpecific, "test-skill");
        assert_eq!(files.len(), 2);
        assert!(files.contains_key("SKILL.md"));
        assert!(files.contains_key("reference/domain.md"));
    }

    #[test]
    fn workflow_template_produces_only_skill_md() {
        let files = template_files(SkillTemplate::Workflow, "test-skill");
        assert_eq!(files.len(), 1);
        assert!(files.contains_key("SKILL.md"));
        let content = files.get("SKILL.md").unwrap();
        assert!(
            content.contains("[ ]"),
            "workflow should have checklist items"
        );
    }

    #[test]
    fn code_skill_template_produces_two_files() {
        let files = template_files(SkillTemplate::CodeSkill, "test-skill");
        assert_eq!(files.len(), 2);
        assert!(files.contains_key("SKILL.md"));
        assert!(files.contains_key("scripts/run.sh"));
    }

    #[test]
    fn code_skill_script_has_shebang() {
        let files = template_files(SkillTemplate::CodeSkill, "test-skill");
        let script = files.get("scripts/run.sh").unwrap();
        assert!(script.starts_with("#!/usr/bin/env bash"));
    }

    #[test]
    fn code_skill_script_has_strict_mode() {
        let files = template_files(SkillTemplate::CodeSkill, "test-skill");
        let script = files.get("scripts/run.sh").unwrap();
        assert!(script.contains("set -euo pipefail"));
    }

    #[test]
    fn claude_code_template_has_extension_fields() {
        let files = template_files(SkillTemplate::ClaudeCode, "test-skill");
        let content = files.get("SKILL.md").unwrap();
        assert!(content.contains("user-invocable: true"));
        assert!(content.contains("argument-hint:"));
    }

    #[test]
    fn claude_code_template_produces_only_skill_md() {
        let files = template_files(SkillTemplate::ClaudeCode, "test-skill");
        assert_eq!(files.len(), 1);
        assert!(files.contains_key("SKILL.md"));
    }

    #[test]
    fn template_names_derive_from_dir_name() {
        let files = template_files(SkillTemplate::Minimal, "My Cool Skill");
        let content = files.get("SKILL.md").unwrap();
        assert!(content.contains("name: my-cool-skill"));
    }

    #[test]
    fn empty_dir_name_defaults_to_my_skill() {
        let files = template_files(SkillTemplate::Minimal, "");
        let content = files.get("SKILL.md").unwrap();
        assert!(content.contains("name: my-skill"));
    }

    #[test]
    fn all_templates_have_valid_frontmatter() {
        let templates = [
            SkillTemplate::Minimal,
            SkillTemplate::ReferenceGuide,
            SkillTemplate::DomainSpecific,
            SkillTemplate::Workflow,
            SkillTemplate::CodeSkill,
            SkillTemplate::ClaudeCode,
        ];
        for t in templates {
            let files = template_files(t, "test-skill");
            let content = files.get("SKILL.md").unwrap();
            assert!(
                content.starts_with("---\n"),
                "{t:?} template should start with frontmatter"
            );
            assert!(
                content.contains("name: test-skill"),
                "{t:?} template should contain name"
            );
            assert!(
                content.contains("description:"),
                "{t:?} template should contain description"
            );
        }
    }

    #[test]
    fn to_kebab_case_simple() {
        assert_eq!(to_kebab_case("Hello World"), "hello-world");
    }

    #[test]
    fn to_kebab_case_already_kebab() {
        assert_eq!(to_kebab_case("my-skill"), "my-skill");
    }

    #[test]
    fn to_kebab_case_special_chars() {
        assert_eq!(to_kebab_case("foo@bar!baz"), "foo-bar-baz");
    }

    #[test]
    fn to_kebab_case_empty() {
        assert_eq!(to_kebab_case(""), "");
    }
}
