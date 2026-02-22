# Review: Default to current directory (#116)

Review of `dev/116/plan.md` against the codebase at `main` (`8884129`).

---

## Verdict

The plan is mostly correct but has one design issue with `Probe` that
needs resolution before execution (§3.1). The clap `default_value`
approach is the right one. Command inventory is accurate.

| Dimension | Rating | Notes |
|-----------|:------:|-------|
| Accuracy | ⚠️ | `Probe` has a positional conflict (§3.1) |
| Completeness | ✅ | All 11 commands correctly identified |
| Design | ✅ | `default_value = "."` is idiomatic clap |
| Risk | Low | Pure clap attribute changes, no logic changes |

---

## 1. Code Location Accuracy

| Claim | Status | Notes |
|-------|:------:|-------|
| `skill_dirs: Vec<PathBuf>` in 7 commands | ✅ | Validate, Check, Prompt, Doc, Build, Test, Fmt |
| `skill_dir: PathBuf` in 4 commands | ✅ | Properties, Score, Probe, Upgrade |
| `New` takes `purpose`, not skill dir | ✅ | `purpose: String` positional |
| `Init` already has `Option<PathBuf>` | ✅ | `dir: Option<PathBuf>`, defaults to `"."` at runtime |

---

## 2. Design Review

### 2.1 `default_value = "."` for `Vec<PathBuf>` commands

This is correct clap behavior. When a positional `Vec<T>` has
`default_value`, clap uses the default when no positional arguments are
provided. When one or more are provided, the default is not included.
This is the expected UX.

### 2.2 `default_value = "."` for single `PathBuf` commands

Also correct. The `#[arg(name = "skill-dir")]` attribute is already
present on these fields. Adding `default_value = "."` makes the
argument optional.

---

## 3. Issues

### 3.1 `Probe` command: positional argument ambiguity (MEDIUM)

`Probe` has two positional arguments:

```rust
Probe {
    #[arg(name = "skill-dir")]
    skill_dir: PathBuf,
    #[arg(name = "query")]
    query: String,
    // ...
}
```

If `skill_dir` gets `default_value = "."`, then `aigent probe "hello"`
becomes ambiguous: is `"hello"` the skill dir or the query? Clap resolves
positionals left-to-right, so `"hello"` would fill `skill_dir` (overriding
the default), and `query` would be missing — a parse error.

The intended usage is `aigent probe "hello"` meaning "probe the current
directory with query `hello`". But clap would interpret it as
`skill_dir = "hello"`, `query` = missing.

**Options:**

1. Make `skill_dir` a named argument in `Probe`:
   `#[arg(long, default_value = ".")]` — then `aigent probe "hello"`
   works, and `aigent probe --skill-dir other/ "hello"` is explicit.

2. Swap the positional order (query first, skill_dir second with default)
   — then `aigent probe "hello"` fills query, skill_dir defaults to `.`.
   But this changes the existing CLI contract.

3. Skip `Probe` for this change — it already requires both arguments,
   so a default on `skill_dir` only helps if query is also made optional
   (which it shouldn't be).

**Recommendation:** Option 3 (skip `Probe`). Defaulting the skill dir
only matters when the user can actually omit it, and `Probe` always
needs a query. The plan should list `Probe` under "Commands NOT
affected" with the reason: "has a required positional `query` after
`skill_dir`; defaulting would create positional ambiguity."

This reduces the single-path commands to: `Properties`, `Score`,
`Upgrade` (3 not 4).

---

## 4. Test Coverage

### 4.1 Planned tests

The plan proposes testing:
- `validate` with no args
- `properties` with no args
- `fmt --check` with no args

These are good choices — they cover both multi-path and single-path
patterns. The tests need to set the working directory to a skill
directory using `Command::current_dir()` in `assert_cmd`.

### 4.2 Suggested additional tests

| Test | Why |
|------|-----|
| `score` with no args | Single-path pattern |
| `validate` with explicit path still works | Regression: verify default doesn't break explicit usage |
| `check` with no args | Exercises the `--no-validate` + default dir path |
| Multi-path: `validate a/ b/` still works | Verify default isn't prepended to explicit args |

The last test is important: with `default_value` on a `Vec`, clap should
*replace* the default when explicit args are given, not *prepend* `.` to
the list. This is standard clap behavior but worth a regression test.

---

## 5. README Assessment

The plan mentions updating the CLI reference. Specific places to update:

- Command usage lines (e.g., `aigent validate <skill-dir>...` →
  `aigent validate [<skill-dir>...]`)
- The quick start section if it shows explicit `.` usage that can be
  simplified
- Consider adding a note: "When no path is given, the current directory
  is used."

---

## 6. Scope

| Metric | Estimate |
|--------|----------|
| New dependencies | 0 |
| Modified files | 3 (`main.rs`, `tests/cli.rs`, `README.md`) |
| Net line delta | +20–40 (mostly tests) |
| Risk | Low — pure attribute changes |

---

## 7. Summary

| Dimension | Rating |
|-----------|:------:|
| Accuracy | ⚠️ |
| Completeness | ✅ |
| Design | ✅ |
| Risk | Low |

**Action required:** Resolve the `Probe` positional ambiguity (§3.1)
before execution. Recommended: skip `Probe`, apply `default_value`
to the remaining 10 commands (7 multi-path + 3 single-path).

---
---

# PR #125: Code Review — Implementation Review

Review of PR #125 (`dev/116-default-dir` → `main`, 7 commits,
`c4d9ba6`).

---

## Verdict

The implementation goes beyond the plan: rather than skipping `probe`
(plan review §3.1, option 3), it redesigns `probe` with `--query`/`-q`
flag and adds multi-dir ranked output. This is a better outcome than
the plan anticipated. The default-directory feature for the other 10
commands is clean. One issue found in the `probe` redesign (§2.2).

| Dimension | Rating | Notes |
|-----------|:------:|-------|
| Correctness | ✅ | 557 tests pass per PR description |
| Plan adherence | ✅+ | Exceeds plan — `probe` redesigned instead of skipped |
| Review feedback | ✅ | Probe ambiguity resolved; all suggested tests adopted |
| Test coverage | ✅ | 9 new tests |
| Breaking changes | ⚠️ | `probe` query is now `--query`/`-q` (was positional) |

---

## 1. Default Directory (10 commands)

All 10 commands correctly have `#[arg(default_value = ".")]`. Doc
comments updated with `[default: .]`. Verified in diff:

| Command | Field | Status |
|---------|-------|:------:|
| `Validate` | `skill_dirs: Vec<PathBuf>` | ✅ |
| `Check` | `skill_dirs: Vec<PathBuf>` | ✅ |
| `Properties` | `skill_dir: PathBuf` | ✅ |
| `Prompt` | `skill_dirs: Vec<PathBuf>` | ✅ |
| `Score` | `skill_dir: PathBuf` | ✅ |
| `Doc` | `skill_dirs: Vec<PathBuf>` | ✅ |
| `Build` | `skill_dirs: Vec<PathBuf>` | ✅ |
| `Test` | `skill_dirs: Vec<PathBuf>` | ✅ |
| `Upgrade` | `skill_dir: PathBuf` | ✅ |
| `Fmt` | `skill_dirs: Vec<PathBuf>` | ✅ |

`New` and `Init` correctly not affected.

---

## 2. Probe Redesign

### 2.1 Design choice: `--query`/`-q` flag + multi-dir

Instead of skipping `probe` (plan review §3.1 option 3), the PR chose
a variant of option 1 (plan review §3.1 option B): move query to a
named flag. This is the right call — it resolves the positional
ambiguity and enables multi-dir support as a natural consequence.

The `probe` command now:
- Takes `skill_dirs: Vec<PathBuf>` with `default_value = "."`
- Takes `query: String` as `#[arg(long, short)]`
- Sorts results by score descending (best first)
- JSON output: single object for 1 result, array for multiple

### 2.2 `probe` exit code behavior (LOW)

```rust
if had_errors && results.is_empty() {
    std::process::exit(1);
}
```

This exits 1 only when *all* directories failed (errors and no results).
If some succeed and some fail, it exits 0. This is reasonable for
multi-dir but is a behavior change from the old single-dir code, which
exited 1 on any error. The README correctly documents the new behavior:
"At least one result printed" = exit 0.

However, there's no exit code distinction between "all results had
score 0" and "all results had strong matches." The old behavior was the
same (always exit 0 on success), so this is not a regression — just
noting it.

### 2.3 JSON output: single vs array

```rust
if json.len() == 1 {
    println!("{}", serde_json::to_string_pretty(&json[0]).unwrap());
} else {
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
```

Single-result JSON outputs an object; multi-result outputs an array.
This preserves backward compatibility for existing single-dir JSON
consumers. Good decision — but downstream parsers should be aware
that the type depends on result count.

An alternative would be to always return an array (even for 1 result),
which is simpler for consumers. The current approach prioritizes
backward compatibility.

### 2.4 Score field added to JSON

The JSON output now includes `"score"` — this was missing from the
old implementation. Good addition for multi-dir ranked output.

---

## 3. Breaking Change Assessment

The `probe` positional-to-flag change is breaking:

```
# Before
aigent probe my-skill/ "validate a skill"

# After
aigent probe my-skill/ --query "validate a skill"
```

All existing probe tests updated to use `--query`. The backward-compat
alias table in README removes `test <dir> <query>` → `probe` (the old
`test` alias was already redirected to the fixture runner in M13).

This is acceptable for a pre-1.0 tool. The PR clearly documents it.

---

## 4. Test Coverage

### 4.1 Default directory tests

| Test | Pattern | Status |
|------|---------|:------:|
| `validate_defaults_to_current_dir` | Vec, `current_dir` | ✅ |
| `validate_explicit_path_still_works` | Vec, regression | ✅ |
| `properties_defaults_to_current_dir` | Single, `current_dir` | ✅ |
| `score_defaults_to_current_dir` | Single, `current_dir` | ✅ |
| `fmt_check_defaults_to_current_dir` | Vec, `current_dir` | ✅ |
| `check_defaults_to_current_dir` | Vec, `current_dir` | ✅ |

All use `current_dir()` — correct. Both Vec and single-path patterns
covered. Explicit-path regression test included (plan review §4.2).

### 4.2 Probe tests

| Test | What it verifies |
|------|------------------|
| `probe_defaults_to_current_dir` | Default dir with `--query` |
| `probe_multiple_dirs_ranked` | Multi-dir, both skills appear |
| `probe_skill_shows_activation_status` (updated) | `--query` flag |
| `probe_skill_no_match_query` (updated) | `--query` flag |
| `probe_skill_json_format` (updated) | `--query` flag |
| `probe_skill_missing_dir_exits_nonzero` (updated) | `--query` flag |
| `probe_command_shows_activation` (updated) | `--query` flag |

### 4.3 Updated test: `doc_no_args`

```rust
fn doc_no_args_defaults_to_current_dir() {
    aigent().arg("doc").assert().success()
        .stderr(predicate::str::contains("cannot read skill properties"));
}
```

Previously tested that `doc` with no args exited non-zero. Now it
defaults to `.`, which succeeds but warns (current dir has no
SKILL.md). This correctly reflects the new behavior.

### 4.4 Missing tests (non-blocking)

- `probe` with `--query` but no dirs and no SKILL.md in cwd → should
  error gracefully
- Multi-dir with `--format json` → verify array output
- `probe` ranking order verified (test only checks both names appear,
  not order)

---

## 5. README Updates

- Commands table: all `<dirs...>` → `[dirs...]`, `<directory>` →
  `[directory]` — correct
- Quick start: adds `cd my-skill/ && aigent validate` example — good
- Default directory note added after commands table — clear, includes
  "does not search parent directories" caveat
- Probe section: updated examples, flags table added, multi-dir
  ranked examples
- Exit codes: `probe` updated from "Always" to "At least one result"
- Stale `test <dir> <query>` alias removed — correct

---

## 6. Scope

| Metric | Plan | Actual |
|--------|:----:|:------:|
| Modified files | 3 | 3 (+plan/review docs) |
| Net code delta | +20–40 | +216 (incl. probe redesign + tests) |
| New tests | 3–7 | 9 |
| Breaking changes | 0 | 1 (`probe` query) |

The scope exceeds the plan due to the probe redesign, which was not
planned but resolves the ambiguity better than skipping.

---

## 7. Summary

| Dimension | Rating |
|-----------|:------:|
| Correctness | ✅ |
| Completeness | ✅ |
| Design | ✅ |
| Breaking change risk | Low (pre-1.0, documented) |

No blocking issues. The probe redesign is a clean resolution of the
plan review's §3.1 concern. Ready to merge.

---

## Review: PR #125 (`origin/dev/116-default-dir`)

Reviewed against `main` (`8884129`), focusing on default-path clap behavior,
`probe` redesign semantics, and CLI/docs consistency.

### Findings

No blocking defects found.

### Validation performed

- Inspected code/documentation deltas in:
  - `src/main.rs`
  - `tests/cli.rs`
  - `README.md`
- Verified clap defaults are applied consistently to all intended commands
  (`default_value = "."` on path positionals).
- Verified `probe` redesign:
  - positional query replaced by required `--query` / `-q`
  - multi-directory probing supported
  - ranked output (descending score) in text/JSON modes
  - partial-failure exit behavior matches docs (`exit 1` only when all dirs fail)
- Ran targeted tests:
  - `cargo test --test cli defaults_to_current_dir --quiet` (7/7 pass)
  - `cargo test --test cli probe_ --quiet` (7/7 pass)

### Residual risks / gaps

- I did not run the full test suite or clippy in this pass.
- Multi-dir JSON probe test currently validates presence/shape indirectly;
  exact ordering assertions are still limited to behavior-by-inspection.
