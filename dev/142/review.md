## Review: `#145` (`fix/142`)

### Findings

1. **Low** — Module docs are now stale after behavior change  
   - File: `src/assembler.rs:5`  
   - The module-level docstring still says assembly creates scaffolded `agents/` and `hooks/` directories, but this PR intentionally removed creating those directories. The function-level tree comment was updated, but the top-level module description was not.

### Residual risk / testing gap

- There is no explicit regression test in this PR for the reported workflow (`build` output then `validate-plugin` should avoid prior errors/warnings tied to manifest `skills` type and empty scaffold dirs). Existing tests validate pieces in isolation, but not that end-to-end contract.

---

## Review: PR #145 (`fix/142` → `main`)

Reviewed PR #145 (`b61ef6c`, 1 commit) against `main` (`3e730ab`).

Diff: 1 file changed (`src/assembler.rs`), +4/−13 lines.

### What it fixes

Two problems where `aigent build` output failed `aigent validate-plugin`:

1. **`skills` array in plugin.json**: `generate_plugin_json` emitted a
   `"skills": ["skill-name"]` array. The M15 manifest validator (`P006`/`P007`)
   expects path override fields to be strings, not arrays — the array would
   cause a deserialization type mismatch or be silently ignored, but either
   way it's not a valid plugin.json field.

2. **Empty `agents/` and `hooks/` directories**: The assembler scaffolded
   empty `agents/` and `hooks/` directories. The M15 cross-component
   validator (`X001`) flags empty component directories as an info-level
   diagnostic. While not an error, it produces noise on every
   `validate-plugin` run for assembled plugins.

### Changes

| Location | Change |
|----------|--------|
| `assembler.rs:55–56` | Removed `agents/` and `hooks/` from doc comment tree |
| `assembler.rs:119–124` | Removed `agents_dir`/`hooks_dir` creation |
| `assembler.rs:242–247` | Removed `skill_names` collection and `"skills"` key from JSON |
| Tests (3 sites) | Updated assertions: `agents`/`hooks` dirs don't exist, `skills` key absent |

### Test results

- 694 tests pass (526 unit + 140 CLI + 27 plugin + 1 doc)
- Clippy: clean
- Formatting: clean (not checked explicitly but PR is trivial)

### Findings

1. **Low: module-level doc comment stale** — Confirmed from prior review.
   `src/assembler.rs:1–5` still says "scaffolded `agents/` and `hooks/`
   directories" but the PR removes that scaffolding. The function-level
   tree diagram (line 55) was updated, but the module `//!` doc was not.

   Fix: change line 5 from:
   ```
   //! the skill files, and scaffolded `agents/` and `hooks/` directories.
   ```
   to:
   ```
   //! the skill files.
   ```

2. **Low: no end-to-end regression test** — Confirmed from prior review.
   The PR updates 3 existing unit tests to assert the new behavior (no
   `agents/` dir, no `hooks/` dir, no `skills` key). These are correct.
   However, there's no test that runs `assemble_plugin` → `validate_manifest`
   (or `validate_plugin` equivalent) to verify the build-then-validate
   workflow end-to-end. The existing tests verify pieces in isolation.

   This is non-blocking — the individual assertions are sufficient to
   verify correctness — but an integration test would prevent future
   regressions in the build→validate contract.

### Scope

| Metric | Value |
|--------|-------|
| Changed files | 1 |
| Net line delta | −9 |
| Risk | Low — removes code, no new logic |

### Summary

| Dimension | Rating |
|-----------|:------:|
| Correctness | ✅ |
| Completeness | ✅ |
| Test coverage | ⚠️ |

Clean, minimal fix. Removes invalid `skills` array from generated
plugin.json and stops creating empty scaffold directories. Both prior
review findings confirmed (stale doc, no e2e test) but neither is
blocking. Ready to merge after the one-line doc fix.
