# CLI Code Generation Tool Patterns

**Date**: 2026-03-17
**Scope**: Prior art survey, CLI architecture, template engines, generator commands, file modification strategies, testing, DX

## 1. Prior Art — How Other Frameworks Do Codegen

### Rails Generators (Gold Standard)

Rails generators are the benchmark. Key design decisions that make them great:

- **Convention-driven**: `rails generate scaffold Post title:string body:text published:bool` creates model, migration, controller, views, routes, tests — all following naming conventions.
- **Template system**: ERB templates (`.tt` files) in `lib/generators/templates/`. Variables like `<%= class_name %>`, `<%= table_name %>`, `<%= attributes.map(&:name) %>`.
- **Hook system**: Generators can invoke other generators. `scaffold` calls `model`, `controller`, `resource_route` in sequence.
- **Destroy command**: `rails destroy scaffold Post` removes everything that `generate` created. Implemented by tracking created files.
- **Customizable**: Override templates by placing them in `lib/templates/`. Generator classes are subclassable.
- **Injection pattern**: Generators modify existing files (e.g., adding routes to `config/routes.rb`) using `inject_into_file` with regex anchors or markers.

**Key lesson**: The power is in *composition* (generators calling generators) and *convention* (one name produces all derived names). The template system itself is simple.

### Loco (Rust)

Loco v0.16.x implements Rails-style generators via `cargo loco generate`:

```
cargo loco generate scaffold Post title:string! body:text
cargo loco generate model Post title:string! body:text
cargo loco generate controller Posts
cargo loco generate worker Report
cargo loco generate mailer Welcome
cargo loco generate deployment
```

Implementation details:
- **Template engine**: Loco uses `rrgen` (their own crate) wrapping Tera templates.
- **Field type DSL**: `name:type` with modifiers — `!` for non-null, `^` for unique, `references:` for FK. Types: `string`, `string!`, `text`, `int`, `bool`, `float`, `decimal`, `uuid`, `date`, `timestamp`, `json`, `references:Users`.
- **Generated code**: SeaORM entities, migration files, controller stubs, test files.
- **File injection**: Uses marker comments in existing files. `src/app.rs` has `// CODEGEN: routes` where new route registrations are injected.
- **Code location**: Generator logic lives in `loco-gen/` crate within the workspace. Templates are embedded via `include_str!`.

**Key lesson**: Marker comments for file injection is practical in Rust where AST manipulation is heavyweight. Loco's field DSL is worth adapting for our types.

### Django

Django's approach is minimal compared to Rails:
- `django-admin startapp <name>` creates a directory with `models.py`, `views.py`, `admin.py`, `apps.py`, `tests.py`, `migrations/__init__.py`.
- No field specification at CLI level — you edit `models.py` manually, then `python manage.py makemigrations` introspects model changes and auto-generates migration files.
- **Management commands**: Custom CLI commands live in `<app>/management/commands/<name>.py`. Any module can add commands by convention.

**Key lesson**: Django's `makemigrations` (auto-diff schema changes) is powerful but requires ORM introspection we don't have with raw sqlx. The management command convention (discover commands by directory structure) is elegant.

### GoFast

GoFast's CLI is a **project generator**, not a module generator:
- `gofast` CLI walks through provider choices (frontend, DB, payments, email, storage, deployment) interactively.
- Outputs a complete configured project with selected providers wired in.
- No ongoing codegen commands for adding modules/endpoints after initial generation.
- Paid model — requires API key.

**Key lesson**: GoFast chose project-level generation over module-level generation. For a template, we need both: initial project scaffold AND ongoing module generators.

### cargo-generate

`cargo-generate` (v0.22.x) is the Rust ecosystem's project template tool:
- Uses Handlebars templates with `.genignore` files.
- Templates live in Git repos or local directories.
- Supports `cargo-generate.toml` for defining template variables, conditionals, placeholders.
- Best for **new project** scaffolding, not ongoing codegen within an existing project.
- No concept of modifying existing files — only creates new ones from templates.

**Key lesson**: cargo-generate is wrong for our use case. It's a project bootstrapper, not a development-time code generator. We need to create files AND modify existing ones.

## 2. CLI Architecture

### Single Binary vs Cargo Subcommand

| Approach | Pros | Cons |
|----------|------|------|
| `cargo saas <cmd>` (cargo subcommand) | Discovered via `cargo`, no PATH setup | Must be named `cargo-saas`, slower startup (cargo overhead), awkward arg passing |
| `saas-cli <cmd>` (standalone) | Full control, faster startup, simpler testing | Must be installed separately, not in cargo ecosystem |
| Workspace binary (`cargo run -p saas-cli`) | Zero install during dev, same workspace | Requires `cargo run -p` prefix during dev |

**Recommendation**: Ship as **both**. A binary crate named `cargo-saas` in the workspace. During development: `cargo run -p cargo-saas -- generate module tenants`. After install: `cargo saas generate module tenants`. Cargo discovers any binary named `cargo-<name>` as a subcommand automatically.

### Clap — Derive API

Use clap's derive API (v4.5.x). It's less boilerplate than the builder API and generates help text automatically:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo-saas", bin_name = "cargo saas")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from templates
    Generate {
        #[command(subcommand)]
        what: GenerateCommands,
    },
    /// Database operations
    Db {
        #[command(subcommand)]
        what: DbCommands,
    },
    /// Development utilities
    Dev {
        #[command(subcommand)]
        what: DevCommands,
    },
}

#[derive(Subcommand)]
enum GenerateCommands {
    /// Scaffold a full module (all 8 files)
    Module {
        /// Module name (plural, snake_case: e.g., "tenants", "blog_posts")
        name: String,
    },
    /// Add an endpoint to an existing module
    Endpoint {
        /// Target module name
        module: String,
        /// HTTP method (get, post, put, patch, delete)
        method: String,
        /// URL path (e.g., "/:id/activate")
        path: String,
    },
    /// Create a new migration file
    Migration {
        /// Migration name (e.g., "add_status_to_orders")
        name: String,
    },
    /// Generate entity struct + CRUD repository
    Entity {
        /// Entity name (singular: e.g., "tenant", "blog_post")
        name: String,
        /// Fields as name:type pairs (e.g., "title:string", "price:decimal")
        fields: Vec<String>,
    },
    /// Generate a value object newtype
    ValueObject {
        /// Value object name (e.g., "TenantName", "Price")
        name: String,
        /// Inner type (string, i64, decimal, uuid)
        inner_type: String,
    },
    /// Generate error enum for a module
    Error {
        /// Module name
        module: String,
    },
}
```

### Workspace Layout

```
rust-saas-template/
├── Cargo.toml              # workspace root
├── crates/
│   ├── app/                # main application binary
│   │   └── Cargo.toml
│   └── cli/                # code generator binary
│       ├── Cargo.toml      # package name = "cargo-saas"
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands/
│       │   │   ├── mod.rs
│       │   │   ├── generate.rs
│       │   │   ├── db.rs
│       │   │   └── dev.rs
│       │   ├── generators/
│       │   │   ├── mod.rs
│       │   │   ├── module.rs
│       │   │   ├── endpoint.rs
│       │   │   ├── migration.rs
│       │   │   ├── entity.rs
│       │   │   ├── value_object.rs
│       │   │   └── error.rs
│       │   ├── templates/     # .tera template files (embedded)
│       │   │   ├── module/
│       │   │   │   ├── mod.rs.tera
│       │   │   │   ├── routes.rs.tera
│       │   │   │   ├── service.rs.tera
│       │   │   │   ├── domain.rs.tera
│       │   │   │   ├── repository.rs.tera
│       │   │   │   ├── entities.rs.tera
│       │   │   │   ├── dtos.rs.tera
│       │   │   │   ├── value_objects.rs.tera
│       │   │   │   └── error.rs.tera
│       │   │   ├── entity/
│       │   │   ├── value_object/
│       │   │   └── migration/
│       │   ├── naming.rs      # case conversions, pluralization
│       │   └── file_ops.rs    # file creation, injection, dry-run
│       └── tests/
│           ├── generate_module_test.rs
│           └── ...
└── migrations/
```

## 3. Code Generation Patterns

### Template Engine Comparison

| Engine | Version | Approach | Compile-time? | Filters | Inheritance | Binary size |
|--------|---------|----------|---------------|---------|-------------|-------------|
| **Tera** | 1.20.x | Jinja2-like, runtime | No | Rich built-in | Yes | ~500KB |
| **Handlebars** | 6.x | Mustache-like, runtime | No | Custom helpers | Partials | ~400KB |
| **Askama** | 0.12.x | Jinja2-like, compile-time | Yes | Custom | Yes | ~0 (codegen) |
| **minijinja** | 2.7.x | Jinja2-like, runtime, minimal | No | Built-in + custom | Yes | ~250KB |
| **String interpolation** | N/A | `format!`, `include_str!` | Yes | None | None | 0 |

**Recommendation: Tera**. Reasons:
1. **Runtime templates** — critical for development. Developers customizing templates shouldn't need to recompile the CLI.
2. **Jinja2 syntax** — familiar to anyone who's used Django, Ansible, or Hugo.
3. **Rich filters** — built-in `upper`, `lower`, `capitalize`, `snake_case` (via custom filter). No need to pre-compute every case variant.
4. **Loco uses it** — proven in Rust codegen context.
5. **Template inheritance** — base templates for common patterns (e.g., all entity files share a header).

Askama is great for app rendering but wrong for codegen — compile-time checking means template changes require recompilation. String interpolation works for trivial cases but becomes unmaintainable past 3-4 templates.

### Template Storage

**Embed in binary with runtime override**:

```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/templates/"]
struct BuiltinTemplates;

fn load_template(name: &str) -> String {
    // 1. Check project-local override: .saas/templates/<name>
    let override_path = PathBuf::from(".saas/templates").join(name);
    if override_path.exists() {
        return std::fs::read_to_string(override_path).unwrap();
    }
    // 2. Fall back to embedded template
    let file = BuiltinTemplates::get(name)
        .unwrap_or_else(|| panic!("missing template: {name}"));
    std::str::from_utf8(file.data.as_ref()).unwrap().to_string()
}
```

This gives us: zero-config out of the box (embedded), customizable per-project (override directory). `rust-embed` (v8.5.x) handles the embedding. Alternative: plain `include_str!` if we don't need directory traversal.

### Variable Substitution — Template Context

Every generator populates a Tera `Context` with these variables:

| Variable | Example | Source |
|----------|---------|--------|
| `module_name` | `tenants` | CLI arg (snake_case, plural) |
| `module_name_singular` | `tenant` | Derived (depluralize) |
| `ModuleName` | `Tenants` | Derived (PascalCase of module_name) |
| `EntityName` | `Tenant` | Derived (PascalCase of singular) |
| `entity_name` | `tenant` | Derived (snake_case singular) |
| `table_name` | `tenants` | Same as module_name |
| `fields` | `[{name, rust_type, sql_type, nullable}]` | Parsed from CLI args |
| `has_tenant_id` | `true` | Default true (multi-tenant by default) |
| `route_prefix` | `/tenants` | Derived from module_name |

### Naming — The `heck` Crate

`heck` (v0.5.x) handles all case conversions:

```rust
use heck::{ToSnakeCase, ToPascalCase, ToKebabCase};

let input = "blog_posts";
input.to_snake_case();   // "blog_posts"
input.to_pascal_case();  // "BlogPosts"

let singular = "blog_post";
singular.to_pascal_case(); // "BlogPost"
```

For pluralization, use `pluralizer` (v0.4.x) or a minimal hand-rolled lookup. Rails uses `inflector` with an extensive irregular table. For a Rust SaaS template, a small hardcoded irregular map + simple rules (`s` suffix, `ies` for `y`-ending) suffices. Most module names are simple English nouns.

```rust
fn singularize(name: &str) -> String {
    // Common patterns — extend as needed.
    if name.ends_with("ies") {
        format!("{}y", &name[..name.len() - 3])
    } else if name.ends_with("ses") || name.ends_with("xes") || name.ends_with("zes") {
        name[..name.len() - 2].to_string()
    } else if name.ends_with('s') && !name.ends_with("ss") {
        name[..name.len() - 1].to_string()
    } else {
        name.to_string()
    }
}

fn pluralize(name: &str) -> String {
    if name.ends_with('y') && !name.ends_with("ey") && !name.ends_with("oy") {
        format!("{}ies", &name[..name.len() - 1])
    } else if name.ends_with('s') || name.ends_with('x') || name.ends_with('z') {
        format!("{name}es")
    } else {
        format!("{name}s")
    }
}
```

### Field Type Mapping

CLI field specs map to three type systems:

| CLI Type | Rust Type | SQLite Type | sqlx Decode | Notes |
|----------|-----------|-------------|-------------|-------|
| `string` | `String` | `TEXT` | Direct | Default type |
| `text` | `String` | `TEXT` | Direct | Same as string in SQLite |
| `int` / `integer` | `i64` | `INTEGER` | Direct | |
| `bool` / `boolean` | `bool` | `INTEGER` | Direct | SQLite uses 0/1 |
| `float` | `f64` | `REAL` | Direct | |
| `decimal` | `rust_decimal::Decimal` | `TEXT` | Custom sqlx impl | Store as string for precision |
| `uuid` | `String` | `TEXT` | Direct | UUID v7 as text |
| `date` | `chrono::NaiveDate` | `TEXT` | sqlx chrono feature | ISO 8601 |
| `datetime` | `chrono::DateTime<Utc>` | `TEXT` | sqlx chrono feature | ISO 8601 |
| `json` | `serde_json::Value` | `TEXT` | sqlx json feature | `json_extract` for queries |
| `ref:<module>` | `String` | `TEXT` | Direct | FK to `<module>.id` |

Field modifiers (inspired by Loco):
- `!` suffix = NOT NULL (default; omit for nullable)
- `?` suffix = nullable (`Option<T>` in Rust)
- `^` suffix = UNIQUE constraint
- `ref:tenants` = foreign key reference

Example: `cargo saas generate entity order customer_ref:ref:customers total:decimal status:string`

Parsing:

```rust
struct FieldSpec {
    name: String,
    rust_type: String,
    sql_type: String,
    nullable: bool,
    unique: bool,
    is_reference: bool,
    ref_table: Option<String>,
}

fn parse_field(spec: &str) -> Result<FieldSpec, String> {
    let parts: Vec<&str> = spec.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid field spec: {spec}. Expected name:type"));
    }
    let name = parts[0].to_string();
    let mut type_str = parts[1].to_string();

    let nullable = type_str.ends_with('?');
    let unique = type_str.ends_with('^');
    if nullable || unique {
        type_str.pop();
    }

    if type_str.starts_with("ref:") {
        let ref_table = type_str[4..].to_string();
        return Ok(FieldSpec {
            name: format!("{}_id", ref_table.trim_end_matches('s')),
            rust_type: "String".into(),
            sql_type: "TEXT NOT NULL".into(),
            nullable: false,
            unique,
            is_reference: true,
            ref_table: Some(ref_table),
        });
    }

    let (rust_type, sql_type) = match type_str.as_str() {
        "string" | "text" => ("String", "TEXT"),
        "int" | "integer" => ("i64", "INTEGER"),
        "bool" | "boolean" => ("bool", "INTEGER"),
        "float" => ("f64", "REAL"),
        "decimal" => ("Decimal", "TEXT"),
        "uuid" => ("String", "TEXT"),
        "date" => ("NaiveDate", "TEXT"),
        "datetime" => ("DateTime<Utc>", "TEXT"),
        "json" => ("serde_json::Value", "TEXT"),
        other => return Err(format!("unknown type: {other}")),
    };

    let null_suffix = if nullable { "" } else { " NOT NULL" };
    Ok(FieldSpec {
        name,
        rust_type: if nullable { format!("Option<{rust_type}>") } else { rust_type.into() },
        sql_type: format!("{sql_type}{null_suffix}"),
        nullable,
        unique,
        is_reference: false,
        ref_table: None,
    })
}
```

## 4. Generator Commands — What Each Creates/Modifies

### `generate module <name>`

**Purpose**: Full module scaffold following our 8-file convention.

**Creates**:
```
src/<name>/
  mod.rs              — pub mod for each submodule
  routes.rs           — router() fn returning axum Router, stub CRUD handlers
  service.rs          — stub service free functions (create, get, list, update, delete)
  domain.rs           — domain object with value objects (re-exports from entities + VOs)
  repository.rs       — CRUD free functions (insert, find_by_id, list, update, delete)
  entities.rs         — Entity struct (FromRow) + row fields
  dtos.rs             — CreateReq, UpdateReq, ValidCreateReq, Response DTOs
  value_objects.rs    — EntityId newtype (typed_id! macro invocation)
  error.rs            — ModuleError enum with standard variants + From<AppError>
```

**Modifies**:
- `src/main.rs` or `src/app.rs` — adds `mod <name>;` declaration and route nesting: `.nest("/<name>", <name>::routes::router())`
- `migrations/` — creates initial migration `NNNN_create_<name>.sql`

### `generate endpoint <module> <method> <path>`

**Purpose**: Add a single endpoint to an existing module.

**Example**: `cargo saas generate endpoint orders post /:id/cancel`

**Modifies**:
- `src/<module>/routes.rs` — adds handler function + route registration
- `src/<module>/service.rs` — adds service function stub
- `src/<module>/repository.rs` — adds repo function stub (if needed)

### `generate migration <name>`

**Purpose**: Create a timestamped sqlx migration file.

**Creates**:
- `migrations/<timestamp>_<name>.sql` — empty migration with header comment

**Example output**: `migrations/20260317143022_add_status_to_orders.sql`

### `generate entity <name> [fields...]`

**Purpose**: Create entity struct + repository CRUD without full module scaffold.

**Creates/Modifies**:
- `src/<module>/entities.rs` — adds `#[derive(FromRow)] struct <Entity>` with fields
- `src/<module>/repository.rs` — adds CRUD functions (insert, find_by_id, list, update, delete)
- `migrations/` — creates migration with CREATE TABLE

### `generate value-object <name> <inner_type>`

**Purpose**: Create a newtype wrapper with sqlx traits.

**Example**: `cargo saas generate value-object TenantName string`

**Creates/Modifies**:
- `src/<module>/value_objects.rs` — adds newtype with `impl_sqlx_newtype!`, validation, Display, From/TryFrom

### `generate error <module>`

**Purpose**: Create or regenerate error enum for a module.

**Creates**:
- `src/<module>/error.rs` — `<Module>Error` enum with standard variants (NotFound, ValidationFailed, AccessDenied, InvalidTransition, Infra(AppError)) + From impls

## 5. File Modification Patterns

### The Three Approaches

#### Approach A: Marker Comments

```rust
// In src/main.rs:
// SAAS_CODEGEN:MODULE_DECLARATIONS
mod tenants;
mod orders;
// SAAS_CODEGEN:END

// SAAS_CODEGEN:ROUTE_REGISTRATION
.nest("/tenants", tenants::routes::router())
.nest("/orders", orders::routes::router())
// SAAS_CODEGEN:END
```

Generator finds the marker, inserts new lines before `// SAAS_CODEGEN:END`.

**Pros**: Simple string search, no parsing, works 100% of the time, language-agnostic.
**Cons**: Markers are visible in source code, developers might accidentally delete them.

#### Approach B: AST Manipulation (syn + quote)

Parse the Rust file with `syn`, find the relevant AST node, insert the new node, re-emit with `prettyplease`.

```rust
use syn::{parse_file, Item, ItemMod};
use quote::quote;
use prettyplease::unparse;

let source = std::fs::read_to_string("src/main.rs")?;
let mut ast = parse_file(&source)?;

// Add: mod tenants;
ast.items.push(Item::Mod(syn::parse_quote! {
    mod tenants;
}));

let output = unparse(&ast);
std::fs::write("src/main.rs", output)?;
```

**Pros**: Semantically correct, survives reformatting, can handle complex modifications.
**Cons**: Heavy dependencies (`syn` + `quote` + `prettyplease` add ~15s compile time), may reformat the entire file (losing intentional formatting), doesn't preserve comments well, brittle across Rust edition changes.

#### Approach C: Regex/String Pattern Matching

Find patterns like `fn router()` or the last `mod ` declaration and insert after them.

**Pros**: No extra dependencies, no markers in source.
**Cons**: Fragile — breaks if code style changes, hard to handle all edge cases.

### Recommendation: Marker Comments (Approach A)

For our use case, marker comments win decisively:

1. **Our template controls the initial code.** We emit the markers in the scaffold. Developers don't write them — the generator creates them.
2. **Reliability over elegance.** A marker that works 100% of the time beats an AST parser that works 95% of the time.
3. **Loco uses this pattern.** Battle-tested in a similar Rust framework.
4. **Minimal dependencies.** No `syn`/`quote` in the CLI crate — faster compile times.
5. **Transparent.** Developers can see where codegen will insert. No magic.

Make markers unobtrusive:

```rust
// <saas:modules>
mod tenants;
mod orders;
// </saas:modules>
```

Implementation:

```rust
fn inject_before_marker(file_path: &Path, marker: &str, content: &str) -> Result<()> {
    let source = std::fs::read_to_string(file_path)?;
    let end_marker = marker.replace("<saas:", "</saas:");

    let Some(pos) = source.find(&end_marker) else {
        return Err(anyhow!("marker {end_marker} not found in {}", file_path.display()));
    };

    let mut output = String::with_capacity(source.len() + content.len());
    output.push_str(&source[..pos]);
    output.push_str(content);
    output.push('\n');
    output.push_str(&source[pos..]);

    std::fs::write(file_path, output)?;
    Ok(())
}
```

### Fallback for No Markers

If a file has been edited and markers removed, fall back to append-at-end with a warning:

```
WARNING: marker </saas:modules> not found in src/main.rs.
  Appended `mod tenants;` at end of file. You may need to move it.
```

## 6. Testing the CLI

### Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `assert_cmd` | 2.0.x | Run CLI binary, assert exit code + stdout/stderr |
| `predicates` | 3.1.x | Fluent assertions (contains, matches, starts_with) |
| `tempfile` | 3.14.x | Temp directories for generated output |
| `insta` | 1.41.x | Snapshot testing — compare generated files against saved snapshots |
| `similar` | 2.6.x | Diff engine (used by insta internally) |

### Test Strategy

**Layer 1: Unit tests for naming/parsing**
```rust
#[test]
fn singularize_regular() {
    assert_eq!(singularize("tenants"), "tenant");
    assert_eq!(singularize("categories"), "category");
    assert_eq!(singularize("addresses"), "address");
}

#[test]
fn parse_field_spec_with_reference() {
    let field = parse_field("ref:tenants").unwrap();
    assert_eq!(field.name, "tenant_id");
    assert!(field.is_reference);
    assert_eq!(field.ref_table.unwrap(), "tenants");
}
```

**Layer 2: Snapshot tests for generated files**

Use `insta` to snapshot the generated output of each template:

```rust
#[test]
fn generate_module_routes() {
    let ctx = make_test_context("tenants", &[]);
    let output = render_template("module/routes.rs.tera", &ctx).unwrap();
    insta::assert_snapshot!(output);
}
```

Snapshots live in `tests/snapshots/`. Review with `cargo insta review`. This catches unintended template changes immediately.

**Layer 3: Integration tests with temp directories**

```rust
use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn generate_module_creates_all_files() {
    let dir = TempDir::new().unwrap();
    // Seed with minimal project structure (src/main.rs with markers)
    setup_minimal_project(dir.path());

    Command::cargo_bin("cargo-saas")
        .unwrap()
        .current_dir(dir.path())
        .args(["saas", "generate", "module", "tenants"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Created src/tenants/mod.rs"));

    // Verify all 8 files exist
    assert!(dir.path().join("src/tenants/mod.rs").exists());
    assert!(dir.path().join("src/tenants/routes.rs").exists());
    assert!(dir.path().join("src/tenants/service.rs").exists());
    assert!(dir.path().join("src/tenants/domain.rs").exists());
    assert!(dir.path().join("src/tenants/repository.rs").exists());
    assert!(dir.path().join("src/tenants/entities.rs").exists());
    assert!(dir.path().join("src/tenants/dtos.rs").exists());
    assert!(dir.path().join("src/tenants/value_objects.rs").exists());
    assert!(dir.path().join("src/tenants/error.rs").exists());

    // Verify main.rs was modified
    let main = std::fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
    assert!(main.contains("mod tenants;"));
}
```

**Layer 4: Compilation test**

The ultimate test — does the generated code compile?

```rust
#[test]
#[ignore] // slow — run in CI only
fn generated_module_compiles() {
    let dir = TempDir::new().unwrap();
    setup_full_project(dir.path()); // includes Cargo.toml, dependencies
    generate_module(dir.path(), "tenants", &[]);

    Command::new("cargo")
        .current_dir(dir.path())
        .args(["check"])
        .assert()
        .success();
}
```

## 7. Developer Experience

### Colorized Output

Use `owo-colors` (v4.1.x) — zero-alloc, supports NO_COLOR env var:

```rust
use owo_colors::OwoColorize;

println!("  {} {}", "create".green().bold(), "src/tenants/mod.rs");
println!("  {} {}", "create".green().bold(), "src/tenants/routes.rs");
println!("  {} {}", "inject".yellow().bold(), "src/main.rs");
println!("  {} {}", "create".green().bold(), "migrations/20260317_create_tenants.sql");
```

Output:
```
  create src/tenants/mod.rs
  create src/tenants/routes.rs
  inject src/main.rs
  create migrations/20260317_create_tenants.sql
```

### Dry-Run Mode

Global `--dry-run` flag shows what would happen without writing:

```
$ cargo saas generate module tenants --dry-run
  [dry-run] would create src/tenants/mod.rs
  [dry-run] would create src/tenants/routes.rs
  [dry-run] would create src/tenants/service.rs
  [dry-run] would create src/tenants/domain.rs
  [dry-run] would create src/tenants/repository.rs
  [dry-run] would create src/tenants/entities.rs
  [dry-run] would create src/tenants/dtos.rs
  [dry-run] would create src/tenants/value_objects.rs
  [dry-run] would create src/tenants/error.rs
  [dry-run] would inject into src/main.rs
  [dry-run] would create migrations/20260317143022_create_tenants.sql
```

Implementation: thread a `DryRun(bool)` through all file operations. When true, print but don't write.

### Interactive Mode — Skip It

Rails has `--no-skip` and `--force` flags but no interactive prompts during generation. Keep it non-interactive:
- All parameters come from CLI args.
- Sane defaults for everything.
- `--force` to overwrite existing files.
- No prompts, no `dialoguer` — keeps it scriptable and CI-friendly.

Exception: a future `cargo saas init` could use interactive prompts for initial project setup (like GoFast). That's a different command.

### Undo/Rollback — Don't Build It

Rails has `rails destroy` which reverses a generator. Pragmatic assessment:

1. File creation is trivially reversible with `git checkout` or `rm`.
2. File injection (modifying existing files) is reversible with `git checkout`.
3. Building a proper undo system adds significant complexity.
4. Git is the undo system.

**Recommendation**: Don't build undo. Instead, print a summary of all created/modified files so the user can `git checkout` selectively. If we want to be extra helpful, print the git command:

```
To undo: git checkout -- src/tenants/ src/main.rs && rm -rf src/tenants/
```

### Error Messages

Clear, actionable errors:

```
Error: module "tenants" already exists at src/tenants/
  Use --force to overwrite, or choose a different name.

Error: unknown field type "varchar" in "name:varchar"
  Valid types: string, text, int, bool, float, decimal, uuid, date, datetime, json, ref:<table>

Error: marker </saas:modules> not found in src/main.rs
  Expected marker comment for code injection. Was it removed?
  Add this line where module declarations should go:
    // </saas:modules>
```

## 8. Recommended Architecture

### Dependency Manifest (cli crate)

```toml
[package]
name = "cargo-saas"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "cargo-saas"
path = "src/main.rs"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
tera = "1.20"
heck = "0.5"
owo-colors = "4.1"
anyhow = "1.0"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
rust-embed = "8.5"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.14"
insta = "1.41"
```

Estimated compile cost: light. No `syn`, no `quote`, no `prettyplease`, no `serde` (unless we want config files). Tera is the heaviest dep at ~2s compile.

### Template Example — `module/routes.rs.tera`

```
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::errors::AppError;
use crate::state::AppState;
use super::dtos::{Create{{ EntityName }}Req, Update{{ EntityName }}Req, {{ EntityName }}Response, List{{ EntityName }}Params};
use super::service;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_{{ entity_name }}s))
        .route("/", post(create_{{ entity_name }}))
        .route("/:id", get(get_{{ entity_name }}))
        .route("/:id", put(update_{{ entity_name }}))
        .route("/:id", delete(delete_{{ entity_name }}))
    // <saas:routes>
    // </saas:routes>
}

async fn list_{{ entity_name }}s(
    State(state): State<AppState>,
    Query(params): Query<List{{ EntityName }}Params>,
) -> Result<Json<Vec<{{ EntityName }}Response>>, AppError> {
    let items = service::list_{{ entity_name }}s(&state.db, params).await?;
    Ok(Json(items))
}

async fn create_{{ entity_name }}(
    State(state): State<AppState>,
    Json(req): Json<Create{{ EntityName }}Req>,
) -> Result<(StatusCode, Json<{{ EntityName }}Response>), AppError> {
    let item = service::create_{{ entity_name }}(&state.db, req).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn get_{{ entity_name }}(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<{{ EntityName }}Response>, AppError> {
    let item = service::get_{{ entity_name }}(&state.db, &id).await?;
    Ok(Json(item))
}

async fn update_{{ entity_name }}(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<Update{{ EntityName }}Req>,
) -> Result<Json<{{ EntityName }}Response>, AppError> {
    let item = service::update_{{ entity_name }}(&state.db, &id, req).await?;
    Ok(Json(item))
}

async fn delete_{{ entity_name }}(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    service::delete_{{ entity_name }}(&state.db, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

### Execution Flow

```
User runs: cargo saas generate module orders

1. CLI parses args → GenerateCommands::Module { name: "orders" }
2. Validate: "orders" is valid snake_case, src/orders/ doesn't exist
3. Derive names: module=orders, singular=order, Entity=Order, Table=orders
4. Load templates (embedded or override)
5. Build Tera Context with all naming variables
6. For each template in module/:
   a. Render template with context
   b. Write to src/orders/<file>.rs (or print in dry-run)
7. Inject into src/main.rs:
   a. Find </saas:modules>, insert `mod orders;` before it
   b. Find </saas:routes>, insert `.nest("/orders", orders::routes::router())` before it
8. Generate migration file
9. Print summary of created/modified files
```

## 9. Key Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Binary name | `cargo-saas` (cargo subcommand) | Ecosystem convention, discoverable |
| CLI parser | clap derive API | Less boilerplate, auto-help |
| Template engine | Tera | Runtime templates, Jinja2 syntax, proven in Loco |
| Template storage | rust-embed + local override | Zero-config + customizable |
| Case conversion | heck | Standard crate, all conversions |
| File injection | Marker comments (`<saas:...>`) | Reliable, simple, transparent |
| Output coloring | owo-colors | Zero-alloc, NO_COLOR support |
| Testing | insta (snapshots) + assert_cmd + tempfile | Three-layer coverage |
| Interactive mode | None (flags only) | Scriptable, CI-friendly |
| Undo | None (use git) | Complexity not justified |
| Workspace location | `crates/cli/` | Separate from app, shared workspace |

## 10. Implementation Priority

**Phase 1** (minimum viable): `generate module` + `generate migration`. These two commands cover 80% of the scaffolding work. Module generation is the most complex (8 files + injection) and proves the entire pipeline.

**Phase 2**: `generate entity` + `generate value-object`. Incremental generators for adding to existing modules.

**Phase 3**: `generate endpoint` + `generate error`. Finer-grained generators.

**Phase 4**: `db` commands (migrate, reset, seed), `dev` commands (doctor, routes list).
