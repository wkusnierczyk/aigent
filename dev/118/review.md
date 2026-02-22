# Review: PR #123 — Fix version.sh heading match and verification

Reviewed branch `dev/118-version` (`c9c9554`) against `main` (`2e3340b`).

---

## Verdict

Clean, well-targeted fix. Root cause correctly identified and resolved.
Two minor observations (§3.1, §3.2) — neither blocking.

| Dimension | Rating | Notes |
|-----------|:------:|-------|
| Correctness | ✅ | Root cause fixed, verified manually |
| Completeness | ✅ | Three independent fixes in one commit |
| Risk | Low | Single file, no behavioral change on success path |

---

## 1. Root Cause Analysis

The bug was a **case-sensitivity mismatch** in the awk pattern, not an
incremental compilation issue as originally suspected in the issue.

| Component | Expected | Actual |
|-----------|----------|--------|
| Awk pattern | `## About and Licence` | — |
| README heading | — | `## About and licence` |

The awk pattern never matched, so the README passed through unchanged.
The script then reported "Updated README.md" — a false positive, because
the success message was unconditional (it only checked that the `--about`
output was non-empty, not that the replacement happened).

**Testing confirmed** that `cargo build` does correctly pick up
`Cargo.toml` version changes — the incremental compilation hypothesis
from the issue was ruled out.

---

## 2. Changes Review

### 2.1 Case-insensitive awk match (line 109)

```awk
/^## About and [Ll]icence/ { in_section=1; print; next }
```

Correct fix. Matches both `Licence` and `licence`. An alternative would
be `tolower($0)` matching, but the character class is simpler and
sufficient — the heading won't vary beyond this one letter.

### 2.2 Force rebuild with `touch` (line 90)

```bash
touch "$ROOT/src/main.rs"
```

Safety net. While testing showed Cargo does detect `Cargo.toml` changes,
touching `main.rs` guarantees a recompile of the binary that embeds
`env!("CARGO_PKG_VERSION")`. Low cost (one extra recompile), high
assurance.

### 2.3 Binary version verification (lines 96–102)

```bash
if ! grep -q "version:.*$VERSION" "$ABOUT_FILE"; then
    echo "Error: built binary reports wrong version (expected $VERSION)" >&2
    cat "$ABOUT_FILE" >&2
    rm -f "$ABOUT_FILE"
    exit 1
fi
```

Good fail-fast guard. Catches the case where the build succeeded but
the binary somehow reports the wrong version. Prints the actual output
to stderr for debugging.

### 2.4 README replacement verification (lines 118–123)

```bash
if ! grep -q "version:.*$VERSION" "$TMPFILE"; then
    echo "Error: README --about block not updated (heading mismatch?)" >&2
    rm -f "$TMPFILE" "$ABOUT_FILE"
    exit 1
fi
```

Catches the exact failure mode that caused the original bug — awk
replacement silently doing nothing. With this guard, the script would
have caught the case mismatch immediately. Temp files are cleaned up
on the error path.

### 2.5 Removed `2>/dev/null` from `cargo build` (line 91)

Build errors are now visible. The `2>/dev/null` on `aigent --about`
(line 94) is correctly retained — stderr from the binary is not part
of the `--about` output.

---

## 3. Observations

### 3.1 Remaining `2>/dev/null` on `cargo check` (line 156)

```bash
(cd "$ROOT" && cargo check --quiet 2>/dev/null)
```

Step 5 (Cargo.lock regeneration) still suppresses stderr. If `cargo
check` fails here, `set -e` will catch the non-zero exit from the
subshell and abort the script — so the error isn't truly silent. But
the user won't see *why* it failed. Minor — `cargo check` at this
point is unlikely to fail since `cargo build` already succeeded two
lines earlier.

### 3.2 Regex dots in version grep

```bash
grep -q "version:.*$VERSION" "$ABOUT_FILE"
```

The dots in `$VERSION` (e.g., `0.4.1`) are regex wildcards, so `0.4.1`
would also match `0X4Y1`. In practice this is irrelevant — the version
string comes from a validated semver input and the about output is
structured. Not worth quoting with `grep -F` since the `.*` before
`$VERSION` requires regex mode anyway.

---

## 4. Scope

| Metric | Value |
|--------|-------|
| Files changed | 1 |
| Lines added | 18 |
| Lines removed | 4 |
| Net delta | +14 |

---

## 5. Summary

Root cause was simpler than suspected — a case mismatch, not a build
cache issue. The fix addresses the root cause and adds two independent
verification layers that prevent silent failures. No blocking issues.
Ready to merge.
