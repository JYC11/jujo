---
name: test-guide
description: >
  Guide for writing tests following the project's layered test standards. Use when the user says
  "write tests", "add tests", "test strategy", "what tests to write", "test this module",
  "where should this test go", or is deciding which test layer to use.
---

# Jujo Test Standards

Each test justifies its infrastructure cost. No duplicate assertions across layers.

## Decision Flowchart

```
Input validation (field parsing, --var parsing)?    → Unit test
Naming/casing (singularize, pluralize, heck)?       → Unit test
TOML deserialization (generator.toml)?              → Unit test
Tera filter correctness?                            → Unit test
Template rendering output?                          → Snapshot test (insta)
File creation, injection, directory ops?            → Integration test (tempdir)
CLI command workflow (generate, list, describe)?    → CLI integration test (assert_cmd + tempdir)
Manifest JSON structure?                            → Snapshot test or integration test
Error messages (missing input, bad TOML)?           → CLI integration test (assert stderr)
Walk-up .jujo/ discovery?                           → Integration test (nested tempdir)
Happy-path end-to-end?                              → CLI integration test (covers all layers)
```

## Layer Guide

### 1. Unit Tests — Pure Logic, No I/O

**Infra:** None (`#[test]`, sync, no filesystem)
**Test:**
- Field parsing: `parse_field("title:string")` → correct FieldSpec
- Field modifiers: `?` for nullable
- Type map lookup: known types resolve, unknown types error
- `--var` parsing: `key=value` split on first `=`, comma splitting for arrays
- Naming: singularize, pluralize, case conversions via filters
- TOML deserialization: `GeneratorDef` from valid/invalid TOML strings
- Input validation: required inputs, defaults, unknown inputs

**Skip:** anything requiring the filesystem or a real `.jujo/` directory

### 2. Snapshot Tests — Template Rendering (insta)

**Infra:** `insta` crate, no filesystem
**Test:**
- Rendered template output matches expected (golden file)
- `_vars.tera` include resolution
- Tera loops over field arrays
- Tera conditionals (nullable fields)
- AI customization marker presence in output

**Pattern:**
```rust
#[test]
fn render_routes_template() {
    let ctx = make_test_context("tenants", &[]);
    let output = render_template("module/routes.rs.tera", &ctx).unwrap();
    insta::assert_snapshot!(output);
}
```

Snapshots live in `tests/snapshots/`. Review with `cargo insta review`.

**Skip:** file writing, manifest generation, CLI output format

### 3. Integration Tests — File Operations (tempdir)

**Infra:** `tempfile::TempDir`, real filesystem
**Test:**
- `create_file` writes to correct path, creates parent dirs
- `create_file` errors on existing file (no --force), overwrites with --force
- `inject_before_marker` finds marker and inserts content
- `inject_before_marker` errors when marker not found
- Conflict detection: content already present near marker
- `--force` and `--skip-existing` behavior
- Manifest JSON written with correct structure
- Walk-up discovery: `.jujo/` found in parent directory

**Skip:** CLI argument parsing, output formatting

### 4. CLI Integration Tests — Command Contract (assert_cmd)

**Infra:** `assert_cmd` + `tempfile::TempDir` with seeded `.jujo/` directory
**Test:**
- `jujo generate <name> --var key=value` end-to-end
- `jujo list` / `jujo list --json` output
- `jujo describe <gen>` / `jujo describe <gen> --json` output
- `jujo validate` catches errors and reports all of them
- `jujo init --lang rust` creates correct `.jujo/` structure
- `--dry-run` shows actions without writing files
- `--json` output is valid JSON
- Error cases: unknown generator, missing required input, bad TOML
- Exit codes: 0 for success, non-zero for errors

**Pattern:**
```rust
use assert_cmd::Command;
use tempfile::TempDir;

fn jujo_cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

fn seed_generator(dir: &TempDir, name: &str, toml: &str, templates: &[(&str, &str)]) {
    let gen_dir = dir.path().join(".jujo/templates").join(name);
    std::fs::create_dir_all(&gen_dir).unwrap();
    std::fs::write(gen_dir.join("generator.toml"), toml).unwrap();
    for (filename, content) in templates {
        std::fs::write(gen_dir.join(filename), content).unwrap();
    }
}

#[test]
fn generate_creates_files() {
    let dir = TempDir::new().unwrap();
    seed_generator(&dir, "example", EXAMPLE_TOML, &[("hello.tera", "Hello {{ name }}!")]);
    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "name=world"])
        .assert()
        .success()
        .stdout(predicates::str::contains("create"));
    assert!(dir.path().join("expected/output/path").exists());
}
```

**Skip:** template rendering details (snapshot tests), SQL/DB (none in jujo), internal error messages

## Test Infrastructure

```rust
// tests/common/mod.rs
use tempfile::TempDir;
use assert_cmd::Command;

pub fn jujo_cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

pub fn init_jujo_dir(dir: &TempDir) {
    let jujo_dir = dir.path().join(".jujo/templates");
    std::fs::create_dir_all(&jujo_dir).unwrap();
    // Write minimal config.toml
    std::fs::write(
        dir.path().join(".jujo/config.toml"),
        "[type_map]\nstring = \"String\"\nint = \"i64\"\nbool = \"bool\"\n\ncomment_prefix = \"//\"\ncomment_suffix = \"\"\n"
    ).unwrap();
}

pub fn seed_generator(dir: &TempDir, name: &str, toml_content: &str, templates: &[(&str, &str)]) {
    let gen_dir = dir.path().join(".jujo/templates").join(name);
    std::fs::create_dir_all(&gen_dir).unwrap();
    std::fs::write(gen_dir.join("generator.toml"), toml_content).unwrap();
    for (filename, content) in templates {
        std::fs::write(gen_dir.join(filename), content).unwrap();
    }
}
```

## Order for New Features

1. Unit tests for parsing/validation logic (fast feedback, no I/O)
2. Snapshot tests for any new template rendering
3. Integration tests for file operations
4. CLI integration tests for command-level behavior
5. **Verify no test duplicates an assertion at another layer**

## Anti-Patterns

- **Don't test template rendering in CLI tests** — snapshot tests own rendering correctness
- **Don't test TOML parsing in integration tests** — unit tests own deserialization
- **Don't test file I/O in unit tests** — integration tests own filesystem behavior
- **Don't share tempdir state between tests** — fresh TempDir per test
- **Don't test internal error messages in CLI tests** — test exit codes and `--json` structure
