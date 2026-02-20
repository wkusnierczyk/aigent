---
name: aigent-scorer
description: >-
  Scores AI agent skill definitions (SKILL.md files) against the Anthropic
  best-practices checklist. Provides quality ratings (0-100) with structural
  and semantic breakdown. Use when reviewing skill quality, improving existing
  skills, or preparing skills for sharing.
allowed-tools: Bash(aigent score *), Bash(aigent validate *), Bash(command -v *), Read, Glob
argument-hint: "[skill-directory-or-file]"
---

# Skill Scorer

## Setup

Check if `aigent` is available:

```bash
command -v aigent
```

## With aigent CLI

If `aigent` is on `$PATH`, use the CLI for authoritative scoring:

```bash
aigent score <skill-directory>
```

Output shows a 0-100 score with breakdown:

```
Score: 76/100

Structural (60/60):
  [PASS] SKILL.md exists and is parseable
  [PASS] Name format valid
  ...

Quality (16/40):
  [PASS] Third-person description
  [FAIL] Missing trigger phrase
  ...
```

Use `--format json` for machine-readable output.

Exit code 0 means perfect score (100/100). Non-zero means room for
improvement.

## Without aigent CLI

If `aigent` is not available, score manually using this checklist.

### Structural checks (60 points)

All structural checks must pass to earn 60 points. Any failure zeroes the
structural score.

1. **SKILL.md exists** — file present with valid YAML frontmatter
2. **Name format** — lowercase, hyphens, digits only; no consecutive hyphens;
   no leading/trailing hyphens; no reserved words; matches directory name;
   64 chars max
3. **Description valid** — non-empty, 1024 chars max, no XML tags
4. **Required fields** — `name` and `description` present and string-typed
5. **No unknown fields** — only spec-defined keys in frontmatter
6. **Body size** — body under 500 lines

### Quality checks (40 points, 8 each)

Each passing check earns 8 points.

1. **Third-person description** — no "I", "me", "my", "you", "your"
2. **Trigger phrase** — description includes "Use when", "Use for", or similar
3. **Gerund name** — first segment ends in "-ing" (e.g., "processing-pdfs")
4. **Specific name** — not generic ("helper", "utils", "tools", etc.)
5. **Detailed description** — at least 20 characters and 4 words

### Scoring formula

```
total = structural_pass ? 60 : 0
total += quality_checks_passed * 8
```

Maximum score: 100 (60 structural + 40 quality).
