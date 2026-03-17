# Research: Jujo — Agent-First Codegen Framework

**Date**: 2026-03-17
**Scope**: A framework-agnostic, customizable code generation CLI where the primary consumer is an LLM agent. Standalone Rust project at `~/Desktop/code/jujo/`.

## What Exists

### The Gap

The codegen landscape splits into three buckets with nothing bridging them:

1. **Pure LLM generation** (smol-developer, gpt-engineer, v0, Bolt.new) — no templates, non-deterministic, inconsistent quality
2. **Pure template scaffolding** (Rails, Loco, cookiecutter, Yeoman, Plop, Hygen) — deterministic but dumb, no AI coordination
3. **AI editing tools** (Aider, Claude Code, Cursor, Cline) — smart but no scaffolding, start from scratch each time

**Nobody has built the bridge**: deterministic scaffolding that produces a structured manifest which an AI agent reads to know exactly what to customize.

### Existing Template Systems (detailed in agent-first-codegen.md)

| Tool | Template Location | File Injection | Ongoing Codegen | Agent-Friendly |
|------|------------------|----------------|-----------------|----------------|
| Cookiecutter | Git repos | No | No | Minimal (replay files) |
| Yeoman | npm packages | Custom | Sub-generators | No |
| Plop | Project-local plopfile | append/modify with pattern anchors | Yes | Best (non-interactive mode, single config file) |
| Hygen | Project-local `_templates/` | Frontmatter `inject: true` | Yes | Good (filesystem-as-API, self-documenting) |
| cargo-generate | Git repos | No | No | No |
| Loco (rrgen) | Embedded in binary | Marker comments | Yes | No |

**Key cross-tool findings:**
- Marker comment injection independently invented by Plop, Hygen, Loco — pattern is validated
- Plop's single-file generator definition is the most agent-readable format
- Hygen's frontmatter-as-config is the most self-documenting template format
- No tool produces a generation manifest for downstream consumers
- No tool has idempotent codegen (run twice = fail or overwrite, never converge)
- No Rust tool combines template generation + file injection + ongoing codegen

### The "Deterministic Scaffolding + AI Customization" Pattern

Exists implicitly in practice (devs run `rails generate` then open Cursor) but no tool formalizes it as a protocol. The bridge is a **generation manifest** — a structured file that tells the agent what was scaffolded and where customization is needed.

## What We're Building

### Core Thesis

**Deterministic codegen followed by agent customization.** The CLI generates consistent, compilable boilerplate from user-defined templates. The agent (Claude Code, Cursor, Aider, etc.) reads the generation manifest to know what was created and where the customization points are.

### The Tool — Jujo (주조, "casting/minting")

A standalone Rust CLI that:
1. Lets users **define template sets** (like Plop generators but in TOML + Tera)
2. **Generates files** from those templates with variable substitution
3. **Injects into existing files** via marker comments
4. Outputs **structured JSON** + **generation manifest** for agent consumption
5. Ships with **zero built-in templates** — it's a framework for building project-specific generators

### What Makes It Different

| Feature | Existing Tools | This Tool |
|---------|---------------|-----------|
| Primary consumer | Humans | LLM agents (humans too) |
| Template definition | Code (JS/Python) | Data (TOML + Tera files) |
| Output format | Human-readable text | JSON (machine) + colored text (human) |
| Generation manifest | None | `.jujo/last-generate.json` |
| Customization markers | None / informal TODOs | `// <ai:customize hint="...">` in templates |
| Idempotency | Fail on re-run | `--ensure` mode converges to desired state |
| Discovery | Read docs | `jujolist --json` + `jujodescribe <gen> --json` |
| Dry-run | Some tools, text output | `--dry-run --json` with full file manifest |

### Components

#### 1. Template Registry (User-Defined)

Templates live in a project-local directory (`.jujo/templates/`) or a user-global directory (`~/.jujo/templates/`). Each template set is a directory:

```
.jujo/
  config.toml            # project-level jujoconfig
  templates/
    module/              # a generator called "module"
      generator.toml     # inputs, outputs, description
      routes.rs.tera     # template files
      service.rs.tera
      domain.rs.tera
    endpoint/
      generator.toml
      handler.rs.tera
    migration/
      generator.toml
      migration.sql.tera
```

#### 2. Generator Definition (`generator.toml`)

Inspired by Plop's single-file approach but in TOML (data, not code):

```toml
[generator]
name = "module"
description = "Full module scaffold: routes, service, domain, repository"

# Input variables — agent reads this schema to know what to provide
[[inputs]]
name = "module_name"
type = "string"
description = "Module name in snake_case plural (e.g., 'tenants', 'blog_posts')"
required = true

[[inputs]]
name = "fields"
type = "string[]"
description = "Field specs as name:type (e.g., 'title:string', 'price:decimal?')"
required = false
default = []

# Derived variables — computed from inputs, available in templates
[derived]
entity_name = "{{ module_name | singularize }}"
EntityName = "{{ entity_name | pascal_case }}"
ModuleName = "{{ module_name | pascal_case }}"
table_name = "{{ module_name }}"
route_prefix = "/{{ module_name }}"

# Actions — what this generator does
[[actions]]
type = "create"                          # create new file
template = "routes.rs.tera"
output = "src/{{ module_name }}/routes.rs"

[[actions]]
type = "create"
template = "service.rs.tera"
output = "src/{{ module_name }}/service.rs"

[[actions]]
type = "inject"                          # modify existing file
target = "src/main.rs"
marker = "modules"                       # injects before </jujo:modules>
content = "mod {{ module_name }};"

[[actions]]
type = "inject"
target = "src/main.rs"
marker = "routes"
content = '.nest("/{{ module_name }}", {{ module_name }}::routes::router())'
```

#### 3. CLI Interface

```
jujogenerate <generator> [--var key=value]... [--json] [--dry-run] [--ensure] [--force]
jujolist [--json]                    # list available generators
jujodescribe <generator> [--json]    # show generator schema (inputs, actions)
jujotemplate add <name> [--from <path|git>]   # register a template set
jujotemplate remove <name>
jujotemplate list [--json]
jujoinit                             # create .jujo/ directory with example config
jujovalidate                         # check all templates parse correctly
```

#### 4. Agent Protocol (the differentiator)

Four-phase protocol for agent consumption:

```bash
# Phase 1: Discover — what generators exist?
jujolist --json
# → {"generators": [{"name": "module", "description": "Full module scaffold..."}, ...]}

# Phase 2: Schema — what inputs does this generator need?
jujodescribe module --json
# → {"name": "module", "inputs": [{"name": "module_name", "type": "string", ...}], "actions": [...]}

# Phase 3: Preview — what would this do?
jujogenerate module --var module_name=orders --dry-run --json
# → {"creates": ["src/orders/routes.rs", ...], "injects": [{"file": "src/main.rs", "marker": "modules", ...}]}

# Phase 4: Execute
jujogenerate module --var module_name=orders --json
# → {"created": [...], "injected": [...], "manifest": ".jujo/last-generate.json"}
```

#### 5. Generation Manifest

After every `jujogenerate`, write `.jujo/last-generate.json`:

```json
{
  "generator": "module",
  "timestamp": "2026-03-17T14:30:22Z",
  "inputs": {"module_name": "orders", "fields": ["customer:ref:customers", "total:decimal"]},
  "created": [
    {"path": "src/orders/routes.rs", "template": "routes.rs.tera"},
    {"path": "src/orders/service.rs", "template": "service.rs.tera"}
  ],
  "injected": [
    {"path": "src/main.rs", "marker": "modules", "content": "mod orders;"}
  ],
  "customize": [
    {"path": "src/orders/service.rs", "line": 15, "hint": "Add order validation logic"},
    {"path": "src/orders/domain.rs", "line": 8, "hint": "Add domain-specific state transitions"}
  ]
}
```

The `customize` array is populated from `// <ai:customize hint="...">` markers in the templates. This is the bridge to the agent.

#### 6. AI Customization Markers (in templates)

Templates can include markers that end up in generated code:

```tera
async fn create_{{ entity_name }}(
    State(state): State<AppState>,
    Json(req): Json<Create{{ EntityName }}Req>,
) -> Result<(StatusCode, Json<{{ EntityName }}Response>), AppError> {
    // <ai:customize hint="Add business-specific validation logic">
    let validated = ValidCreate{{ EntityName }}Req::try_from(req)?;
    // </ai:customize>
    let item = service::create_{{ entity_name }}(&state.db, validated).await?;
    Ok((StatusCode::CREATED, Json(item)))
}
```

When the agent reads the manifest, it sees `{"path": "src/orders/service.rs", "line": 15, "hint": "Add business-specific validation logic"}` and knows exactly where to focus.

### Dependencies (Rust crates)

| Crate | Purpose | Notes |
|-------|---------|-------|
| clap | CLI parsing | Derive API |
| tera | Template rendering | Runtime templates, Jinja2 syntax |
| heck | Case conversions | snake_case, PascalCase, etc. |
| serde + serde_json | JSON output + TOML parsing | Core data layer |
| toml | Generator definition parsing | `generator.toml` |
| owo-colors | Colored human output | Zero-alloc, NO_COLOR support |
| anyhow | Error handling | CLI errors |

No `syn`, no `quote`, no `prettyplease`. Lightweight.

### Relationship to cargo-saas

`cargo-saas` (Step 2) becomes a **template set** that ships with or is installable into `jujo`:

```bash
# Install the SaaS template set
jujotemplate add rust-saas --from https://github.com/user/rust-saas-templates

# Now use it
jujogenerate module --var module_name=orders
jujogenerate endpoint --var module=orders --var method=post --var path="/:id/cancel"
```

The SaaS-specific knowledge (8-file module structure, axum patterns, sqlx repository) lives in the templates, not in the tool. The tool is generic.

### Accompanying Claude Skill

A Claude Code skill (`.claude/skills/forge.md`) that:
- Reads `jujolist --json` to discover available generators
- Reads `jujodescribe <gen> --json` to understand inputs
- Runs `jujogenerate ... --dry-run --json` to preview
- Runs `jujogenerate ... --json` to execute
- Reads `.jujo/last-generate.json` to find customization points
- Visits each `ai:customize` marker and fills in domain logic
- Follows project conventions (CLAUDE.md) for the customization

This skill turns "add a billing module with Stripe integration" into:
1. `jujogenerate module --var module_name=billing` (deterministic)
2. Read manifest, customize each marked point with Stripe-specific logic (AI)

## Risk Areas

- **Template language complexity**: How expressive should `generator.toml` be? Too simple = users can't express real generators. Too complex = reinventing a programming language in TOML. The `derived` section with Tera expressions is the pressure point.
- **Tera in TOML**: Using Tera expressions inside TOML values (for derived variables) is unusual. May need a two-pass render: first resolve derived vars, then render templates.
- **Injection ordering**: Multiple generators injecting into the same file at the same marker. Need deterministic ordering or separate markers.
- **Cross-platform paths**: Template output paths use `/` but Windows uses `\`. Need path normalization.
- **Template validation**: Tera templates can reference variables that don't exist. `jujovalidate` needs to catch this statically.

## Open Questions

1. ~~**Name**~~: Resolved — **jujo** (주조, "casting/minting"). Available on crates.io.

2. **Should derived variables use Tera or a simpler expression language?** Tera in TOML is powerful but may confuse users. Alternative: a fixed set of derivation functions (`singularize`, `pluralize`, `pascal_case`, `snake_case`) as TOML table keys.

3. **Should the tool ship with a built-in "starter" template set?** Or be truly empty and require `jujotemplate add` before first use? A starter template (generic Rust module) would improve onboarding but introduces opinion.

4. **Global vs project-local templates**: Should `~/.jujo/templates/` exist for user-global generators that work across projects? Or keep everything project-local?

5. **How to handle template versioning?** When a user updates a template, code already generated from the old version isn't affected (one-shot). But should the manifest track which template version generated each file?

6. **Should this be an MCP server too?** MCP (Model Context Protocol) is the standard for LLM tool consumption. `jujo` as an MCP server would make it directly invocable by any MCP-compatible agent without going through a skill/prompt. This could be a Phase 2 addition.

7. **Priority relative to the SaaS template**: Build SaaS first and extract the codegen tool? Or build the codegen tool first and use SaaS as the first template set?
