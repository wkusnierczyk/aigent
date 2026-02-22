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
