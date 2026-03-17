# Plan: Jujo — Agent-First Codegen Framework

> Research: [generalizable-codegen-tool.md](research/generalizable-codegen-tool.md), [agent-first-codegen.md](research/agent-first-codegen.md), [cli-codegen-patterns.md](research/cli-codegen-patterns.md)

## Architectural Decisions

- **AD-1: Standalone `jujo` binary.** Not a cargo subcommand. Framework-agnostic.
- **AD-2: Zero built-in templates.** `jujo init` creates `.jujo/` with a trivial example generator. Users define their own.
- **AD-3: Full Tera for templates.** Slots, filters, loops, conditionals. Tera complexity abstracted via an agent skill — agents write templates, humans learn incrementally.
- **AD-4: No `[derived]` section in generator.toml.** Templates use `_vars.tera` includes for shared variable derivation (`{% set EntityName = module_name | singularize | pascal_case %}`). One syntax everywhere.
- **AD-5: Single closing marker for injection.** `// </jujo:marker>` in source files. Comment style configured via `comment_prefix` + `comment_suffix` in `config.toml` (supports line comments `//`/`#` and block comments `<!-- -->`).
- **AD-5b: Walk-up directory discovery.** Jujo walks up from cwd to find `.jujo/`. Like git/cargo/npm.
- **AD-6: No state file for idempotency.** Conflict detection via string match near marker. Default: error. `--force` to inject anyway, `--skip-existing` to silently skip.
- **AD-7: Abstract type system with user-defined type map.** `config.toml` `[type_map]` maps abstract types (string, int, bool, etc.) to language-specific types. Unknown types → error.
- **AD-8: Interactive `jujo init`.** Form-based (language selection → type map + comment style). Also supports `--lang` CLI arg for agents/scripting.
- **AD-9: JSON-first output.** Every command supports `--json`. Human-readable colored output is the default.
- **AD-10: No async, no network, no MCP.** Synchronous file I/O only. Agents invoke via shell.
- **AD-11: anyhow for errors.** CLI tool, not a library.
- **AD-12: Typed Action enum.** `Action::Create { template, output }` and `Action::Inject { target, marker, content }`. Uses `#[serde(tag = "type")]` for TOML deserialization. Invalid states unrepresentable.
- **AD-13: `pluralizer` crate for singularize/pluralize.** Comprehensive English pluralization instead of hand-rolled rules.
- **AD-14: `--var` accepts both comma-separated and repeated forms.** `--var fields="a,b,c"` and `--var fields=a --var fields=b` both work for array inputs.

## Key Types & Interfaces

```rust
// --- generator.toml schema ---

struct GeneratorDef {
    generator: GeneratorMeta,
    inputs: Vec<InputDef>,
    actions: Vec<Action>,
}

struct GeneratorMeta {
    name: String,
    description: String,
}

struct InputDef {
    name: String,
    r#type: InputType,
    description: String,
    required: bool,
    default: Option<toml::Value>,
}

enum InputType {
    String,
    StringArray,
    Bool,
    Int,
    FieldArray,   // parses "name:type?" syntax
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Action {
    Create {
        template: String,         // template filename
        output: String,           // output path (Tera expression)
    },
    Inject {
        target: String,           // file to inject into
        marker: String,           // marker name (becomes </jujo:marker>)
        content: String,          // content to inject (Tera expression)
    },
}

// --- Field parsing ---

struct FieldSpec {
    name: String,
    r#type: String,           // abstract type (e.g. "string")
    mapped_type: String,      // language type (e.g. "String")
    nullable: bool,
}

// Abstract types: string, text, int, bool, float, decimal, uuid, date, datetime, json

// --- config.toml ---

struct ProjectConfig {
    type_map: BTreeMap<String, String>,  // abstract type → language type
    comment_prefix: String,              // "//" or "#" or "<!--"
    comment_suffix: String,              // "" or "-->"
}

// --- Generation result ---

struct GenerationResult {
    generator: String,
    timestamp: String,
    inputs: BTreeMap<String, serde_json::Value>,
    created: Vec<CreatedFile>,
    injected: Vec<InjectedContent>,
    customize: Vec<CustomizeMarker>,
}

struct CreatedFile {
    path: String,
    template: String,
}

struct InjectedContent {
    path: String,
    marker: String,
    content: String,
}

struct CustomizeMarker {
    path: String,
    line: usize,
    hint: String,
}

// --- CLI ---

#[derive(Parser)]
#[command(name = "jujo")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

enum Commands {
    Init { lang: Option<String> },      // interactive or --lang
    List { json: bool },
    Describe { generator: String, json: bool },
    Generate {
        generator: String,
        vars: Vec<String>,              // --var key=value
        json: bool,
        dry_run: bool,
        force: bool,
        skip_existing: bool,
    },
    Validate,
}
```

### Tera Custom Filters

```
singularize  — "tenants" → "tenant"
pluralize    — "tenant" → "tenants"
pascal_case  — "blog_posts" → "BlogPosts"
snake_case   — "BlogPosts" → "blog_posts"
kebab_case   — "blog_posts" → "blog-posts"
camel_case   — "blog_posts" → "blogPosts"
upper_case   — "tenant" → "TENANT"
```

Provided by `heck` crate, registered as Tera filters at startup.

### Template Variable Sharing

Templates use `{% include "_vars.tera" %}` for shared derivations:

```
{# _vars.tera — shared across all templates in a generator #}
{% set entity_name = module_name | singularize %}
{% set EntityName = entity_name | pascal_case %}
{% set ModuleName = module_name | pascal_case %}
```

Each `.tera` template includes this at the top. No `[derived]` section in TOML.

---

## Phase 1: Tracer Bullet — TOML Parse → Tera Render → File Create → JSON Manifest

**Goal**: Prove the end-to-end pipeline with string inputs and create actions only.

### What to build

A `jujo generate <name> --var key=value` command that:
1. Discovers `.jujo/templates/<name>/generator.toml`
2. Parses the TOML into `GeneratorDef`
3. Validates required inputs, applies defaults
4. Renders Tera templates (with `_vars.tera` include support)
5. Writes output files (error if exists, `--force` to overwrite)
6. Writes `.jujo/last-generate.json` manifest
7. Prints human-readable summary to stdout

**Not in this phase**: inject actions, `--json`, `--dry-run`, `--skip-existing`, `field[]` input type, type map.

### Implementation Detail

- `src/main.rs`: clap CLI with `Generate` subcommand. Other subcommands stub with "not yet implemented".
- `src/generator.rs`: `GeneratorDef` types + TOML deserialization + `load_generator(name: &str) -> Result<GeneratorDef>`
- `src/context.rs`: `build_context(def: &GeneratorDef, vars: &BTreeMap<String, String>) -> Result<tera::Context>` — validates required inputs, applies defaults
- `src/render.rs`: `render_template(tera: &Tera, template_name: &str, ctx: &tera::Context) -> Result<String>` — loads `.tera` files from generator directory, renders
- `src/filters.rs`: Tera custom filter registration (heck case conversions, singularize/pluralize)
- `src/file_ops.rs`: `create_file(path: &Path, content: &str, force: bool) -> Result<CreatedFile>` — writes file, creates parent dirs
- `src/manifest.rs`: `write_manifest(result: &GenerationResult) -> Result<()>` — writes `.jujo/last-generate.json`
- `src/discovery.rs`: `find_jujo_root() -> Result<PathBuf>` — walk up from cwd to find `.jujo/`
- `Cargo.toml`: clap, tera, serde + serde_json, toml, heck, pluralizer, anyhow, chrono (timestamp)

### Tests

- [ ] Parse a minimal `generator.toml` with one input and one create action
- [ ] Malformed TOML → clear error with file path
- [ ] Valid TOML but wrong schema (e.g., missing `[generator]` section) → clear error
- [ ] Missing required input → clear error
- [ ] Default value applied when input not provided
- [ ] Unknown generator name → clear error
- [ ] `--var` parsing: `key=value` with `=` in value (split on first `=` only)
- [ ] Walk-up discovery: finds `.jujo/` in parent directory
- [ ] Walk-up discovery: no `.jujo/` found → error suggesting `jujo init`
- [ ] Tera filters: singularize, pluralize, pascal_case, snake_case, kebab_case, camel_case
- [ ] `_vars.tera` include resolves correctly across templates
- [ ] Template rendering with full context produces expected output (snapshot test)
- [ ] `create_file` writes to correct path, creates parent directories
- [ ] `create_file` errors if file exists and `force=false`
- [ ] `create_file` overwrites if `force=true`
- [ ] Manifest JSON written with correct structure
- [ ] Template syntax error → error with file name and line
- [ ] Integration: end-to-end `jujo generate` with a test generator in a tempdir

### Acceptance Criteria

- [ ] `jujo generate example --var module_name=tenants` creates rendered files from `.jujo/templates/example/`
- [ ] `.jujo/last-generate.json` written with `created` array
- [ ] `--force` overwrites existing files

---

## Phase 2: Field Parsing + Type Map

**Goal**: Add `field[]` input type with abstract-to-language type mapping.

### What to build

1. **`field[]` input type**: parse `--var fields="title:string,price:decimal?"` into structured `FieldSpec` objects
2. **`config.toml` with `[type_map]`**: maps abstract types to language types
3. **Type validation**: unknown abstract types → error with list of valid types

### Implementation Detail

- `src/config.rs`: `ProjectConfig` — parse `.jujo/config.toml`, load type map
- `src/fields.rs`: `parse_field(spec: &str, type_map: &BTreeMap<String, String>) -> Result<FieldSpec>` — parses `name:type?` syntax, resolves mapped_type
- `src/context.rs`: extend `build_context` to handle `FieldArray` inputs, inject `Vec<FieldSpec>` into Tera context

### Tests

- [ ] Parse `title:string` → FieldSpec { name: "title", type: "string", mapped_type: "String", nullable: false }
- [ ] Parse `price:decimal?` → FieldSpec { nullable: true, mapped_type: "rust_decimal::Decimal" }
- [ ] Unknown type `money` → error listing valid types
- [ ] `?` modifier correctly sets nullable
- [ ] Malformed field spec (no colon, e.g., `title`) → clear error
- [ ] `--var` comma split for field arrays: `--var fields="a:string,b:int"`
- [ ] `--var` repeated for field arrays: `--var fields="a:string" --var fields="b:int"`
- [ ] Fields available in Tera context as iterable array
- [ ] Template loop over fields produces correct output (snapshot test)
- [ ] Missing config.toml → clear error
- [ ] Empty type_map → error on any field usage

### Acceptance Criteria

- [ ] Generator with `field[]` input renders struct with correct types from type map
- [ ] Same generator works with different `config.toml` type maps (Rust vs Go)

---

## Phase 3: Injection + Dry-Run + JSON Output

**Goal**: Complete the core generation engine.

### What to build

1. **Inject actions**: find `// </jujo:marker>` (using configured comment style), insert content before it
2. **Conflict detection**: if content already present near marker → error. `--force` to inject anyway, `--skip-existing` to skip.
3. **`--dry-run`**: print what would happen without writing files
4. **`--json`**: output `GenerationResult` as JSON
5. **Colored human output**: `create`, `inject`, `skip`, `conflict` labels with owo-colors

### Implementation Detail

- `src/file_ops.rs`: `inject_before_marker(path, marker, content, comment_style, conflict_mode) -> Result<InjectedContent>`
- `src/file_ops.rs`: `DryRun` mode — return results without writing
- `src/output.rs`: human-readable colored output vs JSON based on `--json` flag
- `Cargo.toml`: add `owo-colors`

### Tests

- [ ] Inject content before `// </jujo:marker>`
- [ ] Inject with `#` comment style (`# </jujo:marker>`)
- [ ] Inject fails with clear error if marker not found
- [ ] Multiple injections into same file (different markers) both succeed
- [ ] Conflict: content already present → error by default
- [ ] `--force` injects even when content already present
- [ ] `--skip-existing` silently skips when content present
- [ ] `--dry-run` produces output but writes no files
- [ ] `--json` output parses as valid JSON matching GenerationResult schema
- [ ] Human output has colored action labels

### Acceptance Criteria

- [ ] Generator with both `create` and `inject` actions works end-to-end
- [ ] `--dry-run --json` shows full preview as structured JSON
- [ ] Conflict detection works correctly

---

## Phase 4: Discovery Commands — init, list, describe, validate

**Goal**: Complete the four-phase agent protocol and onboarding.

### What to build

1. **`jujo init [--lang <lang>]`**: interactive form (language selection) or CLI arg. Creates `.jujo/` with `config.toml` (type map + comment style) and example generator.
2. **`jujo list [--json]`**: list all generators in `.jujo/templates/`
3. **`jujo describe <gen> [--json]`**: show generator schema (inputs, actions, description)
4. **`jujo validate`**: parse all generators + templates, report all errors

### Implementation Detail

- `src/commands/init.rs`: interactive prompts via `inquire` crate. Bundled type maps for common languages (Rust, Go, Python, TypeScript, Java). Scaffolds `.jujo/config.toml` + `.jujo/templates/example/`.
- `src/commands/list.rs`: scan `.jujo/templates/*/generator.toml`, parse each, output list
- `src/commands/describe.rs`: parse one generator, output full schema
- `src/commands/validate.rs`: parse all generators, attempt template rendering with dummy context, report errors
- `src/discovery.rs`: `find_generators() -> Result<Vec<(String, GeneratorDef)>>` — shared by list/validate
- `Cargo.toml`: add `inquire`

### Tests

- [ ] `jujo init --lang rust` creates `.jujo/` with Rust type map
- [ ] `jujo init --lang go` creates `.jujo/` with Go type map
- [ ] `jujo init` fails gracefully if `.jujo/` already exists
- [ ] `jujo list` returns all generators with names and descriptions
- [ ] `jujo list --json` outputs valid JSON array
- [ ] `jujo describe example --json` outputs full schema
- [ ] `jujo validate` catches template syntax errors and reports file + line
- [ ] `jujo validate` catches missing template files referenced in actions
- [ ] `jujo validate` reports all errors (doesn't stop at first)

### Acceptance Criteria

- [ ] Full agent protocol: `list --json` → `describe --json` → `generate --dry-run --json` → `generate --json`
- [ ] `jujo init --lang rust && jujo validate` succeeds
- [ ] `jujo init --lang rust && jujo generate example --var module_name=demo` works end-to-end

---

## Phase 5: AI Customization Markers + Template Management

**Goal**: The differentiating features — AI markers in manifest, template add/remove.

### What to build

1. **AI customization markers**: parse `// <ai:customize hint="...">` and `// </ai:customize>` in rendered output. Add to manifest's `customize` array with file path, line number, and hint.
2. **`jujo template add <name> --from <path>`**: copy a template set into `.jujo/templates/`
3. **`jujo template remove <name>`**: remove a template set
4. **`jujo template list [--json]`**: list template sets (alias for `jujo list`)

### Tests

- [ ] AI markers extracted from rendered templates with correct line numbers
- [ ] Manifest `customize` array populated with path, line, hint
- [ ] `template add` copies generator.toml + templates
- [ ] `template add` fails if name already exists (without --force)
- [ ] `template remove` deletes the directory

### Acceptance Criteria

- [ ] Agent can read `.jujo/last-generate.json` and find all customization points
- [ ] Users can maintain a library of generators and install them into projects
