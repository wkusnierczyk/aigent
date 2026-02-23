## Security Review (Whole Codebase)

### Scope
- Reviewed `src/` and key scripts for filesystem safety, path traversal, command execution, and untrusted-input handling.
- Focused on runtime code paths (not test-only code).

### Findings

1. **High** — Path traversal is not blocked for `plugin.json` path overrides
- Files: `src/plugin/manifest.rs:223`, `src/plugin/manifest.rs:239`
- The validator checks only:
  - absolute paths (`P006`), and
  - existence (`P007`).
- It does **not** reject relative escapes like `../outside`.
- Example: `"hooks": "../secrets/hooks.json"` passes `P006` (not absolute) and can pass `P007` if the path exists.
- Security impact: a plugin manifest can reference files outside plugin root while still passing manifest validation, undermining the safety guarantees of `validate-plugin`.
- Recommended fix:
  - canonicalize `plugin_dir.join(value)` and ensure it stays under canonicalized `plugin_dir`.
  - reject any override containing `..` path components.
  - promote this to an error-level diagnostic.

2. **Medium** — Unbounded file reads in plugin validators enable memory DoS
- Files:
  - `src/plugin/manifest.rs:120`
  - `src/plugin/hooks.rs:62`
  - `src/plugin/agent.rs:36`
  - `src/plugin/command.rs:123`
  - `src/plugin/cross.rs:139`
- These paths use `std::fs::read_to_string` without size limits.
- In contrast, `SKILL.md` parsing already has a 1 MiB guard (`src/parser.rs:10`, `src/parser.rs:16`).
- Security impact: validating a malicious plugin directory containing very large files can cause excessive memory allocation or process termination.
- Recommended fix:
  - add a shared bounded-read helper for plugin files (same pattern as `read_file_checked`),
  - apply per-file size caps (e.g., `plugin.json`, `hooks.json`, agent/command markdown files).

3. **Low** — TOCTOU window between safe path selection and write operations
- Files:
  - `src/parser.rs:32` (safe file selection via `is_regular_file`)
  - `src/fixer.rs:87` (writes back to selected path)
  - `src/main.rs:1031`, `src/main.rs:1032` (format command write path)
- Flow validates a regular file first, then later writes by path. A local attacker with filesystem race capability can swap path targets between check and write.
- Security impact: potential unintended overwrite if the process has write access and attacker can race symlink/file replacement.
- Recommended fix:
  - open file descriptors with no-follow semantics where available,
  - re-check file type immediately before write,
  - consider atomic temp-file + rename strategy with parent-dir constraints.

### Notes
- No direct shell command execution from untrusted input was found in Rust runtime paths.
- LLM provider network usage appears explicit and env-configured.
