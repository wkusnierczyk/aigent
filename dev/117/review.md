# Review: Add `release` command to `version.sh` (#117)

Review of `dev/117/plan.md` against `scripts/version.sh` on `main`
(`8884129`).

---

## Verdict

The plan is well-structured and the step sequence is sound. Three
issues need attention before execution: a changelog replacement bug
(§3.1), a missing commit push (§3.2), and the known CHANGES.md sed
portability issue (§3.3). The design is otherwise clean.

| Dimension | Rating | Notes |
|-----------|:------:|-------|
| Accuracy | ✅ | `cmd_set` interaction correctly described |
| Completeness | ⚠️ | Missing `git push` for the commit (§3.2) |
| Design | ✅ | Clean separation: `cmd_set` stubs, `cmd_release` replaces |
| Risk | Medium | Destructive (pushes tag, triggers CI release) |

---

## 1. Code Location Accuracy

| Claim | Status | Notes |
|-------|:------:|-------|
| `cmd_set` inserts CHANGES.md stub | ✅ | Lines 135–151, uses `sedi` |
| `current_version` reads Cargo.toml | ✅ | Lines 39–41 |
| `cmd_bump` resolves semver arithmetic | ✅ | Lines 172–202 |
| Main dispatch at bottom of file | ✅ | Lines 204–229 |
| README release section at line ~1327 | ✅ | Manual 4-step sequence |

---

## 2. Design Review

### 2.1 Step sequence

The 9-step sequence (preflight → version → changelog → set → write →
stage → commit → tag → push) is logically ordered. Key properties:

- **`cmd_set` before changelog write**: Correct — `cmd_set` creates
  the stub, then `cmd_release` replaces the stub body. This keeps
  `cmd_set` unmodified.
- **Dirty-tree check excludes managed files**: Correct — the 5 files
  that `cmd_set` modifies are excluded from the dirty check.
- **`set -e` for error handling**: Appropriate for a linear script.
  No rollback needed — each step is individually reversible.

### 2.2 Dry-run mode

The `run()` helper is only used conceptually in the plan — the actual
`cmd_release` function has an early return after generating the
changelog in dry-run mode. This is the right approach for this script
(the `run()` wrapper pattern works for simple commands but not for
multi-step logic with intermediate state).

### 2.3 Dirty-tree check

The `git status --porcelain -- . ":!Cargo.toml" ...` pattern is
correct and portable. The exclusion list matches the 5 files managed
by `cmd_set`.

### 2.4 Changelog generation via `gh`

Using `gh pr list --state merged --search "merged:>=$SINCE"` is the
right approach. The `--jq` formatting produces clean entries. The
guard against zero PRs is important and correctly implemented.

### 2.5 Bump-level resolution in `cmd_release`

The plan duplicates the bump arithmetic from `cmd_bump`. This is
acceptable — `cmd_release` needs the resolved version before calling
`cmd_set`, and calling `cmd_bump` directly would trigger `cmd_set`
without the changelog step in between. The duplication is minor
(~10 lines).

---

## 3. Issues

### 3.1 CHANGES.md stub replacement with awk (MEDIUM)

```bash
awk -v ver="$VERSION_ESCAPED" -v changelog="$CHANGELOG" '
    $0 ~ "^## \\[" ver "\\]" { print; getline; sub(/_No changes yet\._/, changelog); print; next }
    { print }
' "$CHANGES" > "$TMPFILE"
```

Two problems:

**a) Multi-line changelog in awk variable.** The `$CHANGELOG` variable
contains multiple lines (one per PR). Passing it as `-v changelog=...`
works in most awk implementations, but the `sub()` function replaces
with a literal string, and embedded newlines in the replacement may
behave differently across awk versions. GNU awk handles it; BSD awk
(macOS) may not.

**b) The stub may not be on the next line.** `cmd_set`'s CHANGES.md
insertion (line 141–143) uses `sedi` with `\n` in the replacement,
which has the BSD portability bug flagged in the M14 review. If that
bug is fixed (via the `dev/118-version` branch), the stub structure
will be:

```
## [0.5.0] — 2026-02-22

_No changes yet._
```

The awk script does `getline` to read the *next* line after the heading,
but there's a blank line between the heading and the stub text. The
`getline` would consume the blank line, and the `sub()` would try to
replace `_No changes yet._` in an empty line — which would fail
silently, leaving the stub unchanged.

**Fix:** Use a state-machine approach: after matching the heading,
skip blank lines, then replace the stub line. Or use `sed`/temp-file
to replace the stub text directly without relying on line adjacency.

### 3.2 Missing `git push` for the commit (MEDIUM)

The step sequence does:

```
6. Commit
7. Tag
8. Push tag
```

Step 8 pushes only the tag (`git push origin v0.5.0`), not the commit.
The commit with the version bump and changelog sits on the local branch
but is never pushed. The CI release workflow will be triggered by the
tag, but the tag points to a commit that doesn't exist on the remote.

Depending on git configuration:
- If `git push origin v0.5.0` also pushes the commit the tag points
  to (git's default behavior for annotated tags, but **not** for
  lightweight tags), it works.
- `git tag v0.5.0` creates a lightweight tag, so the commit may
  not be pushed.

**Fix:** Add `git push origin HEAD` before or alongside the tag push,
or use `git push origin HEAD v$VERSION` to push both. Alternatively,
use annotated tags: `git tag -a v$VERSION -m "Release v$VERSION"`.

### 3.3 Inherited CHANGES.md sed portability bug

The `cmd_set` function (line 143) uses `\n` in `sedi` replacement,
which doesn't work on BSD sed. The `dev/118-version` branch has a
fix for this. The `release` command depends on `cmd_set` producing a
correct stub, so this fix is a prerequisite.

---

## 4. Edge Cases

### 4.1 Re-releasing the same version

If `cmd_set` is called with a version that already has a CHANGES.md
entry, it skips the stub insertion ("already has entry for"). Then
`cmd_release`'s awk script would look for `_No changes yet._` but
not find it (the entry already has real content from a previous
release attempt). The replacement would silently fail, preserving the
existing content. This is actually the correct behavior — but the
script doesn't warn about it.

### 4.2 First release (no previous tag)

`generate_changelog` aborts if `git describe --tags` finds no previous
tag. This is correct — the first release should have a manually written
changelog. But the error message could suggest `version.sh set` as
the alternative path.

### 4.3 Tag already exists

If `v0.5.0` already exists as a tag, `git tag v0.5.0` fails. The
script aborts via `set -e`. The user would need to `git tag -d v0.5.0`
first. Consider checking for tag existence in preflight.

---

## 5. Scope

| Metric | Estimate |
|--------|----------|
| New dependencies | 0 (uses existing `gh`) |
| Modified files | 2 (`version.sh`, `README.md`) |
| New functions | 3 (`check_clean_tree`, `generate_changelog`, `cmd_release`) |
| Net line delta | +80–100 |
| Risk | Medium — pushes tags, triggers CI |

---

## 6. Summary

| Dimension | Rating |
|-----------|:------:|
| Accuracy | ✅ |
| Completeness | ⚠️ |
| Design | ✅ |
| Risk | Medium |

**Action items before execution:**

| Priority | Item | Effort |
|----------|------|--------|
| **Must fix** | §3.1: Awk stub replacement — handle blank line between heading and stub | 15 min |
| **Must fix** | §3.2: Push the commit, not just the tag | 1 min |
| **Must fix** | §3.3: Depends on `dev/118-version` fix for CHANGES.md sed portability | Blocked |
| Should fix | §4.3: Check for existing tag in preflight | 5 min |
| Nice to have | §4.1: Warn when stub text not found in CHANGES.md | 5 min |

---

## Review: Branch `dev/117-release`

Reviewed `dev/117-release` (`c0f3230`) against `main` (`8884129`).

### Design: changelog-first strategy

The implementation significantly redesigns the step sequence from plan v1.
Instead of the original plan's approach (call `cmd_set` to create a stub
entry in CHANGES.md, then replace the stub body with awk), the
implementation writes the full CHANGES.md entry *before* calling `cmd_set`:

```
write_changelog()  →  cmd_set()  →  stage  →  commit  →  tag  →  push
```

When `cmd_set` runs, it finds the version entry already exists in CHANGES.md
(`grep -q "## [$VERSION]"` matches) and skips stub insertion entirely. This
eliminates the fragile awk stub-replacement mechanism from plan v1 and
sidesteps the BSD sed portability issue (§3.3), since the buggy `sedi`
code path for CHANGES.md is never exercised during release.

`write_changelog()` itself uses portable `echo`/`tail`/`mktemp`/`mv`
(`scripts/version.sh:242–254`) — no awk, no sed, no `\n` in replacements.

This is a better design than plan v1.

### Plan review issue resolution

| Plan issue | Status | How resolved |
|-----------|:------:|-------------|
| §3.1: Awk stub replacement bug | **Eliminated** | Changelog-first strategy bypasses stub mechanism |
| §3.2: Missing `git push` for commit | **Fixed** | `git push origin HEAD "v$VERSION"` (line 333) |
| §3.3: Inherited sed portability | **Sidestepped** | `cmd_set` skips CHANGES.md write when entry exists |
| §4.3: Tag existence check | **Fixed** | `git rev-parse "v$VERSION"` in preflight (line 293) |
| §4.1: Re-release warning | **Fixed** | `write_changelog()` warns and returns (line 237) |

All five plan review action items are addressed.

### Findings

1. **Medium: managed files dirty check gap** — `check_clean_tree()` explicitly
   ignores `Cargo.toml`, `Cargo.lock`, `.claude-plugin/plugin.json`, `README.md`,
   and `CHANGES.md` when checking working tree cleanliness
   (`scripts/version.sh:176–180`). Later, `cmd_release()` stages those same
   files unconditionally (`scripts/version.sh:323–324`). If any of these files
   contain unrelated local edits before running `release`, those edits get swept
   into the release commit without warning.

   Recommended fix (pick one):
   - Require managed files to be clean too (remove exclusions from
     `check_clean_tree`, rely on the user having committed/stashed first)
   - Snapshot the managed files' git blobs before release, diff after
     `cmd_set`, and warn/abort if unexpected changes appear
   - Add `--allow-dirty` flag for explicit opt-in

2. **Low: `--dry-run` globally parsed** — The `--dry-run` flag is stripped
   from `$@` at the top level (lines 374–383), before subcommand dispatch.
   This means `./scripts/version.sh bump patch --dry-run` is silently
   accepted — `DRY_RUN` is set to 1 but `cmd_bump` never checks it. Not
   harmful, but may surprise users who expect dry-run on `bump` or `set`.

   Fix: either move `--dry-run` parsing into `cmd_release` only, or
   check `DRY_RUN` in the `release)` dispatch case instead.

3. **Low: `write_changelog` assumes fixed CHANGES.md header** — `tail -n +3`
   skips exactly lines 1–2 ("`# Changes`" + blank line). If CHANGES.md
   has a different structure (e.g., no blank line after header, or extra
   preamble), the splice is wrong. Acceptable since the project controls the
   format, but fragile if the file is manually edited.

### Code quality

- **Bump arithmetic**: duplicated from `cmd_bump` (~10 lines, lines 264–274
  vs 340–370). Acceptable per plan review §2.5 — `cmd_release` needs the
  resolved version before calling `cmd_set`.
- **Semver validation**: present in both `cmd_release` (line 282) and `cmd_set`
  (line 54) — defense in depth, correct.
- **Error handling**: `set -euo pipefail` + explicit guards (`command -v gh`,
  empty `PREV_TAG`, empty `ENTRIES`, existing tag). All abort paths print
  clear error messages to stderr.
- **`gh` usage**: `gh repo view --json nameWithOwner` avoids hardcoding the
  repo name. `--base main` filters to correct branch.
- **Dry-run**: early return after changelog generation (line 311), before
  any file modifications. Shows the exact commands that would run.

### README changes

The release workflow section (lines 1326–1358) correctly replaces the manual
4-step process with the single `release` command. Includes:
- Explicit version and bump-level examples
- Numbered step list explaining what the command does
- `--dry-run` example
- `gh` CLI prerequisite note
- Table of CI release steps preserved below

### Validation performed

- Inspected all changes in:
  - `scripts/version.sh` (+189 lines: 3 new functions + dispatch + `--dry-run` parser)
  - `README.md` release workflow section (+29/−8 lines)
- Ran checks:
  - `bash -n scripts/version.sh` — passes
  - `git show origin/dev/117-release:scripts/version.sh | bash -n` — passes
  - `./scripts/version.sh show` — prints current version
  - Full `--dry-run` path not testable (requires `gh` API access for
    changelog generation)

### Scope

| Metric | Plan estimate | Actual |
|--------|:----:|:------:|
| Modified files | 2 | 2 (+plan/review docs) |
| New functions | 3 | 4 (`check_clean_tree`, `generate_changelog`, `write_changelog`, `cmd_release`) |
| Net line delta | +80–100 | +189 (version.sh) +29 (README) |

### Summary

The changelog-first design is a significant improvement over plan v1. All
five plan review action items are addressed. One medium issue (managed files
dirty check gap) should be fixed before merge — it's a release-integrity
risk where unrelated local edits to version-managed files could be silently
committed. The two low-severity items are non-blocking.

---

## Review round 2: Copilot findings + review.md issues

### Issues addressed

| Source | Issue | Fix |
|--------|-------|-----|
| Copilot #3 | Tag check via `git rev-parse` can match branches | Use `--verify --quiet "refs/tags/v$VERSION"` |
| Copilot #4 | Author time `%aI` unstable under rebases | Switched to committer time `%cI` |
| Copilot #5 | `grep -q` treats version dots as regex wildcards | Switched to `grep -Fq` (also fixed same pattern in `cmd_set`) |
| Copilot #1 | No branch/freshness check | Added preflight: verify on `main`, fetch + compare with `origin/main` |
| Review §2 | `--dry-run` globally parsed, silently accepted on non-release commands | Moved parsing into `cmd_release` only; dispatcher passes `"$@"` via `shift` |

### Issues not addressed (by design)

| Source | Issue | Rationale |
|--------|-------|-----------|
| Copilot #2 | `git push origin HEAD` might push to literal `HEAD` ref | Incorrect — `HEAD` is resolved by git to the current branch. `git push origin HEAD v$VERSION` pushes both the branch and the tag. This is documented git behavior. |
| Review §1 | Managed files dirty check gap | The new branch check (must be on `main`, up-to-date with `origin/main`) significantly reduces this risk — if you're on a clean, synced `main`, the managed files should match their committed state. Remaining edge case (manual edits to managed files without committing) is accepted. |
| Review §3 | `write_changelog` assumes fixed CHANGES.md header | Acceptable — project controls the format. |
