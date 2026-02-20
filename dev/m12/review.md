# M12: Ecosystem & Workflow — Plan Review

## Overall Assessment

M12 is the "ecosystem layer" — 9 issues, 3 waves, 9 agents, covering quality
scoring, skill testing, documentation, conflict detection, watch mode, directory
validation, upgrade, and README improvements. The scope is well-contained
compared to the original M10 (22 issues). The wave decomposition is sound and
dependencies are correctly ordered.

The plan was updated with a Reconciliation section (2026-02-20) confirming that
M11 was absorbed into the M10 PR (#72), meaning all M11 APIs are now available.
The `dev/m11` branch (pushed for PR) contains M10 + M11 in its lineage. Once
merged to `main`, M12 can branch directly.

The plan's two main risks are now: (1) two algorithmically novel features
(tester and conflict detection) that involve text similarity scoring with no
established baseline, and (2) the `notify` dependency adding platform-specific
complexity for a low-priority feature.

## Plan Conformance

### Issues Addressed

- [x] #54 — Directory structure validation (Wave 1, Agent C)
- [x] #59 — Quality scoring (Wave 1, Agent A)
- [x] #61 — Skill upgrade/migration (Wave 3, Agent H)
- [x] #63 — Scorer skill (Wave 1, Agent B)
- [x] #64 — Skill tester (Wave 3, Agent G)
- [x] #67 — Documentation generation (Wave 2, Agent E)
- [x] #68 — Watch mode (Wave 2, Agent F)
- [x] #69 — Cross-skill conflict detection (Wave 2, Agent D)
- [x] #70 — README improvements (Wave 3, Agent I)

All 9 issues accounted for. No issues deferred.

### Issue Deviations

1. **Issue #63 (scorer skill)**: The plan lists `allowed-tools: Bash(aigent *)`
   — the same broad pattern flagged in the M10 plan review (Finding 6). The
   scorer only needs `aigent score` and `aigent validate --lint`. The broad
   `aigent *` pattern allows the skill to invoke `aigent build`, which is
   inappropriate for a read-only assessment tool.

2. **Issue #68 (watch mode)**: The plan says to add `notify = "8"` as a
   non-optional dependency. This was flagged in the M10 plan review (Finding 4)
   as a significant dependency for a low-priority feature. The recommendation
   to gate behind a cargo feature flag was not adopted.

### M11 API Verification

The M12 plan depends on M11 APIs. These have been verified against the actual
`dev/m11` codebase (commit `baba66c`):

| Plan Reference | Actual API | Status |
|---------------|-----------|--------|
| `SkillEntry` struct | `pub struct SkillEntry { name, description, location }` in `src/prompt.rs` | ✅ Matches |
| `collect_skills()` | `pub fn collect_skills(dirs: &[&Path]) -> Vec<SkillEntry>` in `src/prompt.rs` | ✅ Matches |
| `estimate_tokens()` | `pub fn estimate_tokens(s: &str) -> usize` in `src/prompt.rs` | ✅ Matches |
| `to_prompt_format()` | `pub fn to_prompt_format(dirs: &[&Path], format: PromptFormat) -> String` | ✅ Available |
| `template_files()` | `pub fn template_files(template: SkillTemplate, dir_name: &str) -> HashMap<String, String>` | ✅ Available |
| `apply_fixes()` | `pub fn apply_fixes(dir: &Path, diags: &[Diagnostic]) -> Result<usize>` | ✅ Available |
| `PromptFormat` enum | `Xml, Json, Yaml, Markdown` in `src/prompt.rs` | ✅ Matches |
| `format_budget()` | `pub fn format_budget(entries: &[SkillEntry]) -> String` | ✅ Available |
| `discover_skills()` | `pub fn discover_skills(root: &Path) -> Vec<PathBuf>` | ✅ Available |

All M11 APIs that M12 depends on are confirmed to exist with the expected signatures.

### Reconciliation Section Quality

The reconciliation section (lines 524–602) is thorough and accurate:

- Correctly maps all 8 M11 issues to their deliverables and locations
- Identifies stale assumptions (branch strategy, dependency ordering)
- Lists inherited findings from M11 with resolution notes
- Documents additional M10 artifacts reusable by M12 (e.g., `to_title_case()`,
  `resolve_dirs()`, `read_body()`)
- Notes implementation patterns to follow (Format enum, re-exports)

One minor note: the reconciliation says "M10 PR (#72, merged as `2c11167`)"
— this commit exists on `dev/m11` but M10 has not yet merged to `main`. The
`dev/m11` branch is pushed for PR. M12 should branch from `main` *after* the
M11 PR merges (which will bring M10 + M11 together).

## Findings

### Finding 1 (Resolved): M11 API surface verified

**Location**: "Current State (after M11)", Dependencies

Previously flagged as High severity. The M11 APIs referenced in the plan
(`SkillEntry`, `collect_skills()`, `estimate_tokens()`, etc.) have been
verified against the actual `dev/m11` implementation. All types and function
signatures match the plan's assumptions. The `dev/m11` branch is pushed for PR.

**Status**: Resolved — no action needed. The plan's API assumptions are correct.

### Finding 2 (Medium): Skill tester scoring algorithm is underspecified

**Location**: Wave 3, Agent G — steps 2, 4

The plan says the scoring algorithm uses "keyword overlap, trigger phrase match,
name relevance" with a 0.0–1.0 confidence score, but:

- No definition of "keyword overlap" — is it Jaccard similarity? TF-IDF?
  Simple token intersection/union ratio?
- No definition of weights for each component
- No threshold for "likely match" (the output shows "← likely match" but
  doesn't define when this label appears)
- "Name relevance" is undefined — is it substring match? Edit distance?

The tester is described as "the most ambitious item" in the plan, yet its core
algorithm gets 4 bullet points. Without a concrete formula, different
implementations will produce different rankings and confidence scores, making
tests hard to write (what should the expected score be?).

**Recommendation**: Define the scoring formula explicitly. Example:

```
score = 0.5 * jaccard(query_tokens, desc_tokens)
      + 0.3 * trigger_match(query, desc)
      + 0.2 * name_match(query, name)

jaccard(A, B) = |A ∩ B| / |A ∪ B|
trigger_match = 1.0 if query appears in description, 0.0 otherwise
name_match = 1.0 if any query token is a substring of name, 0.0 otherwise
```

This makes tests deterministic and the algorithm auditable.

### Finding 3 (Medium): Directory structure validation S002 is platform-dependent

**Location**: Wave 1, Agent C — step 1

S002 checks for "Script missing execute permission." Unix file permissions don't
exist on Windows. The `std::os::unix::fs::PermissionsExt` trait is
platform-specific:

```rust
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
```

The plan doesn't mention platform gating. If the check compiles
unconditionally, it won't compile on Windows. If gated behind `#[cfg(unix)]`,
the check silently disappears on Windows with no warning.

**Recommendation**: Gate S002 behind `#[cfg(unix)]` and document this
limitation. Add a comment in the code and a note in the diagnostic registry
that S002 is Unix-only.

### Finding 4 (Medium): `upgrade` subcommand overlaps significantly with `validate --apply-fixes`

**Location**: Wave 3, Agent H — Design Decisions

The plan says:

> Difference from `validate --apply-fixes`: upgrade also adds missing
> recommended fields (e.g., `compatibility`, trigger phrases) and restructures
> content following current spec patterns.

But the upgrade logic described in Agent H's steps is:

1. Run `validate()` + `lint()` to identify issues
2. Check for missing recommended fields (`compatibility`)
3. Check for missing trigger phrase in description
4. If `--apply`: use `fixer::apply_fixes()` + add missing fields

Steps 1 and 4 are identical to `validate --apply-fixes --lint`. Steps 2 and 3
are lint checks that could be I006/I007 info diagnostics. The only genuinely
new capability is "add missing fields" — writing new YAML frontmatter fields
that didn't exist before.

This creates user confusion: when should I use `validate --apply-fixes` vs
`upgrade --apply`? The answer ("upgrade adds fields, validate only fixes
existing ones") is subtle.

**Recommendation**: Either (a) merge upgrade into validate with a
`--add-recommended` flag, or (b) clearly document the distinction in `--help`
and README. If (b), make `upgrade` call `validate --apply-fixes` internally
rather than duplicating the pipeline.

### Finding 5 (Medium): Conflict detection's 0.7 word-overlap threshold lacks calibration

**Location**: Wave 2, Agent D — step 1

The plan sets `C002` threshold at "word overlap ratio > 0.7" for flagging
description similarity. This number appears arbitrary — no calibration against
real skill collections is mentioned.

Consider: two skills with descriptions "Processes PDF files for extraction" and
"Processes PDF documents for analysis" have ~5/7 word overlap ≈ 0.71 — just
above threshold. Are these genuinely conflicting? They might serve different
purposes.

Meanwhile, two skills with descriptions "Validates SKILL.md files" and
"Checks skill definitions for errors" have near-zero word overlap but describe
the same functionality.

**Recommendation**: Document that the threshold is a heuristic starting point.
Consider supporting `--threshold` as a flag. Log a TODO for semantic similarity
(embeddings) as a future enhancement once a lightweight embedding model is
available.

### Finding 6 (Medium): Watch mode adds a heavy dependency for a deferrable feature

**Location**: Wave 2, Agent F — Cargo.toml Changes

This was flagged in the M10 plan review (Finding 4) and the recommendation was
to defer or gate behind a feature flag. The plan includes `notify = "8"` as a
non-optional dependency, which:

- Adds platform-specific native file watcher backends
- Pulls in several transitive dependencies (`mio`, `inotify`, `kqueue`,
  `fsevent-sys`, `windows-sys`)
- Increases compile time and binary size
- Requires platform-specific testing

The CLI test plan for watch mode is weak: "Watch flag accepted without error"
and a note that the integration test "may be flaky, consider skipping in CI."

**Recommendation**: Gate `notify` behind a cargo feature flag:
```toml
[features]
watch = ["notify"]
```
```rust
#[cfg(feature = "watch")]
mod watch;
```
This makes the dependency opt-in. Users who don't need watch mode don't pay the
compile-time and binary-size cost.

### Finding 7 (Low): Scorer skill `allowed-tools` too broad (repeat of M10 plan review)

**Location**: Design Decisions, Scorer Skill (#63)

```yaml
allowed-tools: Bash(aigent *), Bash(command -v *), Read, Glob
```

The scorer should only be allowed `Bash(aigent score *), Bash(aigent validate *),
Bash(command -v *)` — not the full `aigent *` wildcard. This was Finding 6 in
the M10 plan review.

**Recommendation**: Use `Bash(aigent score *), Bash(aigent validate *)` instead
of `Bash(aigent *)`.

### Finding 8 (Low): New diagnostic code namespace (`S001–S004`, `C001–C003`) not registered centrally

**Location**: Wave 1 Agent C, Wave 2 Agent D

The plan introduces two new code namespaces:
- `S001–S004` for structure validation (Agent C)
- `C001–C003` for conflict detection (Agent D)

But doesn't specify where these constants are defined. The existing pattern has
E/W codes in `diagnostics.rs` and I codes in `linter.rs`. Two new modules
(`structure.rs`, `conflict.rs`) presumably define their own codes, but this is
not stated.

Maintaining the error code registry across 4+ modules increases the risk of
accidental code collisions (e.g., both `structure.rs` and `conflict.rs` using
a code that overlaps).

**Recommendation**: Define all diagnostic code constants in `diagnostics.rs`
with doc comments, following the E/W pattern. The S and C modules import them
from there. Add S and C codes to the `error_codes_are_unique` test.

### Finding 9 (Low): `doc` subcommand output format not versioned or extensible

**Location**: Wave 2, Agent E — Design Decisions

The markdown catalog format is hardcoded:
```markdown
## aigent-builder
> Generates AI agent skill definitions...

**Compatibility**: Claude Code
**Location**: `skills/aigent-builder/SKILL.md`
```

This is fine for v1, but there's no discussion of:
- What happens if `compatibility` is `None` (is the line omitted?)
- Whether `allowed-tools` or other fields are included
- How to handle skills with no description
- Sort order (alphabetical? directory order?)

**Recommendation**: Define the format precisely: which fields appear, how
missing fields are handled, and sort order. Consider making the format
template-driven for future extensibility.

### Finding 10 (Low): Wave 4 verify step doesn't include new integration scenarios

**Location**: Wave 4

The verification agent runs `cargo fmt --check && cargo clippy && cargo test &&
cargo build --release`. But the new features (watch mode, tester ranking,
conflict detection) have inherently hard-to-test behaviors:

- Watch mode requires filesystem timing
- Tester ranking requires known-good score expectations
- Conflict detection requires calibrated similarity thresholds

The unit tests inside each agent may cover these, but the wave-4 verify step
doesn't add cross-module integration tests (e.g., "score → tester → conflict"
pipeline).

**Recommendation**: Add explicit cross-module smoke tests to wave 4 or to a
final integration test file. Example: validate + lint + score a known skill
collection, then run tester queries and conflict detection, verifying the
full pipeline produces consistent results.

### Finding 11 (Low): M11 code review findings may affect M12 assumptions

**Location**: Cross-cutting

The M11 code review (in `dev/m11/review.md`) identified issues that M12
should be aware of:

- **M11 F1 (High)**: `build --template` doesn't wire through to `build_skill()`.
  M12's upgrade command (Agent H) may use `build_skill()` — if so, it inherits
  this limitation. If M11 F1 is resolved before M12 starts, no impact.
- **M11 F5 (Medium)**: Interactive build always forces `no_llm = true`. M12
  doesn't use interactive build directly, but skill authors using `build
  --interactive` won't get LLM-enhanced output.
- **M11 F6 (Medium)**: `--output` always overwrites. The `doc --output` command
  (Agent E) should use read-then-compare to avoid unnecessary mtime changes.
  Don't copy the `to-prompt --output` pattern as-is.

**Recommendation**: Check M11 code review resolution status before M12
implementation. If M11 F1 is unresolved, Agent H should avoid calling
`build_skill()` with templates or document the limitation.

## Observations

1. **Wave ordering is correct**: Wave 1 (scorer + structure) must precede
   Wave 2 (conflict detection uses scorer output) and Wave 3 (upgrade uses
   score + validate). No circular dependencies.

2. **`ScoreResult` is a clean composition**: The scorer runs `validate()` for
   structural checks (60 points) and `lint()` for quality checks (40 points,
   8 per check). This reuses existing infrastructure without duplication. The
   total 0–100 scale is intuitive.

3. **The tester fills a real spec gap**: The Anthropic spec recommends
   "evaluation-driven development" but provides no tooling. Even a simple
   keyword-matching tester is better than nothing — it gives skill authors a
   fast feedback loop for discrimination testing.

4. **`discover_skills()` already exists**: The plan's batch operations
   (conflict detection, doc generation, tester) can reuse the recursive
   discovery function from M10's `validator.rs`. This is good reuse.

5. **The upgrade command addresses the `--dry-run` gap**: Plan review Finding
   3 from M10 flagged that `--apply-fixes` has no preview mode. The `upgrade`
   subcommand defaults to dry-run, which is the safer pattern. However, this
   creates two different UX patterns for the same concept (fix vs. upgrade).

6. **README improvements (#70) is appropriately left flexible**: The plan says
   "Scope TBD based on user input during implementation." This acknowledges
   that documentation content should be determined by what actually ships, not
   planned in advance.

7. **The plan correctly avoids embedding model dependencies**: The tester uses
   simple keyword matching rather than requiring an embedding model. This keeps
   the dependency tree small and avoids API key requirements. The tradeoff is
   lower recall for semantically similar but lexically different descriptions.

8. **Nine agents across 3 waves is manageable**: Each wave has 3 parallel
   agents with clear boundaries. No agent touches more than 2 source files.
   This reduces merge conflicts compared to M10's original 15-agent plan.

9. **Reconciliation section is well-done**: The added reconciliation documents
   the M11 absorption cleanly, maps all deliverables, identifies reusable
   artifacts, and provides implementation guidance. This is a good practice
   for plans that span multiple dependent milestones.

10. **`--output` pattern should learn from M11 F6**: Both `to-prompt --output`
    and the planned `doc --output` write files. M11's implementation always
    overwrites then compares. M12's `doc --output` should read-first-compare
    to avoid unnecessary mtime changes — a lesson from the M11 code review.

## Verdict

**Approved** — the plan is well-structured, all M11 API dependencies are
verified, and the reconciliation section addresses the previous blocking
concern (Finding 1).

**Should address before implementation**:
- Finding 2 (Medium): Specify the tester scoring algorithm concretely. Without
  a formula, tests will be fragile and different implementations may produce
  different results.
- Finding 6 (Medium): Gate `notify` behind a feature flag to keep the default
  binary lean.
- Finding 7 (Low): Tighten scorer skill `allowed-tools` pattern.

**Should consider during implementation**:
- Finding 3: Platform-gate S002 permission check
- Finding 4: Clarify upgrade vs. validate --apply-fixes distinction
- Finding 5: Document 0.7 threshold as heuristic, consider --threshold flag
- Finding 8: Centralize S/C diagnostic codes in diagnostics.rs
- Finding 11: Check M11 code review resolution before implementing Agent H

All other findings are advisory.

### Checklist

- [x] Finding 1 resolved: M11 API surface verified — all types and signatures match
- [ ] Finding 2 addressed: define tester scoring formula
- [ ] Finding 3 considered: platform-gate S002 permission check
- [ ] Finding 4 considered: clarify upgrade vs. validate --apply-fixes distinction
- [ ] Finding 5 considered: document 0.7 threshold as heuristic, consider --threshold flag
- [ ] Finding 6 addressed: gate notify behind feature flag
- [ ] Finding 7 addressed: tighten scorer allowed-tools
- [ ] Finding 8 addressed: centralize S/C diagnostic codes in diagnostics.rs
- [ ] Finding 9 noted: define doc output format precisely
- [ ] Finding 10 noted: add cross-module integration tests
- [ ] Finding 11 noted: check M11 code review resolution before Agent H

## Additional Code Review (2026-02-20)

### Findings

1. High: `upgrade --apply` reports missing `metadata.version` / `metadata.author` but does not apply those fixes when a `metadata:` block already exists.
   - References: `src/main.rs:1007`, `src/main.rs:1009`, `src/main.rs:1018`
   - Current logic only appends metadata keys when `meta_block.is_none()`. If metadata exists but is partial (for example only `author`), suggestions are emitted but no change is written.
   - Repro: SKILL with `metadata.author` only -> `upgrade --apply` prints missing `metadata.version` but leaves file unchanged.

2. Medium: structure validation misses non-executable scripts in nested paths (for example `scripts/run.sh`).
   - References: `src/structure.rs:135`, `src/structure.rs:140`, `src/structure.rs:142`
   - `check_script_permissions()` only scans immediate files in the skill root and never descends into subdirectories.
   - Impact: issue #54 script-permission coverage is incomplete for common layouts where scripts live under `scripts/`.

3. Medium: conflict similarity is case-sensitive despite docs/comments claiming lowercase tokenization.
   - References: `src/conflict.rs:148`, `src/conflict.rs:151`, `src/conflict.rs:156`
   - `jaccard_similarity()` does not lowercase tokens before building sets, so descriptions differing only by case are treated as different words.
   - Repro: "Processes PDF files quickly" vs "processes pdf files quickly" produces no C002 conflict.

4. Low: `doc` catalog omits `compatibility` / `license` because `read_properties()` is called with a SKILL.md file path instead of the parent directory.
   - References: `src/main.rs:879`, `src/main.rs:880`
   - `entry.location` is `.../SKILL.md`; passing it directly to `read_properties()` fails lookup and optional fields are silently skipped.
   - Impact: generated docs miss metadata they intend to show.

### Residual Testing Gaps

1. No upgrade test for partial metadata (`metadata` present but missing one key) with `--apply`.
2. No structure test for nested script permissions (`scripts/run.sh` non-executable).
3. No conflict test for case-only description differences.
4. No doc test asserting `Compatibility`/`License` rendering from SKILL frontmatter.

---

## M12 Code Review — Full Review

**Branch**: `dev/m12`
**Commit**: `9ceea15` — "M12: Ecosystem & Workflow features"
**Baseline**: `main` at `7cd1aa7` (M11 squash-merged)
**Delta**: +4413 / -26 lines across 17 files (1 commit)
**Reviewer**: Claude (automated)
**Date**: 2026-02-20

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --check` | ✅ Clean |
| `cargo clippy -- -D warnings` | ✅ Clean |
| `cargo test` | ✅ 414 passed (313 unit + 75 cli + 25 plugin + 1 doc-test) |
| `cargo doc --no-deps` | ✅ Clean |

### Scope

M12 addresses 9 issues across scoring, testing, documentation, conflict detection,
watch mode, directory validation, upgrade, and README improvements:

| Issue | Title | Status |
|-------|-------|--------|
| #54 | Directory structure validation | ✅ Implemented (S001–S004) |
| #59 | Quality scoring | ✅ Implemented (0–100 scale) |
| #61 | Skill upgrade/migration | ✅ Implemented (dry-run + --apply) |
| #63 | Scorer skill | ✅ Implemented (hybrid mode) |
| #64 | Skill tester | ✅ Implemented (query match, single-skill) |
| #67 | Documentation generation | ✅ Implemented (markdown catalog) |
| #68 | Watch mode | ✅ Implemented (feature-gated) |
| #69 | Cross-skill conflict detection | ✅ Implemented (C001–C003) |
| #70 | README improvements | ✅ Implemented (comprehensive) |

### Changed Files

| File | Lines | Summary |
|------|-------|---------|
| `src/scorer.rs` | 568 | NEW: `score()`, `ScoreResult`, `format_text()`, 13 tests |
| `src/structure.rs` | 518 | NEW: `validate_structure()`, S001–S004, 15 tests |
| `src/conflict.rs` | 368 | NEW: `detect_conflicts()`, C001–C003, Jaccard similarity, 14 tests |
| `src/tester.rs` | 329 | NEW: `test_skill()`, `QueryMatch`, `format_test_result()`, 10 tests |
| `src/diagnostics.rs` | +22 | S001–S004, C001–C003 constants with docs |
| `src/lib.rs` | +12 | 4 new module declarations + re-exports |
| `src/main.rs` | +552 | Score, Doc, Test, Upgrade, watch mode, conflict integration |
| `skills/aigent-scorer/SKILL.md` | 86 | NEW: Hybrid scorer skill |
| `Cargo.toml` | +4 | `regex`, `notify` (optional), `watch` feature |
| `README.md` | +92 | Updated CLI surface, extras table, project structure |
| `tests/cli.rs` | +330 | 21 new CLI tests |
| `tests/plugin.rs` | +40 | 4 new scorer skill tests |
| `Cargo.lock` | +221 | `regex`, `notify` transitive deps |
| `dev/m11/review.md` | +198 | M11 full code review (from prior session) |
| `dev/m12/plan.md` | +643 | Plan + reconciliation sections |
| `dev/m12/review.md` | +419 | Plan review (from prior session) |
| `dev/posts/aigent-linkedin-post.md` | 37 | NEW: Marketing content |

### Plan Conformance

All 9 issues are fully addressed. No issues deferred or partially implemented.

### Plan Review Finding Resolution

| Finding | Resolution | Status |
|---------|-----------|--------|
| F1 (High → Resolved) | M11 API verified | ✅ Confirmed in code |
| F2 (Medium) | Tester formula specified | ✅ Implemented as word-overlap ratio with Strong/Weak/None thresholds |
| F3 (Medium) | S002 platform-gated | ✅ `#[cfg(unix)]` block in `structure.rs:131` |
| F4 (Medium) | Upgrade separate from validate | ⚠️ See observation 5 |
| F5 (Medium) | 0.7 threshold + custom threshold | ✅ `detect_conflicts_with_threshold()` exposed |
| F6 (Medium) | notify behind feature flag | ✅ `[features] watch = ["notify"]` in Cargo.toml |
| F7 (Low) | Scorer allowed-tools tightened | ✅ Uses `Bash(aigent score *), Bash(aigent validate *)` |
| F8 (Low) | S/C codes in diagnostics.rs | ✅ All 7 codes centralized with doc comments |
| F9 (Low) | Doc format defined | ✅ Alphabetical sort, missing fields omitted |
| F10 (Low) | Cross-module tests | ⚠️ Partial — no explicit pipeline test |
| F11 (Low) | M11 findings resolved | ✅ `--template` removed from build, `--output` fixed |

### Prior Finding Validation

**Prior F1 (High): `upgrade --apply` skips partial metadata** — **CONFIRMED**

At `src/main.rs:1009`, the condition `if meta_block.is_none()` means that if a
`metadata:` block exists with *some* keys (e.g., `author` but not `version`),
the upgrade will report the missing key as a suggestion but won't write it. The
`--apply` flag silently does nothing for the metadata portion. This is because
the code only enters the metadata-appending branch when *no* metadata block
exists at all.

**Prior F2 (Medium): S002 misses nested scripts** — **CONFIRMED**

`check_script_permissions()` at `src/structure.rs:135` calls `std::fs::read_dir(dir)`
which only iterates direct children. Scripts at `scripts/run.sh` or similar nested
paths are never checked. This is a common skill layout (especially for
`code-skill` templates from M11).

**Prior F3 (Medium): Jaccard similarity is case-sensitive** — **CONFIRMED**

`jaccard_similarity()` at `src/conflict.rs:150-160` operates on raw `&str`
slices from `split_whitespace()` without lowercasing. The doc comment at line
148 claims "Tokenizes both strings into lowercase words" but the code does not
call `.to_lowercase()`. This means "PDF" and "pdf" are treated as different
tokens, reducing detection sensitivity.

Note: The tester (`src/tester.rs:172`) *does* lowercase the description before
matching (`desc_lower = description.to_lowercase()`), so the tester is correct
but the conflict detector is inconsistent.

**Prior F4 (Low): Doc catalog misses optional fields** — **CONFIRMED**

`format_doc_catalog()` at `src/main.rs:879` constructs `loc_path` from
`entry.location`, which is the SKILL.md file path (e.g., `/path/to/SKILL.md`).
`read_properties()` expects a directory, not a file path, so it looks for
`SKILL.md` inside the path — finding nothing. The `if let Ok(props)` silently
swallows the error, and `compatibility`/`license` are never rendered.

### Additional Findings

**F5 (Medium): `score` exit code policy is unusually strict**

`main.rs:483-484` exits with non-zero if `result.total < result.max` — meaning
any lint issue (e.g., no trigger phrase) causes a non-zero exit. This makes
`aigent score` unsuitable for CI pipelines that want to distinguish "has
structural errors" (truly broken) from "has quality suggestions" (still valid).

- References: `src/main.rs:482-484`
- Impact: Users can't use `aigent score` as a pass/fail gate for structural validity.
- Recommendation: Exit 0 for structural pass (score ≥ 60), exit 1 for structural
  fail (score < 60), or add `--strict` flag for the current behavior.

**F6 (Medium): Tester departs from plan's multi-skill ranking design**

The plan (§Skill Tester, #64) describes a multi-skill ranking system:
"Load all skills from specified directories → rank skills by description relevance
→ report which skills would activate." The implementation tests a *single* skill
against a query (`test_skill(dir, query)`) rather than ranking across a collection.

The CLI signature is `test <skill-dir> <query>` (single skill) rather than
`test <dirs...> --query <query>` (collection ranking) as specified in the plan.

- References: `src/tester.rs:65`, `src/main.rs:173-178`, plan lines 366-408
- Impact: The plan's key feature (competitive ranking with conflict flagging)
  is not implemented. Single-skill testing is useful but less ambitious.
- Recommendation: Document this as a simplification. The current API is a
  reasonable v1 that can be extended to multi-skill ranking later.

**F7 (Medium): Structural scoring is all-or-nothing (60 or 0)**

`score_structural()` at `src/scorer.rs:188-192` awards 60 points only if there
are *no errors AND no warnings*. This means a single W001 (unknown field)
warning zeros the entire structural score, even though the skill is structurally
valid. Individual check results are computed and displayed but don't contribute
to the score — the score is binary.

- References: `src/scorer.rs:188-192`
- Impact: A skill with `context: fork` (Claude Code extension field) scores
  0/60 structural when validated with `--target standard` due to W001, even
  though only the field is unexpected, not the skill structure.
- Recommendation: Consider proportional scoring (pass 5 of 6 checks = 50/60)
  or differentiate warnings from errors (errors zero the score, warnings reduce
  it proportionally).

**F8 (Low): `read_body()` duplicated in 3 files**

The pattern of reading a SKILL.md body (find → read → parse frontmatter → return
body) is copied identically in:
- `src/scorer.rs:294-307`
- `src/structure.rs:226-239`
- `src/main.rs:899-913`

All three share the same function name and identical logic.

- Impact: Low — these are internal helpers, but the duplication increases
  maintenance surface.
- Recommendation: Extract to `parser::read_body()` or a shared utility.

**F9 (Low): `upgrade --apply` error handling is lossy**

At `src/main.rs:1022-1023`, `fs::write` failure is handled with `unwrap_or_else`
that prints an error and *returns* (doesn't call `process::exit`). This means
the function continues and returns `Ok(suggestions)` — the caller sees success
even though the file wasn't written.

- References: `src/main.rs:1022-1023`
- Impact: Silent partial failure — user sees "Applied upgrades" error message
  but the function returns Ok with suggestions, and the exit code may be 0.
- Recommendation: Return `Err` or call `process::exit(1)` on write failure.

**F10 (Low): Watch mode doesn't use `--format` parameter**

`run_watch_mode()` accepts a `_format: Format` parameter (note the underscore)
at `src/main.rs:719` but never uses it. Watch mode always outputs in text format
regardless of `--format json`.

- References: `src/main.rs:719`
- Impact: Low — watch mode is interactive, JSON output is less useful. But the
  flag is accepted without warning.
- Recommendation: Either pass format through to `run_validation_pass()` or
  document that `--format` is ignored in watch mode.

**F11 (Low): `regex` dependency added but not listed in plan's Cargo.toml changes**

The plan's §Cargo.toml Changes lists only `notify = "8"` as a new dependency.
The implementation also adds `regex = "1"` (used by `structure.rs` for markdown
link extraction). This is a minor omission in the plan.

- References: `Cargo.toml:22`, `src/structure.rs:13`
- Impact: None — `regex` is a standard dependency and appropriate for link
  parsing. Just a plan-vs-implementation delta.

### Observations

1. **Test coverage is excellent**: 414 tests total, with 83 new tests in M12
   (48 unit + 21 CLI + 4 plugin + 10 tester). The scorer alone has 13 unit
   tests covering constants, perfect/imperfect/broken skills, JSON serialization,
   and per-check granularity.

2. **Feature-gated watch mode is well-implemented**: The `#[cfg(feature = "watch")]`
   / `#[cfg(not(feature = "watch"))]` pattern in `main.rs` gives a clear error
   message when watch is used without the feature. The watch mode itself has
   proper debouncing, re-discovery of new skills, and terminal clearing.

3. **Diagnostic code centralization done right**: All S001–S004 and C001–C003
   codes are defined as constants in `diagnostics.rs` with doc comments, imported
   by `structure.rs` and `conflict.rs`, and included in the `error_codes_are_unique`
   test. This follows the pattern established for E/W codes.

4. **Scorer skill follows all plan review recommendations**: `allowed-tools` uses
   the tightened pattern (`aigent score *`, `aigent validate *`), description
   includes trigger phrase ("Use when"), and the skill embeds the full checklist
   for prompt-only mode. The hybrid CLI/prompt-only pattern is consistent with
   M9's builder and validator skills.

5. **Upgrade is distinct from validate but minimal in scope**: `run_upgrade()` in
   `main.rs` does *not* call `validate --apply-fixes` internally (plan review F4
   recommended this). Instead it's a standalone check for 5 items: compatibility,
   metadata.version, metadata.author, trigger phrase, body length. This is
   cleaner but means validate fixes and upgrade fixes don't compose.

6. **`doc --output` correctly implements read-then-compare**: Unlike M11's
   `to-prompt --output` (which always wrote then compared), the doc subcommand
   at `src/main.rs:542-547` reads existing content first and only writes when
   changed. This was a lesson from M11 code review F6.

7. **README is comprehensive and up-to-date**: The README now documents all 10
   CLI commands, all validate flags, the full project structure with M12 modules,
   the extras table with all new features, and the API reference with M12 types
   and functions. The milestone table shows M10/M11/M12 in progress.

8. **Conflict detection has a clean API surface**: `detect_conflicts()` (default
   threshold) and `detect_conflicts_with_threshold()` (custom) provide both
   convenience and flexibility. The library re-exports both, making the custom
   threshold available to library consumers.

9. **Tester uses substring matching instead of Jaccard**: While the conflict
   detector uses Jaccard similarity (set intersection/union), the tester's
   `compute_query_match()` uses `desc_lower.contains()` — substring matching.
   This is appropriate for query→description testing (where "PDF" should match
   "processes PDF files") but is a different algorithm than described in the plan.

10. **No new `unwrap()` in library code**: All 4 new library modules
    (`scorer.rs`, `structure.rs`, `conflict.rs`, `tester.rs`) use proper error
    handling with `?` and `match`. The only `unwrap()` calls are in test helpers
    and `main.rs`, consistent with project conventions.

### Verdict

**Conditional merge** — The implementation is solid, well-tested, and addresses
all 9 planned issues with 414 passing tests. Three issues should be addressed
before merge:

1. **Prior F3 (Medium)**: `jaccard_similarity()` must lowercase tokens to match
   its doc comment and provide case-insensitive conflict detection. This is a
   one-line fix (add `.to_lowercase()` before collecting into sets) plus a test.

2. **Prior F4 (Low → should fix)**: `format_doc_catalog()` passes
   `entry.location` (a file path) to `read_properties()` (expects a directory).
   Fix: resolve to parent directory before calling `read_properties()`.

3. **Prior F1 (High)**: `upgrade --apply` silently skips metadata additions when
   a partial metadata block exists. Either extend the YAML manipulation to handle
   partial metadata, or clearly document that `--apply` only adds metadata when
   no metadata block exists at all.

### Pre-merge Checklist

- [ ] Fix Prior F3: Add `.to_lowercase()` in `jaccard_similarity()` + add case-insensitive test
- [ ] Fix Prior F4: Resolve `entry.location` to parent dir in `format_doc_catalog()`
- [ ] Fix Prior F1: Handle partial metadata in `upgrade --apply` or document limitation
- [ ] Consider F5: Adjust `score` exit code policy (structural-only vs all-or-nothing)
- [ ] Consider F6: Document tester simplification vs plan's multi-skill ranking
- [ ] Consider F7: Proportional structural scoring vs binary all-or-nothing
- [ ] Consider F9: Fix `upgrade --apply` write error handling to propagate failure
- [ ] Consider F8: Extract shared `read_body()` to `parser` module
