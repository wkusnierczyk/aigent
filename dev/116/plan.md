# Plan: Default to current directory when no skill path is given (#116)

## Context

Commands that take skill directory paths (`validate`, `check`, `score`,
`format`, `probe`, `upgrade`, `properties`, `prompt`, `doc`, `build`,
`test`) currently require at least one explicit path argument. When
working inside a skill directory, users should be able to omit the path
and have `aigent` default to the current directory (`.`).

## Design

### Approach: default value in clap

Use clap's `default_value = "."` on positional arguments. This is the
simplest approach — no runtime logic needed, clap handles it
transparently. The usage line changes from `<skill-dir> [<skill-dir>...]`
to `[<skill-dir>...]` (or `[<skill-dir>]` for single-path commands),
signaling that the argument is optional.

### Commands affected

Two argument patterns exist:

| Pattern | Commands |
|---------|----------|
| `skill_dirs: Vec<PathBuf>` | `validate`, `check`, `prompt`, `doc`, `build`, `test`, `fmt` |
| `skill_dir: PathBuf` | `properties`, `score`, `upgrade` |

### Commands NOT affected

| Command | Why |
|---------|-----|
| `new` | Takes a `purpose` string, not a skill dir |
| `init` | Already has `dir: Option<PathBuf>` defaulting to `.` |
| `probe` | Has two positionals (`skill_dir`, `query`); see §Appendix |

## Changes

### 1. Multi-path commands: add `default_value = "."` (`src/main.rs`)

For each `skill_dirs: Vec<PathBuf>` field, add the clap attribute:

```rust
#[arg(default_value = ".")]
skill_dirs: Vec<PathBuf>,
```

This makes the argument optional. When omitted, `skill_dirs` defaults
to `vec![PathBuf::from(".")]`. When provided, it works as before.

**Commands**: `Validate`, `Check`, `Prompt`, `Doc`, `Build`, `Test`, `Fmt`

### 2. Single-path commands: add `default_value = "."` (`src/main.rs`)

For each `skill_dir: PathBuf` field, add the clap attribute:

```rust
#[arg(name = "skill-dir", default_value = ".")]
skill_dir: PathBuf,
```

**Commands**: `Properties`, `Score`, `Upgrade`

### 3. Update CLI tests (`tests/cli.rs`)

Add tests that verify each command works when invoked from inside a skill
directory without an explicit path argument. Test at least:

- `validate` with no args from a skill directory
- `properties` with no args from a skill directory
- `fmt --check` with no args from a skill directory

### 4. Update README (`README.md`)

Update the CLI reference to show that path arguments are optional and
default to the current directory.

## Files to modify

| File | Change |
|------|--------|
| `src/main.rs` | Add `default_value = "."` to 10 commands |
| `tests/cli.rs` | Add tests for default directory behavior |
| `README.md` | Update CLI reference |

## Verification

```bash
cargo test                          # all tests pass
cargo clippy -- -D warnings         # no warnings
cd test-skill/ && aigent validate   # works with no args
cd test-skill/ && aigent properties # works with no args
```

## Appendix: Why `probe` is excluded

`Probe` has two positional arguments: `<skill-dir> <query>`. Adding
`default_value = "."` to `skill_dir` creates an ambiguity: `aigent probe
"hello"` would parse `"hello"` as `skill_dir` (clap fills positionals
left-to-right), leaving `query` missing — a parse error.

### Options considered

| Approach | Usage | Breaking? | Complexity |
|----------|-------|:---------:|:----------:|
| A. Skip `probe` | No change | No | None |
| B. Make `skill_dir` a `--dir` flag | `aigent probe "hello"` / `aigent probe --dir my-skill/ "hello"` | Yes | Low |
| C. Swap positional order | `aigent probe "hello"` / `aigent probe "hello" my-skill/` | Yes | Low |
| D. Collect into `Vec<String>`, disambiguate by arg count | 1 arg = query, 2 args = dir + query | No | Medium |

### Decision: Option A (skip)

The cost-benefit doesn't justify the complexity:

- `probe` always requires a query, so the user is never truly "omitting
  all args" — defaulting the dir saves one arg out of two.
- Options B and C break the existing `aigent probe my-skill/ "query"`
  interface.
- Option D requires custom runtime parsing and degrades clap's help text.
- `probe` can be revisited independently if the CLI contract is
  reconsidered in a future milestone.
