---
name: jujo
description: >
  Use the jujo CLI to generate code from template sets. Follow the four-phase agent protocol:
  discover generators, read schemas, preview with dry-run, then execute. After generation,
  read the manifest to find AI customization markers and fill them in.
  Triggers on: "generate module", "scaffold", "jujo generate", "add module", "create entity",
  "new endpoint", "codegen", "stamp out", "generate boilerplate".
---

# Jujo Agent Protocol

Jujo is a language-agnostic code generation CLI. Users define template sets (TOML + Tera),
jujo stamps out files deterministically, and writes a structured manifest for agents to
customize the output. It works with any language or framework.

## Four-Phase Agent Protocol

### Phase 1: Discover

```bash
jujo list --json
```

Returns `[{ "name": "...", "description": "..." }, ...]`. Pick the right generator.

### Phase 2: Schema

```bash
jujo describe <generator> --json
```

Returns `{ name, description, inputs: [...], actions: [...] }`. Each input has:
- `name` — the variable name
- `type` — `string`, `string[]`, `bool`, `int`, or `field[]`
- `required` / `default`

### Phase 3: Preview

```bash
jujo generate <generator> --var key=value --dry-run --json
```

Returns the full result without writing files. Review before executing.

### Phase 4: Execute

```bash
jujo generate <generator> --var key=value --json
```

Writes files, injects into existing files, writes manifest.

### Phase 5: Customize

Read `.jujo/last-generate.json`. The `customize` array lists exactly where to add
domain-specific logic:

```json
{
  "customize": [
    { "path": "src/orders/service.rs", "line": 15, "hint": "Add validation logic" }
  ]
}
```

Visit each location, implement what the hint describes, following project conventions.

## CLI Flags

| Flag | Effect |
|------|--------|
| `--var key=value` | Set an input variable. Repeat for multiple. |
| `--json` | Structured JSON output (for agents). |
| `--dry-run` | Preview only, write nothing. |
| `--force` | Overwrite existing files. |
| `--skip-existing` | Skip injection if content already present. |

### Field input format

For `field[]` inputs, both formats work:
```bash
--var "fields=title:string,price:decimal?,active:bool"   # comma-separated
--var fields=title:string --var fields=price:decimal?     # repeated
```

Abstract types: `string`, `text`, `int`, `bool`, `float`, `decimal`, `uuid`, `date`,
`datetime`, `json`. Append `?` for nullable. Types map to language-specific types via
`config.toml` `[type_map]`.

## Writing Tera Templates

Templates use Jinja2-like syntax:

```
{{ variable }}                              — insert value
{{ name | pascal_case }}                    — PascalCase
{{ name | snake_case }}                     — snake_case
{{ name | camel_case }}                     — camelCase
{{ name | kebab_case }}                     — kebab-case
{{ name | upper_case }}                     — UPPER_CASE
{{ name | singularize }}                    — orders → order
{{ name | pluralize }}                      — order → orders
{% for item in list %}...{% endfor %}      — loop
{% if condition %}...{% endif %}           — conditional
{% set var = expr %}                       — set variable
```

### Whitespace control (CRITICAL)

Tera block tags (`{% %}`) render as blank lines. Use `{%-` / `-%}` to trim whitespace.

| Syntax | Effect |
|--------|--------|
| `{%-` | Trims whitespace/newline BEFORE the tag |
| `-%}` | Trims whitespace/newline AFTER the tag |

**Rules:**
- Use `{%-` on control-flow lines (for/endfor/if/endif/else/set) to prevent blank lines
- Use `{%- ... -%}` on lines that produce NO output (set, endfor, endif)
- Do NOT use `-%}` when the tag is followed by output content on the same line

**Pattern — loop over items:**
```
{%- for field in fields %}
    {{ field.name }}: {{ field.mapped_type }},
{%- endfor %}
```

**Pattern — conditional inclusion:**
```
{%- if feature_enabled %}
    extra_config: true,
{%- endif %}
```

**Pattern — nullable check:**
```
{%- for field in fields %}
{%- if field.nullable %}
    {{ field.name }}?: {{ field.mapped_type }};
{%- else %}
    {{ field.name }}: {{ field.mapped_type }};
{%- endif %}
{%- endfor %}
```

### Shared variables via `_vars.tera`

Create `_vars.tera` in the generator directory. Its content is automatically prepended
to all other templates. Use `{%- -%}` to avoid blank lines:

```
{%- set entity_name = module_name | singularize -%}
{%- set EntityName = entity_name | pascal_case -%}
{%- set table_name = module_name -%}
```

### AI customization markers

Add markers where domain-specific logic should go. Use the project's comment style:

```
// <ai:customize hint="Add validation logic here">
placeholder_code();
// </ai:customize>
```

Works with any comment syntax — `//`, `#`, `<!-- -->`. These are extracted into the
manifest's `customize` array with file path, line number, and hint.

### Injection markers

Templates can inject content into existing files. The target file needs a closing marker:

```
// </jujo:modules>
```

The generator's inject action inserts content before this marker. Comment style is
configured in `config.toml` (`comment_prefix` / `comment_suffix`).

## Config: `.jujo/config.toml`

```toml
comment_prefix = "//"      # or "#" for Python, "<!--" for HTML
comment_suffix = ""         # or "-->" for HTML

[hooks]
post_generate = "fmt {file}"   # run a formatter on each created file

[type_map]
string = "String"    # language-specific type mappings
int = "i64"          # used by field[] inputs
bool = "bool"
decimal = "Decimal"
```

The `[type_map]` maps jujo's abstract types to your language's types. `jujo init --lang`
populates this for common languages (Rust, Go, Python, TypeScript, Java).

The `[hooks]` section runs commands on each generated file. `{file}` is replaced with
the absolute file path. Hook failures warn but don't stop generation.

## Other Commands

```bash
jujo init [--lang <lang>]              # create .jujo/ with type map for a language
jujo validate                          # check all generators and templates
jujo template add <name> --from <path> # install a template set from local path
jujo template remove <name>            # remove a template set
jujo template list [--json]            # list installed template sets
```
