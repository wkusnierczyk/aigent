## Review: `fix/143`

### Findings

1. **Medium** — New regression test checks the wrong stream and can pass while the bug still exists  
   - File: `tests/cli.rs:1389`  
   - `score` text output is emitted to **stderr** (`src/main.rs:668`), but `upgrade_apply_does_not_regress_score` inspects `output.stdout` only. If "Unknown fields found" is printed to stderr (the actual path), this test still passes. It also does not assert the `score` command status.  
   - Impact: The added test does not reliably protect against the intended regression.

2. **Low** — User-facing docs now describe upgrade behavior that no longer exists  
   - File: `README.md:1005`  
   - `run_upgrade` no longer suggests/applies `metadata.version` and `metadata.author` (`src/main.rs:1495-1531`), but the README examples still show those suggestions.

### Residual risk

- This change removes metadata upgrade logic and multiple metadata-related tests, but does not add a targeted assertion that `upgrade --apply` leaves existing `metadata:` blocks untouched. That increases risk of future accidental reintroduction without coverage.

---

## Review: Branch `fix/143` (implementation)

Reviewed `fix/143` (`b81368a`, 1 commit) against `main` (`2c2309d`).

Diff: 2 files changed, +15/−200 lines.

### What it fixes

Issue #143: `upgrade --apply` added non-spec fields (`metadata.version`,
`metadata.author`) that the validator flags as unknown, causing the score to
drop after upgrade. The fix removes the metadata suggestion/apply logic
entirely, along with the now-dead helper functions (`detect_indent`,
`find_metadata_insert_position`).

### Changes

| Location | Change |
|----------|--------|
| `src/main.rs:1435–1437` | Stale doc comment lines left behind (see finding 1) |
| `src/main.rs:1438–1493` (old) | Removed `detect_indent` function (12 lines) |
| `src/main.rs:1454–1493` (old) | Removed `find_metadata_insert_position` function (40 lines) |
| `src/main.rs:1552–1588` (old) | Removed `metadata.version` / `metadata.author` checks |
| `src/main.rs:1618–1654` (old) | Removed metadata `--apply` insertion logic |
| `tests/cli.rs:1318` | `upgrade_clean_skill_no_suggestions` — removed metadata from "clean" skill |
| `tests/cli.rs:1354` | `upgrade_full_reports_suggestions` — removed metadata assertion |
| `tests/cli.rs:1373–1462` (old) | Removed 5 metadata tests (partial meta, 4-space indent, comments, no meta block, preserves keys) |
| `tests/cli.rs:1373` | Added `upgrade_apply_does_not_regress_score` regression test |

### Findings

1. **Medium: stale doc comment on `run_upgrade`** — The deletion removed
   `detect_indent`'s function body and the last 2 lines of its doc comment,
   but left the first 3 lines (`src/main.rs:1435–1437`):

   ```rust
   /// Detect the indentation style used in frontmatter lines.
   ///
   /// Scans for the first indented line and returns its leading spaces.
   /// Run upgrade analysis on a skill directory.       // ← actual run_upgrade doc starts here
   ```

   These now appear as the opening paragraph of `run_upgrade`'s doc comment,
   making `rustdoc` incorrectly describe `run_upgrade` as detecting
   "indentation style". The fix is to delete lines 1435–1437.

2. **Medium: regression test checks wrong stream** — Confirmed from prior
   review. `score` text output goes to **stderr** via `eprint!`
   (`src/main.rs:669`), but the test reads `output.stdout`:

   ```rust
   let stdout = String::from_utf8_lossy(&output.stdout);
   assert!(!stdout.contains("Unknown fields found"), ...);
   ```

   Since stdout is empty for text-format score, the assertion passes
   vacuously. The test does not protect against the intended regression.

   **Fix**: Change to `output.stderr`:
   ```rust
   let stderr = String::from_utf8_lossy(&output.stderr);
   assert!(!stderr.contains("Unknown fields found"), ...);
   ```

3. **Low: README examples still show removed suggestions** — Confirmed from
   prior review. `README.md:1005–1006` still lists `metadata.version` and
   `metadata.author` as upgrade suggestions, but the code no longer emits
   them. `CHANGES.md:123` also references them.

### Validation performed

- `git diff main..origin/fix/143` — full diff reviewed (215 lines)
- No checkout performed; verified removed functions have zero remaining
  references on the branch
- Confirmed `detect_indent` and `find_metadata_insert_position` are not
  called anywhere else

### Scope

| Metric | Value |
|--------|-------|
| Changed files | 2 |
| Net line delta | −185 |
| Risk | Low — removes code, no new logic |

### Summary

| Dimension | Rating |
|-----------|:------:|
| Correctness | ⚠️ |
| Completeness | ⚠️ |
| Test coverage | ⚠️ |

Clean removal of non-spec metadata logic. Two medium issues: (1) stale doc
comment from incomplete deletion makes `run_upgrade` documentation wrong,
(2) regression test checks stdout but score output goes to stderr, making
the assertion vacuously true. Both are straightforward fixes. The README/
CHANGES.md staleness is low priority but should be addressed before release.
