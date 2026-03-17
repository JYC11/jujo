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

Jujo is a code generation CLI. You drive it through a four-phase protocol, then customize
the generated code at marked locations.

## Phase 1: Discover — what generators exist?

```bash
jujo list --json
```

Returns an array of `{ name, description }`. Pick the right generator for the task.

## Phase 2: Schema — what inputs does it need?

```bash
jujo describe <generator> --json
```

Returns `{ name, description, inputs: [...], actions: [...] }`. Each input has:
- `name` — the variable name
- `type` — `string`, `string[]`, `bool`, `int`, or `field[]`
- `required` — whether it must be provided
- `default` — value used if not provided

For `field[]` inputs, provide fields as `name:type` pairs. Types: `string`, `text`, `int`,
`bool`, `float`, `decimal`, `uuid`, `date`, `datetime`, `json`. Append `?` for nullable.

## Phase 3: Preview — what would it do?

```bash
jujo generate <generator> --var key=value --dry-run --json
```

Returns the full `GenerationResult` without writing any files. Review before executing.

## Phase 4: Execute

```bash
jujo generate <generator> --var key=value --json
```

Writes files, injects into existing files, writes manifest. Returns `GenerationResult`.

### Flags

| Flag | Effect |
|------|--------|
| `--var key=value` | Set an input variable. Repeat for multiple. |
| `--json` | Output structured JSON instead of colored text. |
| `--dry-run` | Preview only, write nothing. |
| `--force` | Overwrite existing files. |
| `--skip-existing` | Skip injection if content already present. |

### Field input format

For `field[]` inputs, both formats work:
```bash
# Comma-separated
--var "fields=title:string,price:decimal?,active:bool"

# Repeated
--var fields=title:string --var fields=price:decimal? --var fields=active:bool
```

## Phase 5: Customize — read the manifest

After generation, read `.jujo/last-generate.json`:

```json
{
  "generator": "module",
  "created": [{ "path": "src/orders/service.rs", "template": "service.tera" }],
  "injected": [{ "path": "src/main.rs", "marker": "modules", "content": "mod orders;" }],
  "customize": [
    { "path": "src/orders/service.rs", "line": 15, "hint": "Add validation logic" },
    { "path": "src/orders/routes.rs", "line": 20, "hint": "Add custom routes" }
  ]
}
```

The `customize` array tells you exactly where to add domain-specific logic. Visit each
location and implement what the hint describes, following project conventions.

## Writing Tera Templates

Templates use Jinja2-like syntax. Key features:

```
{{ variable }}                              — insert value
{{ name | pascal_case }}                    — filter: PascalCase
{{ name | snake_case }}                     — filter: snake_case
{{ name | singularize }}                    — filter: orders → order
{{ name | pluralize }}                      — filter: order → orders
{{ name | camel_case }}                     — filter: camelCase
{{ name | kebab_case }}                     — filter: kebab-case
{{ name | upper_case }}                     — filter: UPPER
{% for field in fields %}...{% endfor %}   — loop over array
{% if condition %}...{% endif %}           — conditional
{% set var = expr %}                       — set variable
```

### Shared variables via `_vars.tera`

Create `_vars.tera` in the generator directory. Its content is automatically prepended
to all other templates:

```
{% set entity_name = module_name | singularize %}
{% set EntityName = entity_name | pascal_case %}
```

### AI customization markers

Add these in templates where domain-specific logic should go:

```
// <ai:customize hint="Add validation rules for Order creation">
let validated = req;
// </ai:customize>
```

These are extracted into the manifest's `customize` array after generation.

## Config: `.jujo/config.toml`

```toml
comment_prefix = "//"
comment_suffix = ""

[hooks]
post_generate = "rustfmt {file}"    # run formatter on each created file

[type_map]
string = "String"
int = "i64"
bool = "bool"
decimal = "rust_decimal::Decimal"
```

The `[type_map]` maps abstract types to language-specific types. Used by `field[]` inputs.

## Other Commands

```bash
jujo init --lang rust          # initialize .jujo/ with language type map
jujo validate                  # check all generators parse correctly
jujo template add <name> --from <path>   # install a template set
jujo template remove <name>    # remove a template set
```
