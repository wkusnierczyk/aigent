# Plan: Show diff in `format --check` output (#115)

## Context

`aigent format --check` currently only reports *which* files would change ("Would reformat: my-skill/"). The user wants to also show *what* would change — a unified diff. Per the issue comment, when LLM support is available, a semantic summary should precede the diff. This plan covers the deterministic diff; the LLM semantic summary can be a follow-up.

## Changes

### 1. Add `similar` dependency (`Cargo.toml`)

Add `similar = "2"` to `[dependencies]`. This is the most popular Rust diff crate (~15M downloads), supports unified diff output, and is actively maintained.

### 2. Extend `FormatResult` to include original content (`src/formatter.rs`)

Add an `original: String` field to `FormatResult` so the CLI handler can compute diffs without re-reading the file.

```rust
pub struct FormatResult {
    pub changed: bool,
    pub content: String,
    pub original: String,  // NEW
}
```

Update `format_skill()` to populate the `original` field from the read content.

### 3. Add diff generation function (`src/formatter.rs`)

Add a public `diff_skill(result: &FormatResult, path: &str) -> String` function that produces unified diff output using `similar::TextDiff`. The output uses `--- path` / `+++ path (formatted)` headers with `@@ ... @@` hunks and context lines.

### 4. Update CLI handler to print diff in `--check` mode (`src/main.rs`)

In the `check && result.changed` branch (line 960-962), after printing "Would reformat:", call the diff function and print the output to stderr.

No new `--diff` flag needed — the diff is always shown when `--check` detects changes. This matches the behavior described in the issue.

### 5. Update CLI tests (`tests/cli.rs`)

Update `fmt_check_unformatted_exits_nonzero` to also assert that the output contains unified diff markers (`---`, `+++`, `@@`).

Add a new test `fmt_check_shows_diff_content` that verifies the diff contains the actual changed lines (e.g., key reordering).

## Files to modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add `similar = "2"` |
| `src/formatter.rs` | Add `original` field to `FormatResult`, add `diff_skill()` function |
| `src/main.rs` | Print diff in `--check` mode |
| `tests/cli.rs` | Update and add format check tests |

## Verification

```bash
cargo test                          # all tests pass
cargo clippy -- -D warnings         # no warnings
aigent format --check my-skill/     # shows diff output when unformatted
aigent format --check my-skill/     # no output when already formatted
```
