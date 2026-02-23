# Claude Code plugin

> Back to [README](../README.md)

- [Skills](#skills)
- [Plugin installation](#plugin-installation)

This repository is a
[Claude Code plugin](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code/extensions#custom-slash-commands).
It provides three skills that Claude can use to build, validate, and score
SKILL.md files interactively.

## Skills

| Skill | Description |
|-------|-------------|
| `aigent-builder` | Generates `SKILL.md` definitions from natural language. Triggered by "create a skill", "build a skill", etc. |
| `aigent-validator` | Validates `SKILL.md` files against the Anthropic specification. Triggered by "validate a skill", "check a skill", etc. |
| `aigent-scorer` | Scores `SKILL.md` files against best-practices checklist. Triggered by "score a skill", "rate a skill", etc. |

All skills operate in **hybrid mode**: they use the `aigent` CLI when it is
installed, and fall back to Claude-based generation/validation when it is not.
This means the plugin works out of the box — no installation required — but
produces higher-quality results with `aigent` available.

## Plugin installation

To use the plugin in Claude Code, add it to your project's
`.claude/settings.json`:

```json
{
  "permissions": {
    "allow": []
  },
  "plugins": [
    "wkusnierczyk/aigent"
  ]
}
```
