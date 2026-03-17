---
name: manual-qa
description: >
  Run manual QA against the jujo CLI binary using a real temp project. Build the release binary,
  create an isolated test directory, execute commands, capture stdout/stderr/exit codes,
  and produce a structured results file in the project's .qa/ folder.
  Use when: the user asks to "manual test", "manual QA", "QA the CLI", "smoke test",
  "exercise the binary", "end-to-end QA", or wants to verify CLI behavior outside of
  automated tests.
  Triggers on: "manual qa", "manual test", "smoke test", "QA", "exercise the CLI",
  "test the binary", "end-to-end test the CLI".
---

# Manual QA

Manual QA exercises the real release binary end-to-end against a temp project directory.
It catches issues automated tests miss: output formatting, error message quality, exit
codes, cross-command workflows, and CLI argument ergonomics.

## When to Run

- After completing a development phase
- After fixing bugs found in prior QA
- Before publishing to crates.io
- When adding new commands or changing CLI interface

## Workflow

### 1. Setup

```bash
cargo build --release
cd /tmp && rm -rf jujo-qa && mkdir jujo-qa && cd jujo-qa
jujo init --lang rust
```

### 2. Plan Test Cases

Organize by command group with TC-XX IDs. Typical groups:

- Init (idempotency, double-init, language selection)
- Generate (happy path, missing inputs, unknown generator, --force, --skip-existing, --dry-run, --json)
- List/Describe (human output, --json output, empty state)
- Validate (valid templates, syntax errors, missing files)
- Inject (marker found, marker missing, conflict detection)
- Template management (add, remove, list)
- Error handling (invalid TOML, bad field specs, unknown types)
- Cross-command workflows (init → generate → list → validate)

### 3. Execute and Capture

Write results to `.qa/manual-qa-YYYY-MM-DDTHHMM.md` in the project root.

For each test case, capture:
- The exact command run
- Full stdout and stderr
- Exit code
- Pass/fail verdict with reasoning

```bash
R="$PROJECT_ROOT/.qa/manual-qa-$(date +%Y-%m-%dT%H%M).md"
mkdir -p .qa

echo '### TC-XX: Description' >> "$R"
echo '```' >> "$R"
jujo generate example --var module_name=orders >> "$R" 2>&1; echo "exit: $?" >> "$R"
echo '```' >> "$R"
```

### 4. Analyze Results

After all tests run:
- Mark each TC as PASS or FAIL with reasoning
- For failures, document expected vs actual
- Create a "Bugs Found" section with severity
- Write a summary table by group

### 5. Fix and Verify

For each bug:
1. Fix the code
2. Run `cargo test` to ensure no regressions
3. Rebuild release binary
4. Re-run the failing TC
5. Update results file

## Testing Mindset: Aggressive, Malicious, Stupid

QA is not about confirming the software works — it's about proving it doesn't.

**Aggressive** — Push every boundary. Feed maximum-length strings for --var values, chain
operations in unexpected orders, generate into deeply nested paths. If a field accepts a
name, paste in 2000 characters. Run generate twice with same inputs. Run generate with
50 --var flags.

**Malicious** — Think like an attacker. Inject shell metacharacters in --var values
(`$(rm -rf /)`). Pass `../../../etc/passwd` as output paths in generator.toml. Use null
bytes in template content. Try to write outside the project directory. Put Tera injection
in --var values (`{{ config }}`). Use marker names that look like code.

**Stupid** — Be the user who read nothing. Omit required --var flags. Spell commands wrong.
Use --json on commands that don't support it. Pass --help mid-argument list. Run generate
before init. Use spaces in generator names. Forget the --var prefix.

### What This Looks Like in Practice

| Happy path TC | Aggressive TC | Malicious TC | Stupid TC |
|---------------|---------------|--------------|-----------|
| Generate with valid vars | Generate with 50 --var flags | --var with `{{ system }}` injection | Generate with no --var at all |
| List generators | List after creating 20 generators | Template name `../../etc/passwd` | List before init |
| Init with --lang rust | Init twice in same dir | Init with --lang `; rm -rf /` | Init with no --lang and no tty |
| Validate valid templates | Validate 20 generators at once | Template with `{% include "/etc/passwd" %}` | Validate before init |

### Bug Severity

- **Panic/crash** → Critical (the binary must never crash on bad input)
- **Path traversal** → Critical (writing files outside project directory)
- **Data corruption** → Critical (silent data loss or mangled files)
- **Template injection** → High (--var values executing as Tera code)
- **Leaked internal errors** (raw panic, stack traces) → High
- **Wrong exit code** → Medium (breaks scripting and CI pipelines)
- **Missing/bad error message** → Medium (user can't self-diagnose)

## Key Things to Verify

- **Output formatting**: alignment, colored labels, consistent style
- **Error messages**: human-readable with suggestions, correct exit codes
- **JSON mode**: valid JSON on all commands that support it, correct field names
- **Cross-command consistency**: generator created via init visible in list, describable, validatable
- **Idempotency**: double-init behavior, double-generate behavior
- **Edge cases**: empty template directory, no generators, no config.toml
- **Path safety**: generated files stay within project directory
- **Template safety**: --var values don't execute as Tera expressions
- **Marker injection**: content placed correctly, conflict detection works
- **Dry-run accuracy**: --dry-run output matches what --json would produce

## Results File Structure

Results use datetime in the filename to maintain a log across runs:

```
.qa/
  manual-qa-YYYY-MM-DDTHHMM.md   # e.g. manual-qa-2026-03-18T1430.md
```

Each run produces a new timestamped file. Never overwrite previous results.
