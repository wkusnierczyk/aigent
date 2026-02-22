## Review: `fix/141-probe`

### Findings

1. **High** — UTF-8 slicing can panic in wrapped probe output
   - File: `src/tester.rs:129`, `src/tester.rs:131`, `src/tester.rs:123`
   - `fmt_field` treats `width`/`max_val` as byte offsets (`remaining[..max_val]`, `remaining[..break_at]`) while Rust `str` requires slicing on UTF-8 character boundaries. If `max_val` lands inside a multibyte character (for example `✓`, `⚠`, `✗`, `—` already present in `match_label`), the probe path can panic at runtime when wrapping is triggered.
   - Impact: `aigent probe` can crash for valid non-ASCII content and/or narrow widths.

### Residual risk

- Current tests in `src/tester.rs` and `tests/cli.rs` cover ASCII wrapping and alignment only; they do not exercise multibyte UTF-8 wrapping paths where the panic occurs.

---

## Review: Branch `fix/141-probe` (implementation)

Reviewed `fix/141-probe` (`331bc0c`, 1 commit) against `main` (`3e730ab`).

Diff: 2 files changed, +158/−11 lines.

### Test results

- Unit tests: 523 (was 413 on baseline, +110 — branch is based on a post-M15 main)
- CLI tests: 140 (was 120, +20)
- Total: 691, all passing
- Clippy: clean
- Formatting: clean

### What it does

Adds word-wrapping to probe text output. Long field values (especially
`Description:` and `Activation:`) now wrap at 80 columns with continuation
lines aligned to the value column (14 spaces indent = label width 13 + 1).

Key changes:
- New `fmt_field(out, label, value, col, width)` function handles wrapping
- `format_test_result` delegates to `format_test_result_width` with
  `DEFAULT_WIDTH = 80`
- `format_test_result_width` is `pub` for testability at custom widths

### Findings

1. **High: UTF-8 slicing panic** — Confirmed by prior reviewer. The `fmt_field`
   function uses byte-based slicing on `&str`:

   ```rust
   let chunk = &remaining[..max_val];          // line 129
   out.push_str(&remaining[..break_at]);       // line 131
   ```

   `max_val` is derived from `width - indent` which assumes characters ≈ bytes.
   When `value` contains multibyte characters (the Activation field always does:
   `✓` = 3 bytes, `⚠` = 3 bytes, `✗` = 3 bytes, `—` = 3 bytes), `max_val`
   may land mid-character, causing a panic.

   **Concrete trigger**: Activation label for `QueryMatch::Weak` is:
   ```
   WEAK ⚠ — some overlap, but description may not trigger reliably (score: 0.XX)
   ```
   This is 80 visible characters but ~86 bytes (⚠ and — are 3 bytes each).
   At `DEFAULT_WIDTH = 80`, `max_val = 80 - 14 = 66`. The `value.len()`
   check (`86 + 14 = 100 > 80`) triggers wrapping, then `remaining[..66]`
   lands on byte 66, which depending on whitespace alignment could be inside
   a multibyte sequence.

   **Fix options:**
   - Use `char_indices()` to find the break point by character count, not bytes
   - Use the `textwrap` crate (already battle-tested for Unicode)
   - At minimum, snap `max_val` to the nearest char boundary:
     `while !remaining.is_char_boundary(max_val) { max_val -= 1; }`

2. **Low: mixed byte/char semantics in no-wrap check** — The early return
   condition (`value.len() + indent <= width`) compares byte length against
   column width. A 50-character string with multibyte chars might have
   `len() = 60` bytes, triggering unnecessary wrapping when it would fit
   visually. Not a panic, but incorrect line-breaking decisions for non-ASCII
   content.

   **Fix**: Use `value.chars().count()` or `unicode_width::UnicodeWidthStr`
   for the display-width comparison.

3. **Low: `format_test_result_width` public API** — This function is
   `pub` but only used internally for testing. Consider `pub(crate)` or
   `#[doc(hidden)]` to avoid expanding the public API surface.

### Code quality

- **Wrapping algorithm**: Standard greedy word-wrap — finds last space within
  `max_val`, breaks there, continues with indent. Falls back to hard break
  at `max_val` if no space found. Correct for ASCII.

- **`DEFAULT_WIDTH = 80`**: Hardcoded. Could read from terminal width (e.g.,
  `terminal_size` crate), but 80 is a safe default for a CLI tool. Acceptable.

- **Test coverage**: 3 unit tests (`fmt_field_short_value_no_wrap`,
  `fmt_field_long_value_wraps_aligned`, `format_test_result_wraps_description`)
  and 1 CLI test (`probe_wraps_long_description_aligned`). All ASCII-only —
  no test exercises the multibyte path where the panic occurs.

### Validation performed

- `cargo test` — 691 tests pass
- `cargo clippy -- -D warnings` — clean
- `cargo fmt --check` — clean
- Read full diff (158 lines added)

### Summary

| Dimension | Rating |
|-----------|:------:|
| Correctness | ❌ |
| Design | ✅ |
| Test coverage | ⚠️ |

The wrapping design and alignment are correct for ASCII content. The UTF-8
slicing bug is a blocking issue — `aigent probe` will panic on the
Activation field at narrow terminal widths (or any field containing multibyte
characters that triggers wrapping). Must fix before merge.
