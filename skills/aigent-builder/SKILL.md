---
name: aigent-builder
description: >-
  Generates AI agent skill definitions (SKILL.md files) from natural language
  descriptions. Creates complete skill directories with valid frontmatter and
  markdown body following the Anthropic agent skill specification. Use when
  creating new skills, generating SKILL.md files, or scaffolding skill
  directories.
allowed-tools: Bash(aigent *), Bash(command -v *), Write, Read, Glob
argument-hint: "[skill-description]"
context: fork
---

# Skill Builder

## Setup

Check if `aigent` is available:

```bash
command -v aigent
```

## With aigent CLI

If `aigent` is on `$PATH`, use the CLI for authoritative skill generation:

1. Build the skill:
   ```bash
   aigent build "<purpose>" --dir .claude/skills/<name>/
   ```

2. Validate the result:
   ```bash
   aigent validate .claude/skills/<name>/
   ```

3. If validation reports errors, fix the SKILL.md and re-validate.

Use `--no-llm` to force deterministic mode (no API keys needed).
Use `--name <name>` to override the derived skill name.

The CLI defaults to outputting in `./<name>/` in the current directory. The
`--dir` flag above overrides this to place skills in `.claude/skills/`, which
is the standard Claude Code skill location.

## Without aigent CLI

If `aigent` is not available, generate the SKILL.md directly.

### Name rules

- Lowercase letters, numbers, and hyphens only
- Maximum 64 characters
- Prefer gerund form: "processing-pdfs", "analyzing-data"
- No reserved words: "anthropic", "claude"
- No XML tags
- Must match directory name

### Description rules

- Non-empty, maximum 1024 characters
- Third person: "Processes PDFs..." not "I process PDFs..."
- Include what the skill does AND when to use it
- No XML tags

### Body guidelines

- Keep under 500 lines
- Be concise — only add context Claude doesn't already have
- Use ## headings for sections
- Link to additional files for large content

### Output

Write the SKILL.md to `.claude/skills/<name>/SKILL.md` with:

```
---
name: <kebab-case-name>
description: <what-it-does-and-when-to-use-it>
---

# <Title>

## Quick start

[Concise instructions]

## Usage

[Detailed usage]
```
