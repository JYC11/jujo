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

## Agent Protocol

```bash
jujo list --json                                    # discover available generators
jujo describe module --json                         # get input schema
jujo generate module --var name=billing --dry-run --json  # preview changes
jujo generate module --var name=billing --json      # execute + write manifest
```

## Status

Early development. Research phase complete, implementation plan in progress.

## License

MIT
