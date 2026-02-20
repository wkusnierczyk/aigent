---
name: aigent-validator
description: >-
  Validates AI agent skill definitions (SKILL.md files) against the Anthropic
  agent skill specification. Checks frontmatter fields (name, description),
  format rules, and body guidelines. Use when validating skills, checking
  SKILL.md files, or reviewing skill definitions for spec compliance.
allowed-tools: Bash(aigent validate *), Bash(command -v *), Read, Glob
argument-hint: "[skill-directory-or-file]"
---

# Skill Validator

## Setup

Check if `aigent` is available:

```bash
command -v aigent
```

## With aigent CLI

If `aigent` is on `$PATH`, use the CLI for authoritative validation:

```bash
aigent validate <skill-directory>
```

Exit code 0 means valid. Non-zero means errors. Warnings are printed but
do not cause failure.

If a SKILL.md file path is provided instead of a directory, `aigent`
automatically resolves to the parent directory.

## Without aigent CLI

If `aigent` is not available, validate manually by reading the SKILL.md
and checking these rules:

### Frontmatter checks

1. File starts and ends with `---` delimiters
2. `name` field: present, string, ≤ 64 chars, lowercase + hyphens + numbers
   only, no consecutive hyphens, no leading/trailing hyphens, no reserved
   words ("anthropic", "claude"), no XML tags, matches directory name
3. `description` field: present, string, non-empty, ≤ 1024 chars, no XML tags

### Optional field checks

4. `compatibility`: if present, string, ≤ 500 chars
5. `license`: if present, string
6. `allowed-tools`: if present, string

### Body checks

7. Body ≤ 500 lines (warning, not error)

### Report format

Report each issue on its own line. Prefix warnings with "warning: ".
