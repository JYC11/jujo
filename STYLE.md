# Style Guide

Design goals, in order: **safety, performance, developer experience**.

Adapted from [TigerBeetle's Tiger Style](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md) for Rust.

## Safety

### Assertions and invariants

- **Assert pre/postconditions and invariants.** Use `debug_assert!` for expensive checks, `assert!` for cheap production checks.
- **Assert positive AND negative space.** Tests must cover valid, invalid, and boundary data.
- **Split compound assertions.** `assert!(a); assert!(b);` over `assert!(a && b);` for precise failures.

### Limits and bounds

- **Put a limit on everything.** Loops, buffers, retries — all need a fixed upper bound. Every `Vec` from user input needs a max capacity.

### Control flow

- **Simple, explicit control flow.** Minimize nesting. No recursion unless inherently recursive (and bounded).
- **Split compound conditions.** Prefer separate guard clauses over `if a && b`.
- **State invariants positively.** `if index < length` (holds) over `if index >= length` (doesn't).
- **Push `if`s up and `for`s down.** Parent functions own control flow; helpers own computation.

### Error handling

- **All errors must be handled.** No `let _ = fallible_call();` without a comment explaining why.
- **Error-once rule.** Return the error OR log it, never both.
- `unwrap`/`expect` only in tests and provably infallible cases (e.g., compiled regex).
- Use `anyhow` with `.context()` for actionable CLI errors.

### Variables and scope

- **Declare at the smallest possible scope.** Fewer live variables = fewer bugs.
- **Calculate and check close to use.** Gap between computation and consumption is where bugs hide (POCPOU).

### Data-Oriented Programming

Core principles:

1. **Separate data from behavior.** Data lives in enums, structs. Behavior lives in free functions. No god-objects mixing state + methods.
2. **Data as first-class.** Use typed enums over raw strings where possible — `InputType::String` not `"string"`, `Action::Create { .. }` not an `ActionDef` with `Option` fields.
3. **One data structure, many interpreters.** Adding an interpreter doesn't touch existing code. Adding a variant touches all interpreters (compiler-enforced exhaustive match).

## Performance

- **Think about performance from the outset.** The biggest wins (1000x) come from design, not profiling.
- **Jujo is a local file I/O tool.** Performance is not a concern for typical workloads (a few templates, a handful of files). Don't optimize what isn't slow.

## Developer Experience

### Function shape

- **Hard limit: 70 lines per function.** Good splits divide responsibility, not just line count.
  - Few parameters, simple return type, meaty logic in between.
  - Parent owns control flow. Helpers own computation (ideally pure).
- **God functions and overly fragmented functions are both bad.** Find the balance.
- **Pass-through methods** (method that only invokes another) are a smell.

### Naming

| Element | Convention | Example |
|---------|-----------|---------|
| Types | `PascalCase` noun | `GeneratorDef`, `FieldSpec`, `Action` |
| Functions | `verb_noun` snake_case | `load_generator`, `parse_field`, `render_template` |
| Modules | Singular noun | `generator`, `context`, `render`, `manifest` |
| CLI subcommands | Verb | `generate`, `list`, `describe`, `validate`, `init` |
| Test functions | `test_scenario_expected` | `test_missing_required_input_errors` |

- **Units/qualifiers last**, descending significance: `latency_ms_max` not `max_latency_ms`.
- **Equal-length related names.** `source`/`target` over `src`/`dest`.
- **Infuse names with meaning.** `template_dir`/`output_dir` over two generic `path` variables.

### Comments

- **Always say WHY**, not what. Comment only non-obvious things — but make those count.
- **Comments are sentences.** Capital letter, full stop. Space after `//`. Line-end comments can be phrases.

### Off-by-one awareness

- **Distinguish index (0-based), count (1-based), and size (count * unit).** Mixing them is the #1 off-by-one source.

## Architecture

- **Modules should be deep** — strong functionality behind simple interfaces.
- **Different layer, different abstraction.** CLI parsing, generator loading, template rendering, file operations — each at a distinct level.
- **No circular dependencies.** Acyclic, unidirectional.
- **Free functions everywhere** — no service structs. Pass data explicitly.

## Abstraction and complexity

- **Threshold for abstraction: 4+.** 1-3 times, just repeat (YAGNI). If unsure, repeat more until the abstraction is obvious.
- **Complexity is incremental.** Each shortcut compounds. When cleaning up, fix the small things too.
- **Tactical -> strategic.** Explore with tactical code, then clean up before "done".

## Testing

- **Every bug fix must include a regression test** that would have caught the bug.
- **Tests are correct.** If a test fails, the bug is in the implementation. Never weaken a test to make it pass.
- **Snapshot tests for generated output.** Use `insta` for template rendering results.
- **Tempdir for integration tests.** Every integration test gets a fresh `.jujo/` in a tempdir.

## Git workflow

- Commit after completing a logical unit of work, not after every file edit.
- **Commit messages are read.** Imperative mood, `type: description` (e.g., `feat: add inject action support`).
- Branch naming: `feat/short-description`, `fix/short-description`, `refactor/short-description`.
- Do not push unless explicitly asked.

## Escalation

If any of these rules are unclear or conflict, **ESCALATE** — stop and ask before proceeding.
