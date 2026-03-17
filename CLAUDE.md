# Jujo (주조)

## What This Is

Agent-first code generation framework. Users define template sets (TOML + Tera), the CLI stamps out deterministic boilerplate, and outputs structured manifests for LLM agents to customize. "Deterministic codegen followed by agent customization."

The name means "casting/minting" in Korean.

## Stack

- **Language**: Rust
- **CLI**: clap (derive API)
- **Templates**: Tera (Jinja2 syntax, runtime rendering)
- **Config**: TOML (generator definitions)
- **Output**: JSON (agent mode) + colored text (human mode)

## Architecture

Single binary CLI. No daemon, no server, no async needed.

### Key Patterns

- **Data-driven generators** — `generator.toml` defines inputs and actions (create/inject). No code in generator definitions. Templates use `_vars.tera` includes for shared variable derivation.
- **Marker comment injection** — `// </jujo:marker>` in existing files for inserting generated code. Comment style configured via `comment_prefix`/`comment_suffix` in config.toml. Validated across Plop, Hygen, Loco independently.
- **Generation manifest** — `.jujo/last-generate.json` after every generate. Tells agents what was created and where to customize.
- **AI customization markers** — `// <ai:customize hint="...">` in templates. Agents read the manifest to find these.
- **Four-phase agent protocol** — discover (`list --json`) -> schema (`describe --json`) -> preview (`--dry-run --json`) -> execute (`--json`).

### Directory Layout

```
.jujo/                        # project-local config + state
  config.toml                 # project settings
  last-generate.json          # generation manifest (written after each generate)
  templates/                  # user-defined template sets
    <generator-name>/
      generator.toml          # inputs, actions
      _vars.tera              # shared variable derivations (included by templates)
      *.tera                  # template files
```

## Style & Skills

- **STYLE.md** — Coding style guide (TigerBeetle-inspired). Safety, naming, control flow, error handling, testing.
- `.claude/skills/test-guide/` — Layered test standards (unit → snapshot → integration → CLI)
- `.claude/skills/manual-qa/` — Manual QA protocol for CLI binary

## Conventions

- Safety > performance > developer experience
- 70-line function hard limit
- Error-once: return OR log, never both
- Comments say WHY, not what
- `--json` on every command that produces output
- No interactive prompts for `generate` — all inputs via CLI args. Exception: `jujo init` has an interactive form (also supports `--lang` for scripting/agents).
- Conflict resolution: default error, `--force` to overwrite, `--skip-existing` to silently skip

## Testing

- Unit tests for template rendering, naming/casing, field parsing, input validation
- Snapshot tests (insta) for generated file output
- Integration tests with tempdir for full generate workflows
- Compilation test (does generated Rust code compile?) — CI only

## Research

Research findings in `.plan/research/`:
- `generalizable-codegen-tool.md` — core design, components, agent protocol, open questions
- `agent-first-codegen.md` — landscape survey, template system comparison, structured output patterns
- `cli-codegen-patterns.md` — prior art (Rails, Loco, Django, GoFast, cargo-generate), template engines, field parsing
