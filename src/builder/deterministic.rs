use super::util::{capitalize_first, to_title_case};
use super::ClarityAssessment;

/// Filler words to remove from purpose strings during name derivation.
const FILLER_WORDS: &[&str] = &[
    "a", "an", "the", "to", "for", "from", "with", "and", "or", "that", "which", "this", "my",
    "of", "in", "on", "is", "are", "be",
];

/// Derive a kebab-case skill name from a natural language description.
///
/// Steps: lowercase → remove filler words → gerund-form first word →
/// join with hyphens → sanitize → truncate to 64 characters.
#[must_use]
pub fn derive_name(purpose: &str) -> String {
    let lower = purpose.to_lowercase();

    // Split into words, filter fillers.
    let words: Vec<&str> = lower
        .split_whitespace()
        .filter(|w| {
            let stripped = w.trim_matches(|c: char| !c.is_alphanumeric());
            !FILLER_WORDS.contains(&stripped)
        })
        .collect();

    if words.is_empty() {
        return "my-skill".to_string();
    }

    // Apply gerund form to the first word.
    let mut result_words: Vec<String> = Vec::with_capacity(words.len());
    let first = words[0].trim_matches(|c: char| !c.is_alphanumeric());
    result_words.push(to_gerund(first));

    for w in &words[1..] {
        let cleaned = w.trim_matches(|c: char| !c.is_alphanumeric());
        if !cleaned.is_empty() {
            result_words.push(cleaned.to_string());
        }
    }

    // Join with hyphens, sanitize.
    let joined = result_words.join("-");

    // Remove characters not in [a-z0-9-].
    let sanitized: String = joined
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect();

    // Collapse consecutive hyphens and trim.
    let collapsed = collapse_hyphens(&sanitized);
    let trimmed = collapsed.trim_matches('-');

    if trimmed.is_empty() {
        return "my-skill".to_string();
    }

    // Truncate to 64 characters at a hyphen boundary if possible.
    truncate_at_boundary(trimmed, 64)
}

/// Convert a word to gerund form (add "ing").
fn to_gerund(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }

    // Already ends in "ing".
    if word.ends_with("ing") {
        return word.to_string();
    }

    // Ends in "ie" → drop "ie", add "ying" (e.g., "die" → "dying").
    if let Some(stem) = word.strip_suffix("ie") {
        return format!("{stem}ying");
    }

    // Ends in "e" (not "ee") → drop "e", add "ing" (e.g., "analyze" → "analyzing").
    if word.ends_with('e') && !word.ends_with("ee") && word.len() > 1 {
        let stem = &word[..word.len() - 1];
        return format!("{stem}ing");
    }

    // CVC pattern for short words: double final consonant.
    // Only apply to common short words (3-4 chars) to avoid over-doubling.
    if word.len() >= 3 && word.len() <= 4 && is_cvc(word) {
        if let Some(last) = word.chars().last() {
            return format!("{word}{last}ing");
        }
    }

    // Default: just add "ing".
    format!("{word}ing")
}

/// Check if a word ends in consonant-vowel-consonant pattern.
fn is_cvc(word: &str) -> bool {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    if len < 3 {
        return false;
    }
    let last = chars[len - 1];
    let second_last = chars[len - 2];
    let third_last = chars[len - 3];

    // Don't double w, x, or y.
    if last == 'w' || last == 'x' || last == 'y' {
        return false;
    }

    is_consonant(last) && is_vowel(second_last) && is_consonant(third_last)
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn is_consonant(c: char) -> bool {
    c.is_ascii_lowercase() && !is_vowel(c)
}

/// Collapse consecutive hyphens into a single hyphen.
fn collapse_hyphens(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_hyphen = false;
    for c in s.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }
    result
}

/// Truncate a string to `max_len` characters, preferring a hyphen boundary.
fn truncate_at_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }

    let truncated = &s[..max_len];
    // Find the last hyphen to break cleanly.
    if let Some(pos) = truncated.rfind('-') {
        if pos > 0 {
            return truncated[..pos].to_string();
        }
    }
    truncated.to_string()
}

/// Generate a template-based description from a purpose string.
#[must_use]
pub fn generate_description(purpose: &str, _name: &str) -> String {
    let capitalized = capitalize_first(purpose.trim());

    // Add period if not already present.
    let sentence = if capitalized.ends_with('.') || capitalized.ends_with('!') {
        capitalized
    } else {
        format!("{capitalized}.")
    };

    // Derive trigger context from purpose.
    let trigger = derive_trigger(purpose);
    let description = format!("{sentence} Use when {trigger}.");

    // Truncate to 1024 characters if needed (char-safe for multibyte UTF-8).
    if description.chars().count() > 1024 {
        description.chars().take(1024).collect()
    } else {
        description
    }
}

/// Derive a trigger context from the purpose string.
fn derive_trigger(purpose: &str) -> String {
    let words: Vec<&str> = purpose.split_whitespace().collect();
    if words.len() >= 3 {
        // Use the last significant word as the object.
        if let Some(last_word) = words.last() {
            let last = last_word.trim_matches(|c: char| !c.is_alphanumeric());
            if !last.is_empty() {
                return format!("working with {last}");
            }
        }
    }
    "this capability is needed".to_string()
}

/// Generate a template-based markdown body.
#[must_use]
pub fn generate_body(purpose: &str, name: &str, _description: &str) -> String {
    let title = to_title_case(name);
    let version = env!("CARGO_PKG_VERSION");

    format!(
        "# {title}\n\
         \n\
         ## Quick start\n\
         \n\
         {purpose}\n\
         \n\
         ## Usage\n\
         \n\
         Use this skill to {purpose}.\n\
         \n\
         ## Notes\n\
         \n\
         - Generated by aigent {version}\n\
         - Edit this file to customize the skill\n"
    )
}

/// Evaluate if a purpose description is clear enough for autonomous generation.
///
/// Deterministic heuristics based on word count and structure.
#[must_use]
pub fn assess_clarity(purpose: &str) -> ClarityAssessment {
    let trimmed = purpose.trim();
    let word_count = trimmed.split_whitespace().count();

    // Too short.
    if word_count < 3 {
        return ClarityAssessment {
            clear: false,
            questions: vec![
                "Can you provide more detail about what the skill should do?".to_string(),
            ],
        };
    }

    // Contains question mark — user is asking, not describing.
    if trimmed.contains('?') {
        return ClarityAssessment {
            clear: false,
            questions: vec![
                "Please provide a statement describing the skill, not a question.".to_string(),
            ],
        };
    }

    // Long enough to be clear.
    if word_count > 10 {
        return ClarityAssessment {
            clear: true,
            questions: vec![],
        };
    }

    // Medium length — check for verb-like words (heuristic).
    let has_verb = trimmed.split_whitespace().any(|w| {
        let lower = w.to_lowercase();
        lower.ends_with("ing")
            || lower.ends_with("ate")
            || lower.ends_with("ize")
            || lower.ends_with("ify")
            || lower.ends_with("ect")
            || matches!(
                lower.as_str(),
                "run"
                    | "get"
                    | "set"
                    | "add"
                    | "put"
                    | "use"
                    | "make"
                    | "read"
                    | "write"
                    | "send"
                    | "find"
                    | "check"
                    | "build"
                    | "create"
                    | "delete"
                    | "update"
                    | "parse"
                    | "format"
                    | "deploy"
                    | "process"
                    | "analyze"
                    | "generate"
                    | "validate"
                    | "convert"
                    | "extract"
                    | "transform"
                    | "handle"
                    | "manage"
            )
    });

    if has_verb {
        ClarityAssessment {
            clear: true,
            questions: vec![],
        }
    } else {
        ClarityAssessment {
            clear: false,
            questions: vec![
                "Can you describe the specific task or workflow this skill should handle?"
                    .to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── derive_name tests (1-10) ──────────────────────────────────────

    #[test]
    fn derive_name_process_pdf_files() {
        let name = derive_name("Process PDF files");
        assert!(
            name.starts_with("processing"),
            "expected gerund form, got: {name}"
        );
    }

    #[test]
    fn derive_name_analyze_gerund() {
        let name = derive_name("Analyze spreadsheet data");
        assert!(
            name.starts_with("analyzing"),
            "expected 'analyzing', got: {name}"
        );
    }

    #[test]
    fn derive_name_run_cvc_doubling() {
        let name = derive_name("Run database migrations");
        assert!(
            name.starts_with("running"),
            "expected 'running' (CVC doubling), got: {name}"
        );
    }

    #[test]
    fn derive_name_already_gerund() {
        let name = derive_name("processing files");
        assert!(
            name.starts_with("processing"),
            "expected to keep 'processing', got: {name}"
        );
    }

    #[test]
    fn derive_name_single_word() {
        let name = derive_name("deploy");
        assert_eq!(name, "deploying");
    }

    #[test]
    fn derive_name_filler_words_removed() {
        let name = derive_name("a tool for the processing of data");
        // "a", "for", "the", "of" are fillers; "tool", "processing", "data" remain.
        // "tool" → "tooling" (gerund), then "processing", "data".
        assert!(
            !name.contains("-a-") && !name.contains("-for-") && !name.contains("-the-"),
            "filler words should be removed, got: {name}"
        );
        assert_eq!(name, "tooling-processing-data");
    }

    #[test]
    fn derive_name_special_characters_stripped() {
        let name = derive_name("Process PDFs!");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "should only contain [a-z0-9-], got: {name}"
        );
    }

    #[test]
    fn derive_name_empty_input() {
        assert_eq!(derive_name(""), "my-skill");
    }

    #[test]
    fn derive_name_long_input_truncated() {
        let long_purpose = "process ".repeat(20) + "the final long word";
        let name = derive_name(&long_purpose);
        assert!(
            name.len() <= 64,
            "should be ≤ 64 chars, got {} chars: {name}",
            name.len()
        );
    }

    #[test]
    fn derive_name_passes_validation() {
        let name = derive_name("Process PDF files for automated archival");
        // No uppercase.
        assert_eq!(name, name.to_lowercase(), "should be lowercase");
        // No consecutive hyphens.
        assert!(!name.contains("--"), "should not have consecutive hyphens");
        // No leading/trailing hyphens.
        assert!(!name.starts_with('-'), "should not start with hyphen");
        assert!(!name.ends_with('-'), "should not end with hyphen");
    }

    // ── generate_description tests (11-14) ────────────────────────────

    #[test]
    fn generate_description_non_empty() {
        let desc = generate_description("Process PDF files", "processing-pdf-files");
        assert!(!desc.is_empty(), "description should not be empty");
    }

    #[test]
    fn generate_description_contains_purpose_words() {
        let desc = generate_description("Process PDF files", "processing-pdf-files");
        let lower = desc.to_lowercase();
        assert!(
            lower.contains("process") || lower.contains("pdf") || lower.contains("files"),
            "should contain purpose-related words, got: {desc}"
        );
    }

    #[test]
    fn generate_description_within_limit() {
        let long_purpose = "word ".repeat(300);
        let desc = generate_description(&long_purpose, "long-name");
        assert!(
            desc.len() <= 1024,
            "should be ≤ 1024 chars, got {} chars",
            desc.len()
        );
    }

    #[test]
    fn generate_description_third_person() {
        let desc = generate_description("Process PDF files", "processing-pdf-files");
        assert!(
            !desc.starts_with("I ") && !desc.starts_with("You "),
            "should be third person, got: {desc}"
        );
    }

    // ── generate_body tests (15-18) ───────────────────────────────────

    #[test]
    fn generate_body_non_empty() {
        let body = generate_body("Process PDFs", "processing-pdfs", "Processes PDFs.");
        assert!(!body.is_empty(), "body should not be empty");
    }

    #[test]
    fn generate_body_contains_heading_with_name() {
        let body = generate_body("Process PDFs", "processing-pdfs", "Processes PDFs.");
        assert!(
            body.contains("# Processing Pdfs"),
            "should contain heading with skill name, got:\n{body}"
        );
    }

    #[test]
    fn generate_body_contains_quick_start() {
        let body = generate_body("Process PDFs", "processing-pdfs", "Processes PDFs.");
        assert!(
            body.contains("## Quick start"),
            "should contain Quick start section"
        );
    }

    #[test]
    fn generate_body_contains_version() {
        let body = generate_body("Process PDFs", "processing-pdfs", "Processes PDFs.");
        let version = env!("CARGO_PKG_VERSION");
        assert!(
            body.contains(version),
            "should contain aigent version {version}"
        );
    }

    // ── assess_clarity tests (19-23) ──────────────────────────────────

    #[test]
    fn assess_clarity_short_input_not_clear() {
        let result = assess_clarity("do stuff");
        assert!(!result.clear, "short input should not be clear");
    }

    #[test]
    fn assess_clarity_question_not_clear() {
        let result = assess_clarity("What should this skill do?");
        assert!(!result.clear, "question should not be clear");
    }

    #[test]
    fn assess_clarity_detailed_purpose_clear() {
        let result = assess_clarity(
            "Process PDF files and extract text content for automated archival in a database",
        );
        assert!(
            result.clear,
            "detailed purpose (> 10 words) should be clear"
        );
    }

    #[test]
    fn assess_clarity_clear_has_empty_questions() {
        let result = assess_clarity(
            "Process PDF files and extract text content for automated archival in a database",
        );
        assert!(
            result.questions.is_empty(),
            "clear assessment should have empty questions"
        );
    }

    #[test]
    fn assess_clarity_unclear_has_questions() {
        let result = assess_clarity("do stuff");
        assert!(
            !result.questions.is_empty(),
            "unclear assessment should have non-empty questions"
        );
    }
}
