# Agent-First Code Generation Tools — Landscape Research

**Date**: 2026-03-17
**Scope**: AI-agent-driven codegen tools, template registries/plugin systems, structured output for LLM consumption, deterministic scaffolding + AI customization pattern

## 1. Tools Targeting LLM Agents as Primary Consumers

### Finding: This category barely exists as a distinct product category

The vast majority of "AI codegen" tools are **copilot-style** (inline completion) or **chat-based** (Cursor, Copilot Chat, Cody). Tools where an **agent drives a CLI** to scaffold code are rare. What exists:

#### Smol Developer (2023, smol-ai/developer)
- GitHub: ~11K stars
- A "junior developer" agent that takes a product spec and generates an entire codebase.
- **How it works**: Takes a markdown spec + a file list as prompt, then generates each file via LLM calls. No templates — pure LLM generation.
- **No template system**. Every file is generated from scratch by the LLM. This means output is non-deterministic and varies between runs.
- **Lesson**: Pure LLM generation without templates produces inconsistent scaffolding. The quality ceiling is high but the floor is very low.

#### GPT Engineer / gpt-engineer (2023-2024, ~52K stars)
- Takes a natural language prompt, generates a full project.
- v1 was pure generation. Later versions added "improve existing code" mode.
- **Key design**: Uses a `workspace/` directory concept. Agent reads existing files, proposes changes as file-level diffs, human approves.
- **No template system.** All generation is LLM-driven. No deterministic scaffolding layer.
- Pivoted to a hosted product (gptengineer.app / Lovable) — the open-source CLI is effectively abandoned.

#### Aider (paul-gauthier/aider, ~30K+ stars)
- CLI tool for AI pair programming. Agent edits existing code via git commits.
- **Not a scaffolding tool** — it's an editing tool. But relevant because it's one of the most mature "agent drives CLI" patterns.
- **Key design decisions for agent-friendliness**:
  - Structured edit formats: "search/replace blocks" or "whole file" or "unified diff" — each tuned for different LLMs' strengths.
  - Git integration: every change is a commit, easy to review/revert.
  - Repository map: builds a condensed map of the codebase (using tree-sitter) so the LLM understands project structure without reading every file.
  - `--message` flag for non-interactive use (scriptable by other agents).
- **Lesson**: Aider's "repo map" concept is powerful — give the agent a structural overview so it knows where to put things. Their edit format research (which format works best for which LLM) is directly relevant.

#### Claude Code (Anthropic) / Codex CLI (OpenAI)
- Both are agent-in-terminal tools where the LLM reads/writes files directly.
- Neither has a template/scaffolding system — they rely on the LLM's training data + project context to generate code.
- **Claude Code's approach**: Tools (Read, Edit, Write, Bash) + CLAUDE.md instructions. The "template" is the instruction file.
- **Key insight**: These tools prove that **structured project instructions (CLAUDE.md, .cursorrules) function as implicit templates** for AI agents. The agent reads the rules and generates code that follows them. This is the "soft template" pattern.

#### Cline (formerly Continue, ~25K+ stars)
- VS Code extension where agent executes terminal commands, reads/writes files.
- Has a "custom instructions" system similar to CLAUDE.md.
- No built-in scaffolding, but users report using it to run scaffolding CLIs (rails generate, etc.) and then customize the output.
- **Lesson**: Agents already use existing scaffolding CLIs as tools. Making a CLI agent-friendly means making it work well when invoked by Cline/Claude Code/Aider.

#### Mentat (AbanteAI, ~3K stars)
- Agent coding tool, similar space to Aider.
- **Interesting feature**: "block comments" system where you leave `# TODO: implement X` and Mentat fills them in.
- This is essentially the marker comment pattern from our codegen research, but for AI instead of template engines.

### Summary: Agent-First Codegen Gap

**No tool currently combines deterministic scaffolding with AI customization as a first-class workflow.** The landscape splits into:

1. **Pure LLM generation** (smol-developer, gpt-engineer) — no templates, non-deterministic, inconsistent
2. **Pure template scaffolding** (Rails generators, Loco, cookiecutter) — deterministic but dumb, no AI customization
3. **AI editing tools** (Aider, Claude Code, Cline) — smart but no scaffolding, start from blank or existing code

The gap is: a tool that generates deterministic scaffolding (phase 1) and then invokes/guides an AI agent to customize it (phase 2). **This is our opportunity.**

---

## 2. Template Registries / Plugin Systems for Codegen

### Cookiecutter (Python)

**GitHub**: ~22K+ stars. The most widely-used project templating tool across languages.

**Template system mechanics**:
- Templates are **Git repos** (or local directories) containing a project skeleton.
- Variables defined in `cookiecutter.json` at the repo root:
  ```json
  {
    "project_name": "My Project",
    "project_slug": "{{ cookiecutter.project_name.lower().replace(' ', '_') }}",
    "author": "Your Name",
    "use_docker": ["yes", "no"],
    "database": ["postgres", "sqlite", "mysql"]
  }
  ```
- **Jinja2 templating** — both file contents AND file/directory names are Jinja2 templates. `{{cookiecutter.project_slug}}/src/main.py` becomes `my_project/src/main.py`.
- **Hooks**: `hooks/pre_gen_project.py` and `hooks/post_gen_project.py` run before/after generation. Used for validation, cleanup (delete files based on choices), git init, etc.
- **Conditional file inclusion**: Post-gen hook deletes files. E.g., if `use_docker == "no"`, hook removes `Dockerfile`.
- **No registry**. Templates are discovered by URL. Community maintains curated lists (cookiecutter-awesome). Anyone can publish a template as a Git repo.
- **No composition**. You can't combine multiple cookiecutter templates. Each template is a standalone project skeleton.

**Limitations relevant to us**:
- One-shot project generation only — no ongoing codegen for adding modules.
- No file injection (can't modify existing files).
- No partial templates (can't generate just one file into an existing project).

### Yeoman (JS)

**GitHub**: ~9K+ stars on `yo` CLI. Significant ecosystem but declining usage.

**Generator plugin model**:
- Generators are **npm packages** named `generator-<name>`. Install: `npm install -g generator-react`, run: `yo react`.
- **Sub-generators**: `yo react:component MyButton` — colon syntax for sub-commands within a generator. This maps directly to our `cargo saas generate module/endpoint/entity` pattern.
- **Composability**: Generators can call other generators. `this.composeWith('generator-common', {})` invokes another generator as part of yours. This is Rails' hook system equivalent.
- **File system abstraction**: Yeoman provides `this.fs` (mem-fs-editor) — an in-memory filesystem that batches writes. Files are staged in memory, then committed to disk atomically. Enables conflict detection (file already exists → prompt user).
- **Template engine**: EJS (`<%= name %>`) by default. `this.fs.copyTpl(source, dest, context)`.
- **Prompting built-in**: `this.prompt([{type: 'input', name: 'appName', message: 'App name?'}])` — interactive prompts are a first-class concept.
- **Run loop**: Generators execute in a priority queue: `initializing → prompting → configuring → default → writing → conflicts → install → end`. Each method name maps to a phase.
- **Storage**: `.yo-rc.json` persists configuration between runs. Enables `yo react:component` to know the project's settings without re-prompting.

**Key lessons for agent-first design**:
- The sub-generator pattern (`:component`, `:route`) is good UX for both humans and agents.
- The run loop with phases is over-engineered for a Rust CLI. Simpler: validate → derive → render → write.
- `.yo-rc.json` persistence is valuable — an agent can read this to understand project configuration. Our equivalent: a `.saas/config.toml` or similar manifest.
- **Conflict resolution is critical for agent workflows.** An agent might run the generator multiple times. Yeoman's conflict detection (ask: overwrite/skip/diff) needs an agent-friendly equivalent: `--force` (overwrite) or `--skip-existing`.

### Plop.js

**GitHub**: ~10K+ stars. "Micro-generator framework."

**Design philosophy**: The opposite of Yeoman — minimal, no npm ecosystem, single config file.

**How it works**:
- Single file `plopfile.js` (or `.mjs`, `.cjs`) defines generators:
  ```js
  export default function(plop) {
    plop.setGenerator('component', {
      description: 'React component',
      prompts: [{type: 'input', name: 'name', message: 'Component name?'}],
      actions: [{
        type: 'add',
        path: 'src/components/{{pascalCase name}}/index.tsx',
        templateFile: 'plop-templates/component.hbs'
      }, {
        type: 'append',
        path: 'src/components/index.ts',
        pattern: /\/\/ PLOP_APPEND/,
        template: "export { {{pascalCase name}} } from './{{pascalCase name}}';"
      }]
    });
  }
  ```
- **Template engine**: Handlebars with built-in case helpers: `{{pascalCase name}}`, `{{camelCase name}}`, `{{snakeCase name}}`, `{{kebabCase name}}`, `{{dashCase name}}`, `{{dotCase name}}`, `{{pathCase name}}`, `{{lowerCase name}}`, `{{upperCase name}}`, `{{sentenceCase name}}`, `{{constantCase name}}`, `{{titleCase name}}`.
- **Action types**:
  - `add` — create new file from template
  - `addMany` — create multiple files (glob pattern for templates)
  - `modify` — regex find/replace in existing file
  - `append` — append to file (with optional pattern anchor — **this is their marker comment equivalent**)
  - Custom action functions for arbitrary logic
- **No registry**. Generators are local to the project (plopfile.js + templates directory).
- **Runnable non-interactively**: `plop component -- --name MyButton` bypasses prompts. **Agent-friendly.**

**Key lessons**:
- The `append` action with `pattern` anchor is exactly our marker comment injection pattern, independently invented.
- Plop's action-based model (add/modify/append) is a clean abstraction. Our generators do the same three operations.
- Non-interactive mode via `--` is essential for agent consumption. Our `--force` + CLI args (no prompts) already follows this.
- **Having ALL generators defined in ONE file (plopfile) is powerful for agents.** An agent can read one file and understand all available generators + their inputs. We should consider a manifest file that describes our generators' capabilities.

### Hygen

**GitHub**: ~5K+ stars. "The scalable code generator that lives in your project."

**Design**:
- Templates live in `_templates/` directory in the project root.
- Directory structure IS the generator definition:
  ```
  _templates/
    component/
      new/
        component.ejs.t
        test.ejs.t
        index.ejs.t
      help/       <-- `hygen component help`
        index.ejs.t
    module/
      new/
        ...
  ```
- **Frontmatter-driven**: Each template file has YAML frontmatter controlling output:
  ```
  ---
  to: src/components/<%= name %>/index.tsx
  ---
  import React from 'react'

  export const <%= Name %> = () => <div><%= name %></div>
  ```
- **Injection via frontmatter**:
  ```
  ---
  inject: true
  to: src/components/index.ts
  after: "// hygen-inject"
  ---
  export { <%= Name %> } from './<%= Name %>'
  ```
  The `after:` / `before:` / `at_line:` / `prepend:` / `append:` directives control where content is injected.
- **Template engine**: EJS (`<%= %>`, `<%- %>` for unescaped).
- **Shell actions**: `sh: cd <%= cwd %> && npm install` in frontmatter.
- **Prompt files**: `_templates/component/new/prompt.js` defines interactive prompts (Enquirer). **Optional — if no prompt file, all variables come from CLI args.**
- **No registry**. Templates are project-local by design.

**Key lessons**:
- **Frontmatter-as-config is brilliant for agent readability.** An agent can parse the YAML frontmatter to understand what each template does, where it writes, and what it injects — without executing anything.
- **"inject: true" + "after: marker"** = independent invention of the same marker-comment pattern. Widely validated.
- **File-system-as-API**: The directory structure `_templates/<generator>/<action>/` is self-documenting. An agent can `ls _templates/` to discover all generators.
- Hygen's approach of "templates live in the project" (not in a separate tool) is the same as our `.saas/templates/` override directory.

### Rust Equivalents

**cargo-generate** (covered in cli-codegen-patterns.md):
- Handlebars templates, `cargo-generate.toml` config, Git repo or local templates.
- Project bootstrapper only, no ongoing codegen, no file injection.

**rrgen** (Loco's internal crate):
- Tera templates, embedded in binary.
- Not published as a standalone tool — tightly coupled to Loco's SeaORM model.

**cargo-scaffold** (~200 stars, stale):
- Similar to cargo-generate. TOML config + Handlebars. Mostly abandoned.

**ffizer** (~100 stars):
- "Files from Template" — Handlebars, supports both project generation and "apply template to existing project."
- Interesting because it supports **applying templates to existing directories** (not just empty projects). Uses `.ffizer.yaml` manifest.
- Low adoption, appears unmaintained.

**No Rust tool combines template generation + file injection + ongoing codegen.** This gap is real and matches what we found in cli-codegen-patterns.md. Our `cargo-saas` would be novel in the Rust ecosystem.

### Cross-Tool Comparison: Template System Design Patterns

| Feature | Cookiecutter | Yeoman | Plop | Hygen | cargo-generate |
|---------|-------------|--------|------|-------|----------------|
| Template engine | Jinja2 | EJS | Handlebars | EJS | Handlebars |
| Template location | Git repo | npm package | Project-local | Project-local | Git repo |
| File injection | No | Yes (custom) | Yes (append/modify) | Yes (frontmatter) | No |
| Sub-generators | No | Yes (`:name`) | Multiple generators | Directory-based | No |
| Composition | No | Yes (composeWith) | No | No | No |
| Non-interactive | Replay file | CLI args | `--` bypass | CLI args | `--define` |
| Manifest/config | cookiecutter.json | package.json | plopfile.js | Directory structure | cargo-generate.toml |
| Case helpers | Jinja2 filters | Manual | Built-in (12 cases) | Manual | None |

---

## 3. Structured Output Patterns for CLI Tools Consumed by LLMs

### Emerging Conventions

#### JSON Output Mode
The most common pattern. Tools add `--json` or `--output json` flags:

- **GitHub CLI (`gh`)**: `gh pr list --json number,title,state` — column selection + JSON output. Agent-friendly because the agent can request exactly the fields it needs.
- **Cargo**: `cargo metadata --format-version 1` outputs JSON workspace/dependency graph. Consumed by tools like rust-analyzer, cargo-deny, etc.
- **Docker**: `docker inspect --format '{{json .}}'` — JSON output of container state.
- **kubectl**: `kubectl get pods -o json` — full JSON output. Also `-o jsonpath='{.items[*].metadata.name}'` for structured queries.
- **npm**: `npm ls --json`, `npm info <pkg> --json`.
- **jq as lingua franca**: Many agent workflows pipe CLI output through `jq` for structured extraction. This works but is fragile — better for the CLI to output structured data natively.

**Pattern**: The convention is `--json` flag that changes output from human-readable to machine-readable. Some tools (gh) allow field selection. This is the minimum bar for agent-friendliness.

#### Manifest / Descriptor Files

Tools that describe themselves via static files an agent can read without executing:

- **OpenAPI / Swagger**: REST API descriptions. Agents can read the spec to understand available endpoints. CodeGen tools (openapi-generator) consume these to produce client code.
- **JSON Schema**: Used by VS Code settings, package.json, tsconfig.json, etc. Agents can read schemas to understand valid configurations.
- **MCP (Model Context Protocol)**: Anthropic's protocol for tools consumed by LLMs. Tools describe their capabilities via JSON schema. The agent reads the schema, then invokes tools with structured arguments. **This is the most explicit "tools designed for agent consumption" standard.**
- **package.json `scripts`**: npm scripts are a de facto manifest of available commands. Agents read `package.json` to discover what commands a project supports.
- **Makefile / Taskfile / Justfile**: Task runners that double as self-documenting command manifests. `just --list` outputs available commands with descriptions.
- **GitHub Actions `action.yml`**: Describes inputs, outputs, and runs. Agent-consumable metadata.

**Pattern**: A static manifest file that describes what the tool can do, what inputs it accepts, and what outputs it produces — parseable without executing the tool.

#### Dry-Run / Plan Protocols

Tools that preview changes before applying:

- **Terraform**: `terraform plan` outputs a structured change plan (JSON with `--json`). Shows what will be created/modified/destroyed. `terraform apply` executes. The plan is a structured diff that an agent can review.
- **Ansible**: `--check` (dry-run) + `--diff` (show changes). Not JSON by default, but `ANSIBLE_STDOUT_CALLBACK=json` enables it.
- **Pulumi**: `pulumi preview` with `--json` outputs structured resource changes.
- **Rails generators**: `--pretend` flag shows what files would be created/modified without writing.
- **Our `--dry-run`**: Already planned in cli-codegen-patterns.md. Should output JSON in agent mode.

**Pattern**: Two-phase protocol: preview (structured output of planned changes) → confirm → apply. Enables agent review before destructive operations.

#### Idempotency

- **Terraform / Pulumi / CloudFormation**: Declarative — describe desired state, tool computes diff. Running twice produces same result.
- **Most codegen tools are NOT idempotent.** Running `rails generate scaffold Post` twice fails or overwrites. This is a problem for agents that may retry on failure.
- **Idempotent codegen would mean**: "ensure module `tenants` exists with these files." If already exists, skip or update. If partially exists, fill in missing pieces.

**Pattern**: Idempotent generators that converge to desired state rather than failing on re-runs. Critical for agent reliability.

### Proposed Agent-Friendly CLI Protocol

Based on patterns above, an agent-friendly CLI should support:

```
# 1. Discovery — what can this tool do?
cargo saas --capabilities --json
# Returns: { "generators": ["module", "endpoint", "entity", ...], "commands": [...] }

# 2. Schema — what inputs does each generator need?
cargo saas generate module --schema --json
# Returns: JSON Schema for the module generator's inputs

# 3. Dry-run — what would this command do?
cargo saas generate module tenants --dry-run --json
# Returns: { "creates": ["src/tenants/mod.rs", ...], "modifies": ["src/main.rs"], "migrations": [...] }

# 4. Execute — do it
cargo saas generate module tenants --json
# Returns: { "created": [...], "modified": [...], "summary": "..." }

# 5. Verify — did it work?
cargo saas verify module tenants --json
# Returns: { "status": "ok", "files": [...], "missing": [] }
```

This four-phase protocol (discover → schema → dry-run → execute) maps cleanly to how LLM agents work: they need to understand capabilities, construct valid inputs, preview effects, then execute.

---

## 4. "Deterministic Scaffolding + AI Customization" Pattern

### Finding: The pattern exists in practice but has almost no formal writing

Nobody has published a definitive blog post or paper naming this pattern. But several projects implement it implicitly:

#### Pattern A: "Scaffold then Edit" (current practice, ad hoc)

This is what developers actually do today with Claude Code / Cursor / Aider:
1. Run `rails generate scaffold Post title:string body:text` (deterministic)
2. Open the generated code in Cursor/Claude Code
3. Say "add soft delete to this Post model, update the migration, add a `deleted_at` timestamp, make the list endpoint filter out deleted posts"
4. AI modifies the scaffolded code

**This works but is uncoordinated.** The AI doesn't know what the scaffold produced. It reads the files and figures it out. There's no protocol between the scaffold step and the AI step.

#### Pattern B: "Template + AI Slots" (emerging)

Some developers leave explicit TODO markers for AI:

```rust
// In a generated routes.rs:
async fn create_tenant(
    State(state): State<AppState>,
    Json(req): Json<CreateTenantReq>,
) -> Result<(StatusCode, Json<TenantResponse>), AppError> {
    // TODO(ai): Add business-specific validation logic here
    let item = service::create_tenant(&state.db, req).await?;
    Ok((StatusCode::CREATED, Json(item)))
}
```

The `TODO(ai)` markers are informal, but tools like Mentat and Continue scan for TODO comments as entry points for AI completion. This is a bridge between deterministic scaffolding and AI customization.

**Cursor's `.cursorrules` / Claude's `CLAUDE.md`**: These are essentially "soft templates" — they don't generate code, but they constrain how AI generates code. Combined with scaffolded files, they create a two-layer system:
1. Hard template: File structure, boilerplate, wiring (deterministic)
2. Soft template: Coding conventions, patterns, constraints (AI follows when customizing)

#### Pattern C: Vercel's v0 / Bolt.new / Lovable (2024-2025)

These tools generate full projects from natural language, but they use **deterministic scaffolding internally**:

- **v0** (Vercel): Generates React/Next.js components. Uses a fixed project structure (Next.js conventions) as the skeleton, then AI fills in component logic. The project structure is deterministic; the component code is AI-generated.
- **Bolt.new** (StackBlitz): Generates full-stack apps in a WebContainer. Uses predetermined dependency sets and project structures, then AI generates the custom code.
- **Lovable** (ex-gptengineer.app): Similar — fixed React+Supabase template, AI customizes.

These are **not explicit two-phase tools** — the scaffolding is hidden inside the system prompt. But architecturally, they prove the pattern: deterministic skeleton + AI-generated custom logic.

#### Pattern D: "Convention-Oriented AI" (Loco + AI tools)

Loco's approach accidentally creates a great AI workflow:
1. `cargo loco generate scaffold Post title:string body:text` creates 100% deterministic, convention-following code
2. Every file follows an exact pattern (SeaORM model, controller, migration)
3. AI tools (Claude Code, Cursor) can then modify these files effectively because the conventions are predictable

The convention-heavy approach (Rails, Loco) produces code that is **more AI-editable** than custom architectures. If the AI knows "every controller has this shape," it can confidently modify it.

**Key insight**: Strong conventions + deterministic scaffolding = better AI customization. The two phases amplify each other.

#### Pattern E: GitHub Copilot Workspace (2024-2025)

GitHub's Copilot Workspace (preview) implements something close to the two-phase pattern:
1. **Plan phase**: AI generates a structured plan (list of files to create/modify, with descriptions)
2. **Implementation phase**: AI generates the actual code changes
3. **Human review**: Developer reviews, edits, approves

This is the closest to a formalized "deterministic plan + AI execution" workflow. The plan step could be replaced by (or augmented with) template-based scaffolding, with the AI handling customization.

### The Opportunity: Explicit Two-Phase Protocol

What doesn't exist yet (our opportunity):

```
Phase 1: Deterministic Scaffolding
  cargo saas generate module orders --fields "customer:ref:customers total:decimal status:string"
  → Creates 8 files with deterministic, compilable code
  → Generated code includes AI markers: // <ai:customize> ... // </ai:customize>
  → Outputs a structured manifest: .saas/last-generate.json
    {
      "generator": "module",
      "name": "orders",
      "created": ["src/orders/routes.rs", ...],
      "modified": ["src/main.rs"],
      "ai_markers": [
        {"file": "src/orders/service.rs", "line": 15, "hint": "Add order validation logic"},
        {"file": "src/orders/domain.rs", "line": 8, "hint": "Add order state machine transitions"}
      ]
    }

Phase 2: AI Customization (optional, invoked by agent or developer)
  Agent reads .saas/last-generate.json
  Agent knows exactly which files were created and what needs customization
  Agent visits each ai_marker and fills in domain-specific logic
  Agent follows CLAUDE.md / project conventions for the customization
```

**The manifest file is the bridge.** It transforms ad-hoc "scaffold then edit" into a coordinated protocol. The AI doesn't have to discover what was generated — it's told explicitly.

---

## 5. Implications for Our CLI Design

### What to build now (Phase 1 CLI)

Keep the existing plan from cli-codegen-patterns.md — Tera templates, marker comments, clap CLI, cargo-saas binary. This is solid.

### What to add for agent-friendliness

1. **`--json` output mode on all commands.** Structured output of what was created/modified. Costs almost nothing to add. High value for agents.

2. **`--dry-run --json` combination.** Agents can preview before executing. Already planned but emphasize the JSON aspect.

3. **`--capabilities` flag or `cargo saas list` command.** Outputs available generators and their schemas. Agents can discover what the tool can do without documentation.

4. **Generation manifest (`.saas/last-generate.json`).** After every generate command, write a manifest of what was done. Agents (and humans) can review, and AI customization tools can read it to know what to customize.

5. **AI marker comments in generated code.** `// <ai:customize>` blocks in generated templates where domain-specific logic should go. This bridges the scaffold → AI customization workflow. The markers are also useful for human developers — they flag "you need to change this."

6. **Idempotent `--ensure` mode** (Phase 2). `cargo saas generate module tenants --ensure` succeeds even if the module already exists (fills in missing files, skips existing ones). Critical for agent reliability.

### What NOT to build

- Don't build an AI agent into the CLI. The CLI is the tool; Claude Code / Cursor / Aider is the agent.
- Don't make the CLI interactive. Agents can't handle interactive prompts well. Keep everything as CLI args.
- Don't build a template registry / marketplace. Project-local templates are sufficient. Registries add complexity without proportional value for a single-template project.

---

## 6. Key Takeaways

| Finding | Confidence | Implication |
|---------|------------|-------------|
| No tool combines deterministic scaffolding + AI customization as a first-class protocol | High | Opportunity: we can be first |
| `--json` output is the minimum bar for agent-friendly CLIs | High | Add to all commands |
| Marker comment injection is independently invented by Plop, Hygen, Loco, and us | High | Pattern is validated across ecosystems |
| Generation manifests (.saas/last-generate.json) don't exist in any codegen tool | High | Novel feature, high value for agents |
| AI markers in generated code (// <ai:customize>) are informal but emerging | Medium | Formalize in our templates |
| Plop's plopfile.js (all generators in one file) is agent-readable by design | High | Consider a `.saas/generators.toml` manifest |
| Hygen's frontmatter-as-config is the most self-documenting template format | High | Evaluate for our template format |
| Strong conventions make AI customization more effective | High | Validates our convention-heavy architecture |
| Idempotent codegen is missing from ALL template tools | High | `--ensure` mode would be novel and agent-valuable |
