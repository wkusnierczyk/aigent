use std::path::Path;

use crate::parser::{find_skill_md, read_properties};

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
    let mut out = String::from("<available_skills>\n");

    for dir in dirs {
        // Canonicalize to absolute path; skip if the path doesn't exist.
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Read properties; skip if SKILL.md is missing or unparsable.
        let props = match read_properties(&canonical) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Find SKILL.md path (defensive — should always succeed after read_properties).
        let location = match find_skill_md(&canonical) {
            Some(p) => p,
            None => continue,
        };

        let location_str = xml_escape(&location.to_string_lossy());

        // Build the <skill> element with 2/4-space indentation.
        out.push_str("  <skill>\n");
        out.push_str(&format!("    <name>{}</name>\n", xml_escape(&props.name)));
        out.push_str(&format!(
            "    <description>{}</description>\n",
            xml_escape(&props.description)
        ));
        out.push_str(&format!("    <location>{location_str}</location>\n"));
        out.push_str("  </skill>\n");
    }

    out.push_str("</available_skills>");
    out
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

    // ── to_prompt tests ───────────────────────────────────────────────

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
}
