use std::path::Path;

use crate::parser::{find_skill_md, read_properties};
use crate::validator::DiscoveryWarning;

/// A parsed skill entry for prompt generation.
///
/// Collected from a skill directory by reading its SKILL.md frontmatter.
/// Used as the intermediate representation between skill discovery and
/// format-specific rendering.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// Skill name from frontmatter.
    pub name: String,
    /// Skill description from frontmatter.
    pub description: String,
    /// Absolute path to the SKILL.md file.
    pub location: String,
}

/// Output format for prompt generation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PromptFormat {
    /// XML format (default, matches Anthropic spec examples)
    #[default]
    Xml,
    /// JSON array
    Json,
    /// YAML document
    Yaml,
    /// Markdown document
    Markdown,
}

/// Escape all five XML predefined entities: `& < > " '`.
///
/// Ampersand is escaped first to prevent double-escaping of other
/// entity references.
#[must_use]
pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Collect skill entries from a list of directories.
///
/// Each directory is canonicalized to an absolute path, then its SKILL.md
/// is read and parsed. Directories that cannot be canonicalized or parsed
/// are silently skipped.
#[must_use]
pub fn collect_skills(dirs: &[&Path]) -> Vec<SkillEntry> {
    let mut entries = Vec::new();

    for dir in dirs {
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let props = match read_properties(&canonical) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let location = match find_skill_md(&canonical) {
            Some(p) => p.to_string_lossy().to_string(),
            None => continue,
        };

        entries.push(SkillEntry {
            name: props.name,
            description: props.description,
            location,
        });
    }

    entries
}

/// Collect skill entries from directories, collecting warnings for skills that could not be parsed.
///
/// Returns `(entries, warnings)`. The original [`collect_skills()`] function
/// remains unchanged for backward compatibility.
#[must_use]
pub fn collect_skills_verbose(dirs: &[&Path]) -> (Vec<SkillEntry>, Vec<DiscoveryWarning>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for dir in dirs {
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(DiscoveryWarning {
                    path: dir.to_path_buf(),
                    message: format!("cannot canonicalize path: {e}"),
                });
                continue;
            }
        };

        let props = match read_properties(&canonical) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(DiscoveryWarning {
                    path: canonical,
                    message: format!("cannot read skill properties: {e}"),
                });
                continue;
            }
        };

        let location = match find_skill_md(&canonical) {
            Some(p) => p.to_string_lossy().to_string(),
            None => {
                warnings.push(DiscoveryWarning {
                    path: canonical,
                    message: "SKILL.md not found after successful parse".to_string(),
                });
                continue;
            }
        };

        entries.push(SkillEntry {
            name: props.name,
            description: props.description,
            location,
        });
    }

    (entries, warnings)
}

/// Generate an `<available_skills>` XML block from skill directories.
///
/// Each directory is canonicalized to an absolute path, then its SKILL.md
/// is read and parsed. Directories that cannot be canonicalized or parsed
/// are silently skipped — no `<skill>` element is emitted for them.
///
/// The output format matches the
/// [Anthropic skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
/// examples, with 2-space indentation and inline text content.
///
/// # Examples
///
/// ```text
/// <available_skills>
///   <skill>
///     <name>my-skill</name>
///     <description>What it does</description>
///     <location>/abs/path/to/SKILL.md</location>
///   </skill>
/// </available_skills>
/// ```
#[must_use]
pub fn to_prompt(dirs: &[&Path]) -> String {
    let entries = collect_skills(dirs);
    format_xml(&entries)
}

/// Generate prompt output in the specified format.
///
/// This is the multi-format variant of [`to_prompt`]. Use [`PromptFormat::Xml`]
/// for backward-compatible output.
#[must_use]
pub fn to_prompt_format(dirs: &[&Path], format: PromptFormat) -> String {
    let entries = collect_skills(dirs);
    format_entries(&entries, format)
}

/// Format pre-collected skill entries in the specified output format.
///
/// Use this with [`collect_skills_verbose`] when you need access to both the
/// formatted output and any discovery warnings.
#[must_use]
pub fn format_entries(entries: &[SkillEntry], format: PromptFormat) -> String {
    match format {
        PromptFormat::Xml => format_xml(entries),
        PromptFormat::Json => format_json(entries),
        PromptFormat::Yaml => format_yaml(entries),
        PromptFormat::Markdown => format_markdown(entries),
    }
}

/// Estimate the number of tokens in a string.
///
/// Uses the `chars / 4` heuristic, which is a standard approximation for
/// English text. This may underestimate by 30–50% for technical content
/// containing YAML frontmatter, XML tags, or code blocks. Uses character
/// count (not byte length) to handle non-ASCII content correctly.
#[must_use]
pub fn estimate_tokens(s: &str) -> usize {
    // Minimum 1 token for non-empty strings.
    if s.is_empty() {
        0
    } else {
        (s.chars().count() / 4).max(1)
    }
}

/// Format a token budget report for a collection of skill entries.
///
/// Reports per-skill estimates and a total with context usage percentage.
/// Emits a warning if the total exceeds 4000 tokens (~2% of 200k context).
#[must_use]
pub fn format_budget(entries: &[SkillEntry]) -> String {
    let mut out = String::from("Token budget (estimated):\n");

    let mut total = 0usize;
    for entry in entries {
        // Estimate tokens for the prompt representation of this skill.
        let skill_text = format!("{} {} {}", entry.name, entry.description, entry.location);
        let tokens = estimate_tokens(&skill_text);
        total += tokens;
        out.push_str(&format!("  {:<30} ~{} tokens\n", entry.name, tokens));
    }

    out.push_str(&format!("  {:<30} ---\n", ""));
    out.push_str(&format!("  {:<30} ~{} tokens\n", "Total:", total));

    let pct = (total as f64 / 200_000.0) * 100.0;
    out.push_str(&format!("  {:<30} {:.1}% of 200k\n", "Context usage:", pct));

    if total > 4000 {
        out.push_str(
            "\n  ⚠ Total exceeds 4000 tokens (~2% of context). Consider consolidating skills.\n",
        );
    }

    out
}

// ── Format implementations ─────────────────────────────────────────────

fn format_xml(entries: &[SkillEntry]) -> String {
    let mut out = String::from("<available_skills>\n");

    for entry in entries {
        out.push_str("  <skill>\n");
        out.push_str(&format!("    <name>{}</name>\n", xml_escape(&entry.name)));
        out.push_str(&format!(
            "    <description>{}</description>\n",
            xml_escape(&entry.description)
        ));
        out.push_str(&format!(
            "    <location>{}</location>\n",
            xml_escape(&entry.location)
        ));
        out.push_str("  </skill>\n");
    }

    out.push_str("</available_skills>");
    out
}

fn format_json(entries: &[SkillEntry]) -> String {
    let items: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "description": e.description,
                "location": e.location,
            })
        })
        .collect();

    serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".to_string())
}

fn format_yaml(entries: &[SkillEntry]) -> String {
    let mut out = String::from("skills:\n");

    for entry in entries {
        out.push_str(&format!("  - name: {}\n", yaml_quote(&entry.name)));
        out.push_str(&format!(
            "    description: {}\n",
            yaml_quote(&entry.description)
        ));
        out.push_str(&format!("    location: {}\n", yaml_quote(&entry.location)));
    }

    out
}

fn format_markdown(entries: &[SkillEntry]) -> String {
    let mut out = String::from("# Available Skills\n\n");

    for entry in entries {
        out.push_str(&format!("## {}\n\n", entry.name));
        out.push_str(&format!("> {}\n\n", entry.description));
        out.push_str(&format!("**Location**: `{}`\n\n", entry.location));
        out.push_str("---\n\n");
    }

    out
}

/// Quote a YAML string value if it contains special characters.
fn yaml_quote(s: &str) -> String {
    if s.contains(':')
        || s.contains('#')
        || s.contains('\'')
        || s.contains('"')
        || s.contains('\n')
        || s.starts_with(' ')
        || s.starts_with('{')
        || s.starts_with('[')
        || s.starts_with('*')
        || s.starts_with('&')
    {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Create a temp dir with a named subdirectory containing a SKILL.md.
    /// Returns the parent TempDir (for lifetime) and the path to the subdirectory.
    fn make_skill_dir(name: &str, frontmatter: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let parent = tempdir().unwrap();
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), frontmatter).unwrap();
        (parent, dir)
    }

    // ── xml_escape tests ──────────────────────────────────────────────

    #[test]
    fn xml_escape_ampersand() {
        assert_eq!(xml_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn xml_escape_less_than() {
        assert_eq!(xml_escape("a < b"), "a &lt; b");
    }

    #[test]
    fn xml_escape_greater_than() {
        assert_eq!(xml_escape("a > b"), "a &gt; b");
    }

    #[test]
    fn xml_escape_double_quote() {
        assert_eq!(xml_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn xml_escape_single_quote() {
        assert_eq!(xml_escape("it's"), "it&apos;s");
    }

    #[test]
    fn xml_escape_no_special_characters() {
        assert_eq!(xml_escape("hello world"), "hello world");
    }

    #[test]
    fn xml_escape_multiple_special_characters() {
        assert_eq!(
            xml_escape("<tag attr=\"v\">&'x'</tag>"),
            "&lt;tag attr=&quot;v&quot;&gt;&amp;&apos;x&apos;&lt;/tag&gt;"
        );
    }

    #[test]
    fn xml_escape_ampersand_first_no_double_escape() {
        // Input contains literal "&lt;" (ampersand + "lt;").
        // The & must be escaped first → "&amp;lt;", not left as "&lt;".
        assert_eq!(xml_escape("&lt;"), "&amp;lt;");
    }

    // ── collect_skills tests ──────────────────────────────────────────

    #[test]
    fn collect_skills_empty_list() {
        let entries = collect_skills(&[]);
        assert!(entries.is_empty());
    }

    #[test]
    fn collect_skills_single_skill() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let entries = collect_skills(&[dir.as_path()]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "my-skill");
        assert_eq!(entries[0].description, "A test skill");
        assert!(entries[0].location.ends_with("SKILL.md"));
    }

    #[test]
    fn collect_skills_skips_invalid() {
        let parent = tempdir().unwrap();
        let bad = parent.path().join("no-skill");
        fs::create_dir(&bad).unwrap();
        let entries = collect_skills(&[bad.as_path()]);
        assert!(entries.is_empty());
    }

    // ── to_prompt tests (backward compatibility) ──────────────────────

    #[test]
    fn to_prompt_empty_list() {
        let result = to_prompt(&[]);
        assert_eq!(result, "<available_skills>\n</available_skills>");
    }

    #[test]
    fn to_prompt_single_skill() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let result = to_prompt(&[dir.as_path()]);
        assert!(result.contains("<name>my-skill</name>"));
        assert!(result.contains("<description>A test skill</description>"));
        assert!(result.contains("<location>"));
        assert!(result.contains("SKILL.md</location>"));
        assert!(result.starts_with("<available_skills>\n"));
        assert!(result.ends_with("</available_skills>"));
    }

    #[test]
    fn to_prompt_multiple_skills() {
        let (_p1, d1) = make_skill_dir(
            "skill-one",
            "---\nname: skill-one\ndescription: First\n---\n",
        );
        let (_p2, d2) = make_skill_dir(
            "skill-two",
            "---\nname: skill-two\ndescription: Second\n---\n",
        );
        let result = to_prompt(&[d1.as_path(), d2.as_path()]);
        assert!(result.contains("<name>skill-one</name>"));
        assert!(result.contains("<name>skill-two</name>"));
        assert!(result.contains("<description>First</description>"));
        assert!(result.contains("<description>Second</description>"));
        // Two <skill> blocks.
        assert_eq!(result.matches("<skill>").count(), 2);
        assert_eq!(result.matches("</skill>").count(), 2);
    }

    #[test]
    fn to_prompt_special_characters_escaped() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: Uses <xml> & \"quotes\"\n---\n",
        );
        let result = to_prompt(&[dir.as_path()]);
        assert!(result.contains("&lt;xml&gt; &amp; &quot;quotes&quot;"));
    }

    #[test]
    fn to_prompt_invalid_directory_skipped() {
        let parent = tempdir().unwrap();
        let bad_dir = parent.path().join("no-skill-here");
        fs::create_dir(&bad_dir).unwrap();
        // Directory exists but has no SKILL.md.
        let result = to_prompt(&[bad_dir.as_path()]);
        assert_eq!(result, "<available_skills>\n</available_skills>");
    }

    #[test]
    fn to_prompt_mix_valid_and_invalid() {
        let (_p1, good) = make_skill_dir(
            "good-skill",
            "---\nname: good-skill\ndescription: Works\n---\n",
        );
        let parent = tempdir().unwrap();
        let bad = parent.path().join("bad-skill");
        fs::create_dir(&bad).unwrap();
        // bad has no SKILL.md.
        let result = to_prompt(&[good.as_path(), bad.as_path()]);
        assert!(result.contains("<name>good-skill</name>"));
        assert_eq!(result.matches("<skill>").count(), 1);
    }

    #[test]
    fn to_prompt_location_is_absolute() {
        let (_parent, dir) =
            make_skill_dir("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        let result = to_prompt(&[dir.as_path()]);
        // Extract the location value from the XML.
        let start = result.find("<location>").unwrap() + "<location>".len();
        let end = result.find("</location>").unwrap();
        let location = &result[start..end];
        // Location must be an absolute path.
        assert!(
            std::path::Path::new(location).is_absolute(),
            "expected absolute path, got: {location}"
        );
        assert!(location.ends_with("SKILL.md"));
    }

    // ── Multi-format tests ────────────────────────────────────────────

    #[test]
    fn xml_format_matches_to_prompt() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let xml = to_prompt_format(&[dir.as_path()], PromptFormat::Xml);
        let legacy = to_prompt(&[dir.as_path()]);
        assert_eq!(xml, legacy);
    }

    #[test]
    fn json_format_is_valid_json() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let json = to_prompt_format(&[dir.as_path()], PromptFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "my-skill");
        assert_eq!(arr[0]["description"], "A test skill");
    }

    #[test]
    fn json_format_empty() {
        let json = to_prompt_format(&[], PromptFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.as_array().unwrap().is_empty());
    }

    #[test]
    fn yaml_format_has_skills_key() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let yaml = to_prompt_format(&[dir.as_path()], PromptFormat::Yaml);
        assert!(yaml.starts_with("skills:\n"));
        assert!(yaml.contains("  - name: my-skill\n"));
        assert!(yaml.contains("    description: A test skill\n"));
        assert!(yaml.contains("    location:"));
    }

    #[test]
    fn yaml_format_empty() {
        let yaml = to_prompt_format(&[], PromptFormat::Yaml);
        assert_eq!(yaml, "skills:\n");
    }

    #[test]
    fn markdown_format_has_headings() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let md = to_prompt_format(&[dir.as_path()], PromptFormat::Markdown);
        assert!(md.starts_with("# Available Skills\n"));
        assert!(md.contains("## my-skill\n"));
        assert!(md.contains("> A test skill\n"));
        assert!(md.contains("**Location**:"));
    }

    #[test]
    fn markdown_format_empty() {
        let md = to_prompt_format(&[], PromptFormat::Markdown);
        assert_eq!(md, "# Available Skills\n\n");
    }

    // ── Token budget tests ────────────────────────────────────────────

    #[test]
    fn estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_short() {
        // 3 chars → max(3/4, 1) = 1
        assert_eq!(estimate_tokens("abc"), 1);
    }

    #[test]
    fn estimate_tokens_longer() {
        // 100 chars → 100/4 = 25
        let s = "a".repeat(100);
        assert_eq!(estimate_tokens(&s), 25);
    }

    #[test]
    fn format_budget_single_skill() {
        let entries = vec![SkillEntry {
            name: "my-skill".to_string(),
            description: "Does things".to_string(),
            location: "/path/to/SKILL.md".to_string(),
        }];
        let budget = format_budget(&entries);
        assert!(budget.contains("my-skill"));
        assert!(budget.contains("Total:"));
        assert!(budget.contains("Context usage:"));
    }

    #[test]
    fn format_budget_warning_over_threshold() {
        // Create entries that exceed 4000 tokens total.
        let big_desc = "x".repeat(20_000);
        let entries = vec![SkillEntry {
            name: "big-skill".to_string(),
            description: big_desc,
            location: "/path/to/SKILL.md".to_string(),
        }];
        let budget = format_budget(&entries);
        assert!(
            budget.contains("⚠"),
            "should warn when over 4000 tokens: {budget}"
        );
    }

    #[test]
    fn format_budget_no_warning_under_threshold() {
        let entries = vec![SkillEntry {
            name: "small-skill".to_string(),
            description: "Short".to_string(),
            location: "/path/to/SKILL.md".to_string(),
        }];
        let budget = format_budget(&entries);
        assert!(
            !budget.contains("⚠"),
            "should not warn under 4000 tokens: {budget}"
        );
    }

    // ── yaml_quote tests ──────────────────────────────────────────────

    #[test]
    fn yaml_quote_plain() {
        assert_eq!(yaml_quote("hello"), "hello");
    }

    #[test]
    fn yaml_quote_colon() {
        assert_eq!(yaml_quote("key: value"), "\"key: value\"");
    }

    #[test]
    fn yaml_quote_quotes() {
        assert_eq!(yaml_quote("say \"hi\""), "\"say \\\"hi\\\"\"");
    }

    // ── collect_skills_verbose tests ─────────────────────────────────

    #[test]
    fn collect_skills_verbose_valid() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let (entries, warnings) = collect_skills_verbose(&[dir.as_path()]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "my-skill");
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn collect_skills_verbose_invalid_skill_md() {
        let parent = tempdir().unwrap();
        let bad = parent.path().join("bad-skill");
        fs::create_dir(&bad).unwrap();
        // Write an invalid SKILL.md with no frontmatter delimiters.
        fs::write(bad.join("SKILL.md"), "no frontmatter here").unwrap();
        let (entries, warnings) = collect_skills_verbose(&[bad.as_path()]);
        assert!(entries.is_empty(), "expected no entries for invalid skill");
        assert_eq!(warnings.len(), 1, "expected one warning, got: {warnings:?}");
        assert!(
            warnings[0].message.contains("cannot read skill properties"),
            "expected parse error warning, got: {}",
            warnings[0].message
        );
    }

    #[test]
    fn collect_skills_verbose_missing_skill_md() {
        let parent = tempdir().unwrap();
        let empty = parent.path().join("empty-dir");
        fs::create_dir(&empty).unwrap();
        let (entries, warnings) = collect_skills_verbose(&[empty.as_path()]);
        assert!(entries.is_empty());
        assert_eq!(warnings.len(), 1, "expected one warning, got: {warnings:?}");
        assert!(
            warnings[0].message.contains("cannot read skill properties"),
            "expected parse error warning, got: {}",
            warnings[0].message
        );
    }

    #[test]
    fn collect_skills_verbose_nonexistent_path() {
        let nonexistent = std::path::Path::new("/nonexistent/path/does/not/exist");
        let (entries, warnings) = collect_skills_verbose(&[nonexistent]);
        assert!(entries.is_empty());
        assert_eq!(warnings.len(), 1, "expected one warning, got: {warnings:?}");
        assert!(
            warnings[0].message.contains("cannot canonicalize"),
            "expected canonicalize error, got: {}",
            warnings[0].message
        );
    }

    #[test]
    fn collect_skills_backward_compat() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A test skill\n---\n",
        );
        let original = collect_skills(&[dir.as_path()]);
        let (verbose, _) = collect_skills_verbose(&[dir.as_path()]);
        assert_eq!(original.len(), verbose.len());
        assert_eq!(original[0].name, verbose[0].name);
        assert_eq!(original[0].description, verbose[0].description);
    }

    // ── format_entries tests ─────────────────────────────────────────

    #[test]
    fn format_entries_xml() {
        let entries = vec![SkillEntry {
            name: "test-skill".to_string(),
            description: "Does things".to_string(),
            location: "/path/to/SKILL.md".to_string(),
        }];
        let result = format_entries(&entries, PromptFormat::Xml);
        assert!(result.contains("<name>test-skill</name>"));
        assert!(result.starts_with("<available_skills>"));
    }
}
