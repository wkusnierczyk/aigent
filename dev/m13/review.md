# M13: Cleanup — Plan Review

## Overall Assessment

M13 is a quality and consistency pass — 6 issues (plus 3 M12 residual bugs),
4 waves, 10 agents. The scope is well-bounded: doc-comments, scorer labels,
tester algorithm, upgrade robustness, YAML handling, and CLI naming. No new
external features — everything improves or polishes existing code.

The plan is clearly structured with a conservative wave ordering: low-risk
fixes first (Wave 1), algorithm improvements (Wave 2), CLI surface redesign
(Wave 3), verification (Wave 4). This is the right approach for a cleanup
milestone.

The plan's primary risks are: (1) #81 (YAML AST-preserving parser) which the
plan correctly identifies as the highest-risk item and provides a fallback
strategy, and (2) #76 (CLI renames) which has a wide ripple effect across
skills, hooks, tests, and documentation.

## Plan Conformance

### Issues Addressed

- [x] #75 — Doc-comments (Wave 1, Agent A)
- [x] #76 — CLI naming alignment (Wave 3, Agents G/H/I)
- [x] #78 — Score check labels (Wave 1, Agent B)
- [x] #79 — Tester weighted scoring (Wave 2, Agent D)
- [x] #80 — Upgrade --full (Wave 2, Agent E)
- [x] #81 — YAML AST-preserving parser (Wave 2, Agent F)

6 of 9 M13 milestone issues addressed. 3 issues in the M13 milestone are
**not covered by the plan**:

| Issue | Title | Missing Reason |
|-------|-------|----------------|
| #45 | Replace `eprintln!` warnings in builder with structured warning channel | Not mentioned |
| #74 | Audit Claude Code hook variable quoting conventions | Not mentioned |
| #82 | Add version management CLI targets | Created after plan was written |

### Issue Deviations

None for the 6 covered issues — all are faithfully represented.

### M12 Residual Bugs

The plan correctly incorporates 4 M12 code review findings as Wave 1 fixes.
However, **2 of 4 are already fixed** on the current `main`:

| Bug | Plan Status | Actual Status |
|-----|-------------|---------------|
| Jaccard case-sensitivity | Wave 1 Agent C fix | **Already fixed** on `main` (commit `0cb0713`) |
| Doc catalog path resolution | Wave 1 Agent C fix | **Already fixed** on `main` (commit `0cb0713`) |
| Partial metadata in upgrade --apply | Wave 2 Agent E fix | **Already fixed** on `main` (commit `0cb0713`) |
| `read_body()` duplication | Wave 1 Agent C fix | **Still present** — 3 copies in `scorer.rs`, `structure.rs`, `main.rs` |

The plan was written against the M12 merge commit `0d6de3d` but the fixes
were applied in the follow-up commit `0cb0713` (merged via PR #77). Agent C's
scope should be reduced to only the `read_body()` deduplication.

## Findings

### Finding 1 (High): Plan references stale baseline — doc-comments already present

**Location**: Design Decisions §Doc-Comments (#75), Wave 1 Agent A

The plan says 5 public items are missing doc-comments. Checking the current
codebase (`main` at `e991b69`), **all 5 already have doc-comments**:

| Item | File | Has `///`? |
|------|------|-----------|
| `AnthropicProvider` struct | `src/builder/providers/anthropic.rs:11` | ✅ Yes |
| `GoogleProvider` struct | `src/builder/providers/google.rs:11` | ✅ Yes |
| `OllamaProvider` struct | `src/builder/providers/ollama.rs:12-13` | ✅ Yes |
| `capitalize_first` fn | `src/builder/util.rs:1` | ✅ Yes |
| `to_title_case` fn | `src/builder/util.rs:13` | ✅ Yes |

The only remaining part of #75 is adding `#![warn(missing_docs)]` to
`src/lib.rs` — a one-liner. Agent A's scope should be reduced accordingly.

**Recommendation**: Verify whether #75 can be closed as already-done (minus
the `warn` attribute), or keep it open solely for the `#![warn(missing_docs)]`
gate.

### Finding 2 (Medium): Three milestone issues not covered by plan

**Location**: Issue list (line 10)

The plan lists issues `#75, #76, #78, #79, #80, #81`. But the M13 milestone
has 9 open issues. Three are missing:

1. **#45** — `[FEATURE] Replace eprintln! warnings in builder with structured
   warning channel`. This is a genuine code quality issue (11 `eprintln!`
   calls in `src/builder/mod.rs`) that the codebase audit confirms. It fits
   naturally in a cleanup milestone.

2. **#74** — `Audit Claude Code hook variable quoting conventions`. This was
   created during M11 review. It's documentation/audit work — low effort.

3. **#82** — `Add version management CLI targets`. Created during this
   session's release process. Involves adding `cargo-edit` documentation and
   potentially a `scripts/bump-version.sh`.

**Recommendation**: Either add these to the plan (Wave 1 for #74/#82, Wave 2
for #45) or explicitly mark them as deferred to a future milestone. Leaving
issues in a milestone but out of the plan creates ambiguity about scope.

### Finding 3 (Medium): `fmt` subcommand is a feature, not cleanup

**Location**: Wave 3, Agent H — Design Decisions §CLI Naming Alignment (#76)

The `fmt` subcommand described in the plan is a substantial new feature:
~200 lines of new code in a new module (`src/formatter.rs`), with canonical
key ordering, quoting normalization, markdown body formatting, `--check`
flag, and recursive discovery. This is new functionality, not cleanup.

Including it in a "Cleanup" milestone muddles the milestone's identity. The
`build → new` rename and `check` alias are genuine cleanup. The `fmt`
subcommand is a new capability that deserves its own issue and could be
tracked in a separate milestone.

**Recommendation**: Consider either (a) splitting `fmt` into its own issue
with an `enhancement` label, or (b) renaming the milestone to "M13: Cleanup &
Polish" to reflect the mix. The `fmt` subcommand is valuable but its scope
(~200 lines, new module, new tests) is larger than any other individual item
in this plan.

### Finding 4 (Medium): Wave 2 Agent F depends on Agent E but the plan says they're independent

**Location**: Wave 2, Agent F (line 398)

The plan states Wave 2 agents are "independent of each other" (line 332), but
then says:

> "Agent F depends on Agent E's partial metadata fix — Agent E establishes
> the test cases and overall flow, Agent F refines the implementation."

This is a contradiction. If F depends on E, they cannot run in parallel.
Furthermore, since the partial metadata fix is **already on main** (see
Finding 1), Agent E's scope changes and the dependency chain shifts.

**Recommendation**: Re-evaluate the dependency. With partial metadata already
fixed, Agent F only needs to improve the YAML manipulation approach. It may
now be truly independent of Agent E.

### Finding 5 (Medium): Upgrade --full composes two error-reporting pipelines with different semantics

**Location**: Wave 2, Agent E — Design Decisions §Upgrade --full (#80)

When `--full` is set, the plan says:
1. Run `validate()` + `lint()` to collect diagnostics
2. If `--apply`: call `apply_fixes()` first
3. Re-read the file
4. Continue with upgrade pipeline

Step 2 applies fixes for *invalid* metadata (e.g., uppercase names). Step 4
adds *recommended* fields. But the output to the user merges both into a
single suggestion list. The user can't distinguish "your skill was broken and
I fixed it" from "here's a nice-to-have improvement."

**Recommendation**: Separate the output into two sections when `--full` is
used:

```
Validation fixes applied (--full):
  ✓ Fixed name casing: MySkill → my-skill

Upgrade suggestions:
  ⚠ Missing 'compatibility' field
  ⚠ Missing 'metadata.version'
```

This makes the `--full` flag's value proposition clear to the user.

### Finding 6 (Medium): CLI rename ripple effects are underspecified

**Location**: Wave 3, Agent I

Agent I lists ripple effects for skill files, hooks, and tests but doesn't
mention:

- **CHANGES.md**: The changelog references `build` subcommand — should be
  updated if renamed to `new`.
- **dev/m12/plan.md and review.md**: Historical documents reference `build`
  subcommand — these should probably *not* be updated (they're historical
  records), but the decision should be explicit.
- **install.sh**: If install docs reference CLI subcommands.
- **GitHub issues**: Open issues (#79, #80, etc.) reference current
  subcommand names in their bodies.

**Recommendation**: Add a checklist of all files containing subcommand name
references (a simple `grep -r "aigent build"` across the repo) and decide
which ones to update.

### Finding 7 (Low): `--full` flag name may confuse with `--apply`

**Location**: Design Decisions §Upgrade --full (#80)

The upgrade command will have three flags: `--apply`, `--full`, and
`--format`. The interaction between `--full` and `--apply` is:

- `upgrade` → dry-run upgrade suggestions
- `upgrade --apply` → apply upgrade suggestions
- `upgrade --full` → dry-run validate + upgrade suggestions
- `upgrade --full --apply` → apply validate fixes + upgrade suggestions

The `--full` name doesn't clearly communicate "also run validate." Users might
expect `--full` to mean "apply everything" (conflating it with `--apply`).

**Recommendation**: Consider `--with-validate` or `--validate-first` as
alternative names. Or combine: `--full-apply` as a single flag that does
both. The current API is workable but may need clear `--help` text.

### Finding 8 (Low): Tester scoring formula removes stopwords but doesn't define the stopword list

**Location**: Design Decisions §Tester Weighted Scoring (#79)

The formula says:
> `jaccard(A, B) = |A ∩ B| / |A ∪ B|` (lowercase word tokens, stopwords removed)

But no stopword list is defined. English stopword lists vary from 25 to 500+
words depending on the source. The choice of stopword list directly affects
scoring: a short list (articles + prepositions) preserves more signal than a
long NLP-style list.

**Recommendation**: Define a minimal stopword list (e.g., `a, an, the, is,
are, was, were, of, to, in, for, on, with, and, or, but, not, it, this,
that`) inline in `tester.rs` with a comment explaining the choice. Keep it
short — skill descriptions are already concise.

### Finding 9 (Low): `#![warn(missing_docs)]` may cause cascade of warnings in builder submodules

**Location**: Wave 1, Agent A

Adding `#![warn(missing_docs)]` to `lib.rs` makes missing docs a compile
warning for *all* public items in all modules — including `pub(crate)` items
in `builder/`. The `pub(crate)` items in `builder/util.rs` already have doc
comments, but future additions to any module will need them too.

This is a good quality gate, but the plan doesn't mention verifying that
`cargo clippy -- -D warnings` passes *after* adding the attribute (it does
mention `cargo doc --no-deps` but that's different). The clippy check is the
real gatekeeper since CI runs it.

**Recommendation**: Step 4 of Agent A ("Verify `cargo clippy -- -D warnings`
passes") covers this. Just ensure it's run *after* step 2, not in parallel.

### Finding 10 (Low): Wave 4 manual smoke test references commands that may not exist yet

**Location**: Wave 4, Agent J (line 483)

The smoke test says:
```
aigent new "test skill" --no-llm
aigent check skills/ --lint --structure
aigent fmt skills/ --check
```

If Wave 3 (which adds `new`, `check`, `fmt`) has merge conflicts or is
partially deferred, Wave 4's manual tests will fail. The plan doesn't specify
fallback commands using the original names.

**Recommendation**: Include both old and new names in the smoke test:
```
aigent new "test skill" --no-llm && aigent build "test skill" --no-llm  # alias
```

### Finding 11 (Low): Version management (#82) should be done first

**Location**: Not in plan (see Finding 2)

Issue #82 (version management with `cargo-edit`) is a process fix that
prevents the exact class of error that caused the v0.2.0–v0.2.2 release
failures. If M13 produces a release, the same manual version-bump problem
could recur.

**Recommendation**: Address #82 in Wave 1 alongside the other low-risk fixes.
It's minimal effort (document `cargo-edit` in README, add
`scripts/bump-version.sh`) and directly prevents release failures.

## Observations

1. **Wave ordering is correct**: Low-risk → algorithm → surface redesign →
   verify. Each wave is self-contained and wave N depends on wave N-1 being
   stable. This is conservative but appropriate for a cleanup milestone.

2. **`fmt` is the most valuable addition**: A formatting command is a
   developer experience multiplier — it eliminates manual YAML/markdown
   alignment across skill collections. The `--check` flag enables CI
   enforcement. This is worth the scope increase.

3. **The plan correctly defers #81 decisions**: The YAML AST-preservation
   problem is genuinely hard (comment preservation during round-trip
   serialization). The plan's fallback strategy ("if no clean solution exists,
   keep string-append with better edge-case handling") is pragmatic.

4. **Score fail labels are a good UX fix**: The current `[FAIL] No unknown
   fields` reading is genuinely confusing. The proposed `fail_label` approach
   with `Option<String>` is backward-compatible (JSON serialization unchanged)
   and solves the problem cleanly.

5. **Tester scoring formula is well-specified**: The 0.5/0.3/0.2 weighted
   formula with Jaccard, trigger match, and name match is concrete and
   testable. The `QueryMatch` enum thresholds (Strong ≥ 0.5, Weak ≥ 0.2)
   give clear boundaries for test assertions.

6. **The plan handles the `build → new` rename carefully**: Adding `build` as
   a hidden alias during transition prevents breaking existing workflows.
   This is the right approach for pre-1.0 software.

7. **Agent count (10) is appropriate**: Each agent has a clearly scoped task.
   No agent modifies more than 3 files. The widest-impact agent (I, ripple
   effects) touches 5 files but with mechanical changes only.

8. **Test count delta is reasonable**: +25–30 tests for a cleanup milestone
   that changes scoring algorithms, adds a new subcommand, and renames
   existing ones. The new tests are well-targeted at behavior changes.

9. **`read_body()` deduplication is overdue**: Three identical copies of the
   same function across `scorer.rs`, `structure.rs`, and `main.rs` is the
   clearest cleanup target in the codebase. Moving to `parser.rs` as a
   `pub fn` is straightforward.

10. **The plan doesn't mention `eprintln!` cleanup (#45)**: The builder module
    has 11 `eprintln!` calls that should use structured errors per project
    conventions. This is exactly the kind of issue a cleanup milestone should
    address.

## Verdict

**Approved with revisions** — the plan is well-structured and addresses the
core cleanup issues with appropriate wave ordering and risk management.

**Must address before implementation**:

- Finding 1 (High): Update Agent A scope — doc-comments are already present.
  Agent A reduces to adding `#![warn(missing_docs)]` only.
- Finding 2 (Medium): Decide on #45, #74, #82 — either add to plan or
  explicitly defer. Three M13 issues outside the plan creates scope ambiguity.

**Should address before implementation**:

- Finding 3 (Medium): Acknowledge that `fmt` is a feature, not cleanup. Either
  create a separate issue or adjust milestone framing.
- Finding 4 (Medium): Correct the Wave 2 independence claim — Agent F depends
  on Agent E (or clarify that the dependency is resolved by existing fixes on
  main).

**Should consider during implementation**:

- Finding 5: Separate validate-fix vs. upgrade-suggestion output in `--full`
- Finding 6: Enumerate all files with subcommand references for rename ripple
- Finding 7: Consider `--with-validate` naming over `--full`
- Finding 8: Define the stopword list explicitly
- Finding 11: Prioritize #82 in Wave 1

All other findings are advisory.

### Checklist

- [ ] Finding 1 addressed: update Agent A scope (doc-comments already present)
- [ ] Finding 2 addressed: decide on #45, #74, #82 inclusion
- [ ] Finding 3 considered: `fmt` is a feature, not cleanup
- [ ] Finding 4 addressed: correct Wave 2 dependency claim
- [ ] Finding 5 considered: separate output for `--full` mode
- [ ] Finding 6 considered: enumerate all rename ripple files
- [ ] Finding 7 noted: `--full` naming alternatives
- [ ] Finding 8 noted: define stopword list
- [ ] Finding 9 noted: verify clippy after `warn(missing_docs)`
- [ ] Finding 10 noted: wave 4 fallback commands
- [ ] Finding 11 considered: prioritize #82 in Wave 1

---

## Consolidated Plan Review (2026-02-21)

Review of the consolidated plan (lines 806–1117 of `dev/m13/plan.md`), which
supersedes the incremental plan and incorporates all prior review findings.

### Scope Change Assessment

The milestone has evolved substantially from the original "M13: Cleanup":

| Version | Issues | Waves | Agents | New modules | Estimated lines |
|---------|--------|-------|--------|-------------|-----------------|
| Original plan | 6 | 4 | 10 | 1 (`formatter.rs`) | +600–800 |
| Consolidated plan | 12 | 5 | 13 | 3 (`formatter.rs`, `assembler.rs`, `test_runner.rs`) | +900–1200 |

The milestone title changed from "M13: Cleanup" to "M13: Enhancements" to
reflect the addition of three new feature issues (#83, #84, #85) and the
inclusion of three previously missing issues (#45, #74, #82). This is a
significant scope expansion — the milestone now includes 4 genuine new
features (`fmt`, `build`, `test`, `check`), not just cleanup work.

### Prior Review Finding Resolution

| Finding | Severity | Resolution in Consolidated Plan | Status |
|---------|----------|--------------------------------|--------|
| F1 (High) | Agent A scope stale — doc-comments already present | ✅ Plan acknowledges all 5 doc-comments exist; Agent A reduced to `warn(missing_docs)` + hook audit | **Resolved** |
| F2 (Medium) | Three milestone issues not in plan (#45, #74, #82) | ✅ All three added: #45 → Wave 2 Agent G-alt, #74 → Wave 1 Agent A, #82 → Wave 1 Agent C | **Resolved** |
| F3 (Medium) | `fmt` is a feature, not cleanup | ✅ Milestone renamed "Enhancements"; plan explicitly notes `fmt` is "a **new feature**, not cleanup" (line 281) | **Resolved** |
| F4 (Medium) | Wave 2 Agent F depends on Agent E | ✅ Reconciliation §Wave 2 Dependency Correction confirms Agent F is now independent (partial metadata already fixed on main) | **Resolved** |
| F5 (Medium) | `--full` should separate output sections | ✅ Design Decisions §Upgrade --full and Agent E step 3 specify two-section output with "Validation fixes" and "Upgrade suggestions" | **Resolved** |
| F6 (Medium) | Rename ripple effects underspecified | ✅ Agent I now lists all affected files including `CHANGES.md`, with explicit note that historical docs are NOT updated | **Resolved** |
| F7 (Low) | `--full` naming may confuse | Not addressed — name kept as `--full` | **Accepted** |
| F8 (Low) | Stopword list not defined | ✅ Explicit 20-word `STOPWORDS` list defined inline (lines 144–148) | **Resolved** |
| F9 (Low) | `warn(missing_docs)` cascade risk | ✅ Agent A step 3 verifies `cargo clippy -- -D warnings` passes | **Resolved** |
| F10 (Low) | Wave 4 smoke test references future commands | Smoke test moved to Wave 5 Agent L; references both old and new names | **Resolved** |
| F11 (Low) | Version management should be Wave 1 | ✅ #82 placed in Wave 1 Agent C | **Resolved** |

All 8 "must/should address" findings are resolved. The 3 advisory findings
(F7, F10, F11) are either resolved or explicitly accepted.

### Baseline Verification

| Claim | Verified? | Notes |
|-------|-----------|-------|
| Main at `e991b69` | ✅ | `git log --oneline -1 main` confirms |
| 416 tests (314 + 75 + 26 + 1) | ✅ | `cargo test` confirms 314 + 75 + 26 + 1 = 416 |
| 10 CLI subcommands | ✅ | `Validate`, `Lint`, `ReadProperties`, `ToPrompt`, `Build`, `Score`, `Doc`, `Test`, `Upgrade`, `Init` |
| Jaccard fix on main | ✅ | `conflict.rs:155,163` has `.to_lowercase()` |
| Doc catalog path fix on main | ✅ | `main.rs:881` resolves to parent dir |
| Partial metadata fix on main | ✅ | `main.rs:1019-1037` handles `else` branch |
| `read_body()` still duplicated | ✅ | 3 copies: `scorer.rs:294`, `structure.rs:233`, `main.rs:901` |
| `#![warn(missing_docs)]` already present | ✅ | `lib.rs:22` — already on main |
| All 5 doc-comments present | ✅ | `cargo doc --no-deps` and `cargo clippy` both clean |
| `dev/m13` branch not yet created | ✅ | Only `main`, `dev/m10`, `dev/m11`, `dev/m12` exist |

### New Findings

#### Finding 1 (High): `#![warn(missing_docs)]` already on main — Agent A #75 work is fully done

**Location**: Consolidated plan line 884, Wave 1 Agent A (line 976)

The plan says "Add `#![warn(missing_docs)]` to `src/lib.rs`" as remaining
work for #75. But `lib.rs:22` already has `#![warn(missing_docs)]`, and
both `cargo doc --no-deps` and `cargo clippy -- -D warnings` pass cleanly.

Issue #75 is **fully resolved on main**. Agent A's #75 scope is a no-op.
Agent A's only remaining work is the hook audit (#74).

**Impact**: Agent A can be simplified to hook audit only. Alternatively,
#75 can be closed without any M13 code changes.

**Recommendation**: Close #75 as already completed on main. Rename Agent A
to just "Hook Audit (#74)".

#### Finding 2 (Medium): Plan says 11 `eprintln!` calls but actual count is 13

**Location**: Consolidated plan line 920, Design Decisions §Structured Warning Channel

The plan states "Replace 11 `eprintln!` calls with `warnings.push(...)`."
Actual count in `src/builder/mod.rs` is **13** `eprintln!` calls. More
importantly, these serve two distinct purposes:

- **3 warning calls** (lines 88, 107, 135): LLM fallback warnings —
  these should become structured warnings in `BuildResult.warnings`
- **10 interactive output calls** (lines 332–400): `interactive_build()`
  UI — purposeful console output (printing name, description, body
  preview, validation results)

The plan says to replace all `eprintln!` with `warnings.push(...)`, but
the interactive output calls are **not warnings** — they're intentional
user-facing output during `interactive_build()`. Pushing these into
`BuildResult.warnings` would be semantically wrong and would silence
the interactive flow.

**Recommendation**: Agent G-alt should only replace the 3 LLM fallback
`eprintln!` calls (lines 88, 107, 135) with `warnings.push(...)`. The 10
interactive output calls should either stay as `eprintln!` or be refactored
to use a separate output mechanism (e.g., a callback or writer), but they
are not "warnings" and should not go into `BuildResult.warnings`.

#### Finding 3 (Medium): Issue #85 body contradicts the plan — says `validate` → `check`

**Location**: Issue #85 body, section "Renames (#76)"

Issue #85 body states:
> `validate` → `check` (with `validate` as hidden alias)

But the consolidated plan (and the #76 design decision at line 857-868)
clearly specifies that `validate` **stays as-is** (spec conformance) and
`check` is a **new superset command** (validate + semantic). These are
differentiated commands, not a rename. `validate` is not being aliased or
replaced.

This is a stale/incorrect claim in the issue body. If not corrected, an
implementer reading only the issue (not the plan) would incorrectly rename
`validate` to `check`.

**Recommendation**: Update issue #85 body to remove the `validate → check`
rename from the list and replace it with: "`check` is a new command
(validate + semantic quality); `validate` stays as-is."

#### Finding 4 (Medium): Issues #83, #84, #85 missing assignment, labels, project fields

**Location**: GitHub issue metadata

These three new issues have incomplete metadata compared to #75-#82:

| Issue | Labels | Assigned | Project Priority/Type |
|-------|--------|----------|----------------------|
| #83 | `enhancement` only (no priority) | ❌ unassigned | Not in project / no fields |
| #84 | `enhancement` only (no priority) | ❌ unassigned | Not in project / no fields |
| #85 | `documentation` only (no priority) | ❌ unassigned | Not in project / no fields |

All other M13 issues (#45, #74, #75-#82) have consistent metadata:
priority labels (`low`/`medium`), assignment, project with Type/Priority/Status.

**Recommendation**: Normalize metadata for #83, #84, #85 to match the
existing M13 issues before implementation begins.

#### Finding 5 (Medium): `check` absorbing `lint` changes the public API surface

**Location**: Wave 3 Agent G, step 4 (line 1020-1022)

The plan says: "Remove the standalone `Lint` variant (absorbed into `Check`)."
Currently `Lint` is a CLI subcommand variant in `Commands` enum. Removing it
and replacing with a `check` alias means:

1. The `aigent lint` command still works (via alias) — backward compatible.
2. But `lint` in `check` mode now also runs `validate` by default. Previously
   `lint` was semantic-only. Users who relied on `aigent lint` returning only
   semantic issues will get structural diagnostics too.

This is a behavioral change, not just a rename. The `--no-validate` flag
mitigates this (`aigent check --no-validate` = old `lint` behavior), but
the default behavior of `aigent lint` changes.

**Recommendation**: Document this behavioral change clearly in the README
(Agent L-doc) and in `CHANGES.md`. Consider whether `aigent lint` (the
alias) should default to `--no-validate` to preserve old behavior, or
whether the superset behavior is the desired default.

#### Finding 6 (Low): `build` subcommand (#83) has a `--multi` flag in the issue but not in the plan

**Location**: Issue #83 body vs. consolidated plan line 943-945

Issue #83 lists a `--multi` flag: "bundle multiple skill directories into
one plugin." The plan's `Build` subcommand struct (lines 579-589) doesn't
include `--multi` — it just takes `skill_dirs: Vec<PathBuf>`, implying
multi-skill is the default when multiple paths are given.

The plan's approach is simpler (implicit multi), but it deviates from the
issue's explicit `--multi` flag. Either is fine, but the discrepancy
should be resolved.

**Recommendation**: Accept the plan's implicit multi approach (simpler)
and update the issue body to match, or explain in the plan why `--multi`
was dropped.

#### Finding 7 (Low): Wave dependency chain creates a long critical path

**Location**: Wave Plan structure (lines 968-1076)

The 5-wave structure creates a serial dependency chain:
Wave 1 → Wave 2 → Wave 3 → Wave 4 → Wave 5

Waves 1 and 2 are independent of each other — nothing in Wave 2 depends
on Wave 1 outputs. The plan orders them sequentially ("Agents D, E, and F
are independent") but doesn't exploit this for parallelism.

**Recommendation**: Consider running Waves 1 and 2 in parallel (on
separate task branches) to reduce the critical path. Wave 3 still
depends on both being complete (since it restructures the CLI surface
that Waves 1-2 modify). This would reduce the total implementation time
without increasing risk.

#### Finding 8 (Low): `fmt` canonical key order omits `context`

**Location**: Design Decisions §`fmt` Subcommand, line 315

The canonical key order is listed as:
> `name`, `description`, `instructions`, `compatibility`, `context`,
> `allowed-tools`, `metadata`

This includes `context`, which is a Claude Code extension field (not in
the base Anthropic spec). When `--target standard` is used with
`validate`, a `context` field triggers a W001 (unknown field) warning.

The formatter should handle this gracefully — it should reorder `context`
into canonical position if present, but not add it. The plan doesn't
mention target-awareness for `fmt`.

**Recommendation**: Document that `fmt` is target-agnostic (it formats
whatever fields exist without judging their validity). This is the right
behavior — `fmt` normalizes form, `validate` checks content.

#### Finding 9 (Low): Post-M13 CLI surface list is missing `read-properties`

**Location**: Consolidated plan line 828

The "Post-M13 CLI" line lists:
> `validate`, `check`, `new`, `prompt`, `probe`, `score`, `doc`, `fmt`,
> `build`, `test`, `upgrade`, `init`, `read-properties`

This is actually correct (13 commands). But the plan doesn't mention what
happens to `read-properties` — it's not renamed, not aliased, not
deprecated. It's just silently carried forward. Given the naming cleanup
effort, this is worth a brief note.

`read-properties` is the only hyphenated command name remaining. While
renaming it isn't necessary (it's a utility/debug command), the plan
should acknowledge it exists and explain why it's unchanged.

#### Finding 10 (Low): Estimated scope may undercount new tests

**Location**: Consolidated plan line 1113

The plan estimates "+45–55" new tests. Let me count from the agent
descriptions:

| Agent | New tests (from plan) |
|-------|----------------------|
| A | 0 (cargo checks serve as verification) |
| B | ~2 (fail label assertions) |
| C | 0 (existing tests cover `read_body`) |
| D | ~3 (trigger boost, name boost, zero score) |
| E | ~3 (full applies, two-section output, write error) |
| F | ~4 (comments preserved, values unchanged, edge cases) |
| G-alt | ~1 (unavailable LLM → warnings) |
| G | ~7 (4 alias tests + 3 check behavior) |
| H | ~5 (idempotent, reorder, --check, preserve, recursive) |
| I | 0 (mechanical updates to existing tests) |
| J | ~5 (single, multi, --validate, plugin.json, copy) |
| K | ~5 (all-pass, failure, missing fixture, JSON, generate) |
| L-doc | 0 |
| L | 0 (manual verification) |

Subtotal: ~35 new tests. But Agent G also needs to update many existing
CLI tests that reference old subcommand names (these aren't "new" tests
but substantial rework). The estimate of 45-55 may be achievable if
agents are thorough, but the lower bound is more like 30–35.

### Observations

1. **The consolidated plan is a substantial improvement over the
   incremental version.** The document structure is clean: design
   decisions section, then wave plan with compact agent descriptions.
   The obsolete plan is retained for historical record, which is good
   practice.

2. **The reconciliation section (lines 750-803) bridges the two versions
   well.** It explicitly documents what changed between the original and
   consolidated plans, including baseline updates, agent scope changes,
   and dependency corrections. This makes the evolution of the plan
   traceable.

3. **The `check` command design (validate + semantic quality) is the
   right abstraction.** The quality spectrum `validate → check → score`
   gives users three levels of analysis. `--no-validate` on `check`
   preserves the old `lint` behavior for users who want semantic-only
   analysis.

4. **The `test` subcommand (#84) has a well-designed YAML schema.**
   The `tests.yml` format with `should_match`, `min_score`, and
   `strength` fields is expressive enough for real test cases while
   staying simple. The `--generate` flag for bootstrapping is a good
   DX touch.

5. **The `build` subcommand (#83) fills a real gap.** Currently there's
   no way to go from skills to a distributable plugin without manual
   scaffolding. The assembler module automates a tedious process.

6. **Agent naming has a collision: G and G-alt.** Wave 2 has "Agent G-alt"
   for structured warnings, and Wave 3 has "Agent G" for CLI renames.
   While the "alt" suffix disambiguates, it suggests the agent was
   added late (which it was, per the reconciliation). Consider renaming
   to avoid confusion in implementation references.

7. **The plan correctly keeps `read-properties` unchanged.** It's a
   debug/utility command with no naming conflict. Renaming it to `props`
   or similar would be gratuitous.

8. **Error handling fix for `upgrade --apply` is well-placed in Agent E.**
   The `unwrap_or_else` at `main.rs:1043` that silently swallows write
   errors is called out as step 4 of Agent E. This is a genuine bug fix
   that should be prioritized within the agent.

9. **The `dev/m13` branch doesn't exist yet.** This is correct — the
   plan specifies creating it from main after v0.2.3, and the branch
   strategy is documented. No premature work has been done.

10. **The issue tracker is slightly out of sync.** Issues #83-85 exist
    with milestone "M13: Enhancements" but lack assignment, priority
    labels, and project fields. The earlier issues (#45, #74, #75-82)
    were cleaned up in this session but the new three were not.

### Verdict

**Approved** — the consolidated plan is thorough, well-structured, and
resolves all prior review findings. The scope expansion from 6 to 12
issues is justified by the addition of three genuine new features (`build`,
`test`, `check`) that leverage the CLI rename work.

Three issues should be addressed before implementation:

1. **Finding 1 (High)**: `#![warn(missing_docs)]` is already on main.
   Agent A's #75 work is a no-op — close #75 or reduce Agent A to hook
   audit only.

2. **Finding 3 (Medium)**: Issue #85 body incorrectly says `validate →
   check`. This must be corrected to prevent implementation errors.

3. **Finding 2 (Medium)**: `eprintln!` count is 13, not 11, and only 3
   are actual warnings. Agent G-alt scope should target only the 3 LLM
   fallback warnings, not all 13 calls.

All other findings are advisory or low severity.

### Pre-Implementation Checklist

- [ ] Finding 1: Close #75 or acknowledge as already done; reduce Agent A
- [ ] Finding 2: Correct Agent G-alt scope to 3 LLM fallback warnings only
- [ ] Finding 3: Update issue #85 body (remove `validate → check` rename claim)
- [ ] Finding 4: Add assignment, labels, project fields to #83, #84, #85
- [ ] Finding 5: Decide on `aigent lint` alias default behavior (superset vs. semantic-only)
- [ ] Finding 6: Resolve `--multi` flag discrepancy between #83 and plan
- [ ] Finding 7: Consider running Waves 1 and 2 in parallel
- [ ] Finding 8: Confirm `fmt` is target-agnostic (formats all fields present)
- [ ] Finding 10: Recalibrate test count estimate (30–35 realistic lower bound)

---

## Code Review (2026-02-21)

Review of all code changes on `dev/m13` branch (20 commits since `main`).

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --check` | ✅ Clean |
| `cargo clippy -- -D warnings` | ✅ Clean |
| `cargo test` | ✅ 476 tests (342 unit + 106 CLI + 27 plugin + 1 doc-test) |
| `cargo doc --no-deps` | ✅ Clean |

**Test delta**: +60 tests (416 → 476). Consistent with plan estimate of +45–55
after accounting for the test runner and assembler suites.

### Diff Summary

+4500/−232 across 17 files. 3 new modules, 2 new support files.

| File | Status | Lines | Issue |
|------|--------|-------|-------|
| `src/assembler.rs` | NEW | 366 | #83 |
| `src/formatter.rs` | NEW | 405 | #76 |
| `src/test_runner.rs` | NEW | 320 | #84 |
| `src/tester.rs` | MAJOR | 490 | #79 |
| `src/scorer.rs` | MODIFIED | 593 | #78 |
| `src/main.rs` | MAJOR | 1427 | #76, #80, #81, #83, #84 |
| `src/lib.rs` | MODIFIED | 92 | (all) |
| `src/builder/mod.rs` | MODIFIED | — | #45 |
| `src/parser.rs` | MODIFIED | — | (dedup) |
| `src/structure.rs` | MODIFIED | — | (dedup) |
| `tests/cli.rs` | MAJOR | +635 | #76, #80, #81, #83, #84 |
| `tests/plugin.rs` | MODIFIED | +37 | #74 |
| `hooks/hooks.json` | RESTRUCTURED | 16 | #74 |
| `scripts/bump-version.sh` | NEW | 94 | #82 |
| `skills/aigent-builder/SKILL.md` | MODIFIED | — | #76 |

### Plan Conformance

#### Issue-by-Issue Status

| Issue | Title | Plan Status | Code Status | Notes |
|-------|-------|-------------|-------------|-------|
| #45 | Structured warning channel | Wave 2 Agent G-alt | ✅ Implemented | `BuildResult.warnings: Vec<String>`, 3 `eprintln!` → `warnings.push(...)` |
| #74 | Hook variable audit | Wave 1 Agent A | ✅ Implemented | `hooks.json` restructured, stdin-based jq, no `$TOOL_INPUT` |
| #76 | CLI naming alignment | Wave 3 Agents G/H/I | ✅ Implemented | All renames + `fmt`/`check` + aliases; see deviations below |
| #78 | Score check labels | Wave 1 Agent B | ✅ Implemented | `fail_label: Option<String>`, `display_label()`, all 11 checks |
| #79 | Tester weighted scoring | Wave 2 Agent D | ⚠️ Deviations | Algorithm differs from plan; see F1 |
| #80 | Upgrade --full | Wave 2 Agent E | ✅ Implemented | Two-section output, `apply_fixes`, write error uses `?` |
| #81 | YAML parser edge cases | Wave 2 Agent F | ✅ Implemented | `extract_frontmatter_lines()`, `detect_indent()`, `find_metadata_insert_position()` |
| #82 | Version management | Wave 1 Agent C | ✅ Implemented | `scripts/bump-version.sh` (94 lines) |
| #83 | Plugin assembly | Wave 4 Agent J | ✅ Implemented | `src/assembler.rs` (366 lines), `build` subcommand |
| #84 | Fixture-based testing | Wave 4 Agent K | ✅ Implemented | `src/test_runner.rs` (320 lines), `test` subcommand |
| #85 | CLI renames meta-issue | Wave 3 | ✅ Covered | Tracked via #76 implementation |

**11/11 issues addressed.** All M13 milestone issues have corresponding code
changes. The scope is complete.

#### Alias Conformance

| Old Name | New Name | Plan: Hidden Alias | Code: Hidden Alias | Status |
|----------|----------|-------------------|-------------------|--------|
| `build` | `new` | `build` | `create` | ⚠️ **Deviation** — alias is `create`, not `build` |
| `to-prompt` | `prompt` | `to-prompt` | `to-prompt` | ✅ Match |
| `test` | `probe` | `test` | (none) | ⚠️ **Missing** — no backward-compat alias |
| `lint` | `check` | `lint` | `lint` | ✅ Match |
| (new) | `fmt` | `format` | `format` | ✅ Match |

**Two alias deviations**:
1. `new` has alias `create` instead of `build`. The old `build` name is now
   the plugin assembly command, making `build` as alias for `new` impossible.
   This is a correct design decision but a deliberate deviation from the
   plan's literal text.
2. `probe` has no hidden alias for `test`. The old `test` name is now the
   fixture runner, making `test` as alias for `probe` impossible. Same logic
   as above — correct but plan didn't account for the name collision.

Both deviations are logical consequences of reusing freed names. The plan said
"old preserved as hidden alias" but didn't anticipate that `build` and `test`
would be assigned to new commands. No action needed.

### Prior Review Finding Resolution

Validation of the 10 plan-review findings from the consolidated review against
actual code implementation:

| Finding | Severity | Plan Review Finding | Code Resolution | Status |
|---------|----------|-------------------|-----------------|--------|
| F1 | High | `#![warn(missing_docs)]` already on main | N/A — attribute was already present, no code change needed | ✅ Confirmed |
| F2 | Medium | `eprintln!` count is 13, not 11 | Code correctly replaces only 3 LLM fallback calls; 10 interactive calls untouched | ✅ Resolved correctly |
| F3 | Medium | Issue #85 says `validate → check` | `validate` stays as-is in code; `check` is new command | ✅ Resolved correctly |
| F4 | Medium | Issues #83-85 missing metadata | Not verifiable in code — GitHub metadata issue | ⚪ N/A |
| F5 | Medium | `check` absorbing `lint` changes API | `lint` alias maps to `check` (superset); `--no-validate` restores old semantic-only behavior | ✅ Resolved |
| F6 | Low | `build` --multi flag discrepancy | Code uses `Vec<PathBuf>` (implicit multi) — simpler than explicit `--multi` | ✅ Resolved |
| F7 | Low | Wave dependency chain | Waves executed successfully (tests pass) | ✅ N/A |
| F8 | Low | `fmt` key order omits `context` | `KEY_ORDER` includes `context` at position 5 | ✅ Resolved |
| F9 | Low | Post-M13 CLI missing `read-properties` | `ReadProperties` variant unchanged in code | ✅ Confirmed |
| F10 | Low | Test count estimate | 476 − 416 = 60 new tests (above 45–55 estimate) | ✅ Exceeded |

**All 10 findings resolved or confirmed.**

### Code Review Findings

#### F1 (Medium): Tester scoring formula deviates from plan specification

**Location**: `src/tester.rs:227-285`

The plan specifies (line 923):
```
score = 0.5 * jaccard(query, description) + 0.3 * trigger_match + 0.2 * name_match
```
With thresholds: Strong ≥ 0.5, Weak ≥ 0.2, None < 0.2.

The code implements:
```
score = 0.5 * recall(query, description) + 0.3 * trigger_match + 0.2 * name_match
```
With thresholds: Strong ≥ 0.4, Weak ≥ 0.15, None < 0.15.

Two differences:
1. **Recall instead of Jaccard**: The code computes `intersection / query_set_len`
   (recall) rather than `intersection / union_len` (Jaccard). Recall is
   arguably better for this use case — it measures "what fraction of query
   terms appear in the description" without penalizing descriptions that
   contain additional terms. Jaccard would penalize long descriptions.

2. **Lower thresholds**: Strong moved from 0.5 to 0.4, Weak from 0.2 to 0.15.
   This makes matching more permissive. All 13 unit tests pass with these
   thresholds, and the fixture-based CLI tests also pass, suggesting the
   thresholds are calibrated.

**Assessment**: The deviations are improvements over the plan specification.
Recall is more appropriate than Jaccard for asymmetric matching (short query
against long description). The lower thresholds compensate for recall's
higher base values compared to Jaccard. The plan should be updated to reflect
the actual implementation, or a design note should be added.

**Severity**: Medium (specification drift, but functionally correct).

#### F2 (Medium): `Probe` JSON output omits `score` field

**Location**: `src/main.rs:724-735`

`TestResult` now includes `pub score: f64` (the weighted match score), and
the text formatter shows it (`score: {:.2}`). But the JSON output for the
`probe` command does not include the `score` field:

```rust
let json = serde_json::json!({
    "name": result.name,
    "query": result.query,
    "description": result.description,
    "activation": format!("{:?}", result.query_match),
    "estimated_tokens": result.estimated_tokens,
    "validation_errors": ...,
    "validation_warnings": ...,
    "structure_issues": ...,
});
```

The `score` field is available as `result.score` but not serialized. API
consumers using `--format json` can see the activation category but not the
numeric score. This reduces the utility of the JSON output for programmatic
analysis (e.g., threshold tuning, test fixture calibration).

**Recommendation**: Add `"score": result.score` to the JSON object.

#### F3 (Medium): `assembler.rs` uses `eprintln!` for warnings — same pattern #45 fixed

**Location**: `src/assembler.rs:69,73,120`

Issue #45 explicitly replaced `eprintln!` warnings in `builder/mod.rs` with
`warnings.push(...)` on `BuildResult`. The new `assembler.rs` introduces 3
new `eprintln!` calls for the same purpose (skipping invalid skills, missing
SKILL.md, validation diagnostics):

```rust
eprintln!("warning: skipping {}: {e}", dir.display());  // line 69
eprintln!("warning: no SKILL.md in {}", dir.display());  // line 73
eprintln!("{name}: {d}");  // line 120
```

These calls bypass the structured warning pattern that #45 established.
Library consumers of `assemble_plugin()` cannot capture or suppress these
warnings — they go directly to stderr. `AssembleResult` has no `warnings`
field.

**Recommendation**: Add `warnings: Vec<String>` to `AssembleResult` (matching
`BuildResult` pattern) and replace the 3 `eprintln!` calls with structured
warnings. The CLI layer then prints them to stderr.

#### F4 (Medium): `generate_plugin_json()` uses string formatting — potential JSON injection

**Location**: `src/assembler.rs:190-209`

```rust
fn generate_plugin_json(name: &str, skills: &[(String, PathBuf)]) -> String {
    format!(r#"{{"name": "{name}", ...}}"#, name = name, ...)
}
```

The `name` parameter is interpolated directly into JSON via `format!()` with
no escaping. If a plugin name or skill name contains double quotes, backslashes,
or control characters, the generated `plugin.json` would be malformed JSON.

Currently, skill names are validated (lowercase + hyphens only via parser), so
this is unlikely to trigger. But the plugin name can be overridden with
`--name <arbitrary>`, which bypasses validation. For example:
```
aigent build skill/ --name 'my "plugin'
```
would produce invalid JSON.

**Recommendation**: Use `serde_json::to_string_pretty()` to generate
`plugin.json` instead of string formatting. Define a `PluginManifest` struct
with `#[derive(Serialize)]`. This is ~5 lines of code change and eliminates
the entire class of injection issues.

#### F5 (Low): `validate` command lost `--lint` flag without deprecation

**Location**: `src/main.rs` diff, lines ~89-92 (removed)

The `Validate` variant previously had a `--lint` flag. The code removes it
entirely (no deprecation period, no hidden alias). Users who relied on
`aigent validate --lint` will get an error. The migration path is
`aigent check`, which runs validate + lint by default.

The plan's intent was clear (absorb lint into check), but removing the flag
from validate without any deprecation notice is a breaking change. Pre-M13
users may have `--lint` in scripts.

**Assessment**: For pre-1.0 software this is acceptable, but `CHANGES.md`
should document it clearly.

#### F6 (Low): `upgrade --full` output doesn't clearly separate sections

**Location**: `src/main.rs:1267-1306`

The plan review Finding 5 asked for two-section output separating validate
fixes from upgrade suggestions. The implementation prefixes validate/lint
items with `[full]`:

```rust
suggestions.push(format!("[full] Applied {fix_count} validation/lint fix(es)"));
suggestions.push(format!("[full] error: {d}"));
suggestions.push(format!("[full] warning: {d}"));
```

This is a reasonable compromise — the `[full]` prefix distinguishes the
source, and all items share the same `suggestions` Vec. But it's not the
two-section visual layout the plan specified:

```
Validation fixes applied (--full):
  ✓ Fixed name casing: MySkill → my-skill

Upgrade suggestions:
  ⚠ Missing 'compatibility' field
```

**Assessment**: Functional but not visually separated. The current approach
is simpler to implement and test. Acceptable for M13; visual improvement
could follow.

#### F7 (Low): `formatter.rs` KEY_ORDER doesn't include `argument-hint`

**Location**: `src/formatter.rs:24-33`

The `KEY_ORDER` constant lists 8 known keys. The `argument-hint` key (a
Claude Code extension) is not included. When present, `argument-hint` will
be sorted alphabetically among unknown keys, placing it before `compatibility`
instead of after `allowed-tools` where it logically belongs.

The formatter is target-agnostic by design (formats whatever fields exist),
so this is by-design behavior. But `argument-hint` is listed in
`CLAUDE_CODE_KEYS` in `parser.rs` and used in `builder/template.rs`. It's a
known key that the formatter doesn't know about.

**Recommendation**: Either add `argument-hint` to `KEY_ORDER` (after
`allowed-tools`) or document the intentional omission with a comment.

#### F8 (Low): `copy_skill_files` skips `tests.yml` during plugin assembly

**Location**: `src/assembler.rs:146-158`

The `copy_skill_files()` function copies all non-SKILL.md sibling files from
a skill directory to the assembled plugin. This includes `tests.yml` if
present, which is a development artifact (test fixtures) not a distributable
asset. Including test fixtures in the assembled plugin increases its size
without benefit.

```rust
if name_str == "SKILL.md" || name_str == "skill.md"
    || name_str.starts_with('.') || name_str == "target" {
    continue;
}
```

The skip list filters SKILL.md, hidden files, and `target/` but not
`tests.yml`.

**Assessment**: Minor — test fixtures in a plugin don't cause errors, just
add unnecessary files. Consider adding `tests.yml` to the skip list.

#### F9 (Low): `detect_indent` returns first-found indentation, not most-common

**Location**: `src/main.rs:1202-1210`

The `detect_indent()` function returns the indentation of the first indented
line it finds. If a frontmatter has mixed indentation (e.g., 2-space for most
keys but 4-space for one), it will use whatever comes first rather than the
dominant pattern. This is a fragile heuristic.

In practice, YAML files almost always use consistent indentation, so this is
unlikely to cause issues. But the function's doc comment says "detects the
indentation style used" which implies reliability.

**Assessment**: Acceptable for M13. The heuristic covers the common case.

#### F10 (Low): `test` exit code doesn't distinguish fixture errors from test failures

**Location**: `src/main.rs:849-851`

The plan specified exit codes: 0 = all pass, 1 = failure, 2 = fixture error.
The code uses exit 1 for both:

```rust
if total_failed > 0 || any_error {
    std::process::exit(1);
}
```

CI scripts can't distinguish between "tests ran and some failed" (actionable)
vs. "tests.yml couldn't be parsed" (infrastructure error). Both get exit 1.

**Assessment**: Minor deviation from plan. Exit code 2 for fixture errors
would improve CI integration but isn't critical for M13.

### Observations

1. **Code quality is consistently high.** All new modules follow project
   conventions: `///` doc comments on public items, `#[must_use]` where
   appropriate, `Result<T>` propagation with `?`, no `unwrap()` in library
   code. Clippy and doc generation pass cleanly.

2. **The `formatter.rs` module is well-designed.** The `YamlBlock` enum for
   parsing YAML frontmatter is a clean abstraction. The separation between
   `format_skill()` (filesystem-aware) and `format_content()` (pure string
   transform) follows the same pattern as `validate_structure()` vs.
   `validate()`. The idempotency test ensures reformatting stability.

3. **The YAML manipulation helpers in `main.rs` are solid.** The
   `extract_frontmatter_lines()`, `detect_indent()`, and
   `find_metadata_insert_position()` trio handles the #81 edge cases well:
   comment-aware scanning, indentation detection, metadata block locating.
   The CLI tests cover comments, partial metadata, 4-space indentation,
   and no-metadata-block scenarios.

4. **The `test_runner.rs` module integrates cleanly with tester.** It reuses
   `tester::test_skill()` for the actual matching, adding only the fixture
   parsing and result comparison layer. The `--generate` flag is a good DX
   touch that creates starter fixtures from skill metadata.

5. **The CLI restructuring is comprehensive.** 106 CLI tests cover all the
   new commands, aliases, and edge cases. The test organization uses clear
   section headers (`// ── M13: YAML parser edge cases (#81) ──────`).

6. **The `read_body()` deduplication is clean.** Three copies reduced to
   one `pub fn read_body()` in `parser.rs`, with `structure.rs` and
   `scorer.rs` both calling `crate::parser::read_body()`. This was the
   clearest cleanup target from M12 review.

7. **The `builder/mod.rs` structured warnings are minimal and correct.**
   Only the 3 LLM fallback `eprintln!` calls were replaced with
   `warnings.push(...)`. The 10 interactive output calls were correctly
   left untouched. The CLI prints warnings with the `warning:` prefix
   (line 648).

8. **The `hooks.json` restructuring follows the Claude Code plugin schema.**
   The new format (`{"description": ..., "hooks": {"PostToolUse": [...]}}`)
   is the correct schema. The stdin-based `jq` approach (no `$TOOL_INPUT`
   env variable) is more robust.

9. **The `bump-version.sh` script is well-structured.** It validates semver
   format, updates 3 files atomically, regenerates `Cargo.lock`, and is
   idempotent. Uses `sed -i ''` for macOS compatibility.

10. **The scorer's `read_body()` now uses the shared parser function.**
    Line 110 of `scorer.rs` calls `crate::parser::read_body(dir)` instead
    of a local copy. The `fail_label` additions are backward-compatible
    (JSON serialization unchanged via `skip_serializing_if`).

### Verdict

**Approved.** The M13 code changes are well-implemented, comprehensive, and
follow project conventions. All 11 milestone issues have corresponding code
changes. Verification passes (476 tests, clean clippy/fmt/doc).

**Should fix before merge**:

- F2 (Medium): Add `score` field to `probe` JSON output. One-line change.
- F4 (Medium): Replace `format!()` with `serde_json` in `generate_plugin_json()`.
  Prevents potential JSON injection via `--name`.

**Should consider before merge**:

- F3 (Medium): Add `warnings` to `AssembleResult` (matching `BuildResult`
  pattern from #45). The new `assembler.rs` reintroduces the same
  `eprintln!` pattern that #45 specifically fixed.
- F1 (Medium): Update plan to reflect actual scoring formula (recall, not
  Jaccard; lower thresholds). Or add a design note explaining the deviation.

**Advisory (post-merge)**:

- F5: Document `--lint` removal in CHANGES.md
- F6: Visual separation for `--full` output
- F7: Add `argument-hint` to formatter KEY_ORDER
- F8: Skip `tests.yml` in plugin assembly
- F9: Improve indent detection heuristic
- F10: Differentiated exit codes for test runner

### Summary Statistics

| Metric | Value |
|--------|-------|
| Issues addressed | 11/11 |
| New modules | 3 (`assembler.rs`, `formatter.rs`, `test_runner.rs`) |
| Lines added | ~4500 |
| Lines removed | ~232 |
| Tests added | +60 (416 → 476) |
| Findings | 10 (0 high, 4 medium, 6 low) |
| Plan deviations | 2 (alias naming, scoring formula) |
| Verification | All 4 checks pass |
