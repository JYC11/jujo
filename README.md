# jujo (주조)

Agent-first code generation framework. Define template sets, stamp out deterministic boilerplate, let AI agents customize the rest.

**jujo** means "casting" or "minting" in Korean — you mint your code from molds.

## The Problem

LLM agents waste tokens and produce inconsistent code when generating boilerplate from scratch. Template tools (cookiecutter, Plop, Hygen) produce consistent scaffolding but have no coordination with AI agents. Nobody bridges the gap.

## The Solution

**Deterministic codegen followed by agent customization.**

1. **Define templates** — TOML config + Tera template files describe your project's patterns
2. **Generate scaffolding** — `jujo generate module --var name=billing` stamps out consistent, compilable boilerplate
3. **Agent customizes** — The generation manifest tells the AI exactly what was created and where to add business logic

## Quick Start
- currently we only have build from source
- we will add an install with curl option when project is done
```bash
# Install
cargo install jujo

# Initialize a project (11 languages supported)
cd my-project
jujo init --lang rust

# Create your first generator
mkdir -p .jujo/templates/module
```

Create `.jujo/templates/module/generator.toml`:

```toml
[generator]
name = "module"
description = "Application module with entity and service"

[[inputs]]
name = "module_name"
type = "string"
required = true

[[inputs]]
name = "fields"
type = "field[]"
required = true

[[actions]]
type = "create"
template = "entity.tera"
output = "src/{{ module_name }}/entity.rs"

[[actions]]
type = "create"
template = "service.tera"
output = "src/{{ module_name }}/service.rs"
```

Create `.jujo/templates/module/entity.tera`:

```
{%- set entity_name = module_name | singularize -%}
{%- set EntityName = entity_name | pascal_case -%}
pub struct {{ EntityName }} {
{%- for field in fields %}
    pub {{ field.name }}: {{ field.mapped_type }},
{%- endfor %}
}

// <ai:customize hint="Add domain methods for {{ EntityName }}">
impl {{ EntityName }} {
    // TODO: add methods
}
// </ai:customize>
```

Generate:

```bash
jujo generate module \
  --var module_name=orders \
  --var "fields=title:string,price:decimal,active:bool"
```

## Agent Protocol

jujo is designed for AI agents to drive programmatically via a four-phase protocol:

```bash
# 1. Discover — what generators are available?
jujo list --json

# 2. Schema — what inputs does this generator need?
jujo describe module --json

# 3. Preview — what would this generate?
jujo generate module --var module_name=orders --var "fields=title:string" --dry-run --json

# 4. Execute — generate files + write manifest
jujo generate module --var module_name=orders --var "fields=title:string" --json
```

After generation, the manifest at `.jujo/last-generate.json` tells the agent exactly what was created and where `<ai:customize>` markers need domain-specific logic filled in.

### Claude Code Integration

jujo ships with two Claude Code skills for seamless agent workflows:

- **`/jujo`** — Agent protocol skill. Discovers generators, reads schemas, previews with dry-run, executes, then reads the manifest to find and fill in `<ai:customize>` markers automatically.
- **`/pattern-analyzer`** — Analyzes an existing codebase to extract repeating patterns and auto-generate jujo templates from them. Turns "how we already do it" into reusable generators.

Install the skills by copying `.claude/skills/jujo/` and `.claude/skills/pattern-analyzer/` into your project's `.claude/skills/` directory.

## Features

### Language Support

`jujo init --lang <lang>` sets up comment styles and type mappings for your language:

| Language | Comment Style | Example Types |
|----------|--------------|---------------|
| C# | `//` | `string`, `long`, `bool`, `decimal`, `Guid`, `DateTimeOffset` |
| CSS | `/* */` | `string`, `number`, `boolean` (all map to CSS value types) |
| Elixir | `#` | `String.t()`, `integer()`, `boolean()`, `Decimal.t()`, `DateTime.t()` |
| Go | `//` | `string`, `int64`, `bool`, `decimal.Decimal`, `time.Time` |
| HTML | `<!-- -->` | `string`, `number`, `boolean` (all map to HTML attribute types) |
| Java | `//` | `String`, `Long`, `Boolean`, `BigDecimal`, `Instant` |
| Kotlin | `//` | `String`, `Long`, `Boolean`, `BigDecimal`, `Instant` |
| PHP | `//` | `string`, `int`, `bool`, `float`, `DateTimeImmutable` |
| Python | `#` | `str`, `int`, `bool`, `Decimal`, `datetime` |
| Ruby | `#` | `String`, `Integer`, `Boolean`, `BigDecimal`, `DateTime` |
| Rust | `//` | `String`, `i64`, `bool`, `rust_decimal::Decimal`, `chrono::DateTime<Utc>` |
| Swift | `//` | `String`, `Int64`, `Bool`, `Decimal`, `UUID`, `Date` |
| TypeScript | `//` | `string`, `number`, `boolean`, `Date`, `Record<string, unknown>` |

Add custom languages by creating `~/.jujo/languages.toml` — see `.jujo/languages.toml` (generated during init) for the format.

### Field Types

Abstract types that map to language-specific types via `config.toml`:

`string`, `text`, `int`, `bool`, `float`, `decimal`, `uuid`, `date`, `datetime`, `json`

Append `?` for nullable (e.g., `price:decimal?` renders as `Option<rust_decimal::Decimal>` in Rust).

### Template Filters

```
{{ name | pascal_case }}     →  OrderItem
{{ name | snake_case }}      →  order_item
{{ name | camel_case }}      →  orderItem
{{ name | kebab_case }}      →  order-item
{{ name | upper_case }}      →  ORDER_ITEM
{{ name | singularize }}     →  orders → order
{{ name | pluralize }}       →  order → orders
```

### Injection

Insert generated code into existing files using marker comments:

```rust
// In src/main.rs — add this marker where modules should be registered:
mod existing_module;
// </jujo:modules>
```

```toml
# In generator.toml:
[[actions]]
type = "inject"
target = "src/main.rs"
marker = "modules"
content = "mod {{ module_name }};"
```

Each `jujo generate` inserts content before the marker. Conflicts are detected on re-run; use `--skip-existing` for idempotent injection.

### AI Customization Markers

Templates can mark locations where domain-specific logic should go:

```
// <ai:customize hint="Add validation logic for {{ EntityName }}">
// placeholder
// </ai:customize>
```

After generation, these appear in `.jujo/last-generate.json` with file path, line number, and hint — giving agents precise instructions for what to fill in.

### Post-Generate Hooks

Run formatters or linters on each generated file:

```toml
[hooks]
post_generate = "rustfmt --edition 2024 {file}"
```

`{file}` is replaced with the absolute path. Hook failures warn but don't block generation.

## CLI Reference

```
jujo init [--lang <lang>]                Initialize .jujo/ with language-specific config
jujo generate <name> --var key=value     Generate files from a template set
  --dry-run                                Preview without writing
  --json                                   Structured JSON output
  --force                                  Overwrite existing files
  --skip-existing                          Skip injection if content present
jujo list [--json]                       List available generators
jujo describe <name> [--json]            Show generator input schema
jujo validate                            Check all generators and templates
jujo template add <name> --from <path>   Install a template set from local path
jujo template remove <name>              Remove a template set
jujo template list [--json]              List installed template sets
```

## Project Structure

```
.jujo/
  config.toml                 # Language config: comment style, type map, hooks
  last-generate.json          # Generation manifest (written after each generate)
  templates/
    <generator-name>/
      generator.toml          # Inputs and actions
      _vars.tera              # Shared variable derivations (auto-prepended)
      *.tera                  # Template files
```

## Design Principles

- **Deterministic first.** Templates produce identical output given identical inputs. No randomness, no LLM in the generation loop.
- **Agent-friendly.** Every command supports `--json`. The manifest is the contract between codegen and AI customization.
- **Language-agnostic.** jujo doesn't know or care what language your templates produce. Type maps and comment styles are configured, not hardcoded.
- **No interactive prompts.** `generate` takes all inputs via `--var` flags. Scriptable, CI-friendly, agent-friendly. (Exception: `jujo init` has an interactive form when `--lang` is omitted.)

## License

MIT
