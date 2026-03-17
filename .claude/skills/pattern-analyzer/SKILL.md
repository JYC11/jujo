---
name: pattern-analyzer
description: >
  Analyze an existing codebase to extract repeating patterns and auto-generate jujo generator
  templates from them. Turns "how we already do it" into reusable generators.
  Use when: the user says "analyze patterns", "extract templates", "create generator from existing",
  "reverse engineer templates", "make a generator for this pattern", "automate this boilerplate",
  or wants to turn repeated code structures into jujo generators.
  Triggers on: "analyze pattern", "extract template", "create generator from", "reverse engineer",
  "automate boilerplate", "find patterns", "what repeats".
---

# Pattern Analyzer

Analyze an existing codebase to find repeating structural patterns and generate jujo templates
from them. This turns manually-copied boilerplate into automated, deterministic generators.

## When to Use

- Project has 3+ modules/components that follow the same file structure
- Developer says "every time I add a module I copy-paste these files"
- Onboarding new developers who need to create components matching existing patterns
- Migrating from ad-hoc codegen to structured jujo generators

## Workflow

### Step 1: Identify candidate patterns

Look for directories that share the same file structure:

```bash
# Find directories with similar file sets
find src -type d -mindepth 1 -maxdepth 1 | while read dir; do
  echo "$(ls "$dir" | sort | tr '\n' ',')" "$dir"
done | sort | uniq -c -w 50 | sort -rn
```

A pattern exists when 3+ directories have the same set of files.

**Example output:**
```
5 entity.rs,mod.rs,routes.rs,service.rs  src/orders
5 entity.rs,mod.rs,routes.rs,service.rs  src/products
5 entity.rs,mod.rs,routes.rs,service.rs  src/customers
```

This tells you: there's a "module" pattern with 4 files used by 5 modules.

### Step 2: Diff instances to find the variable parts

Pick 2-3 instances of the pattern and diff them:

```bash
diff src/orders/entity.rs src/products/entity.rs
diff src/orders/service.rs src/products/service.rs
```

The differences are the **variables** — the parts that change between instances.
The identical parts are the **template** — the boilerplate that stays the same.

Common variable categories:
- **Names**: module name, entity name, table name (usually derived from each other)
- **Fields**: struct fields, SQL columns, form inputs (usually a list)
- **Flags**: boolean toggles that include/exclude sections (e.g., tenant_scoped)
- **Types**: field types that map to language types

### Step 3: Classify variables

For each variable part found in the diff:

| What changes | Variable type | Input type |
|---|---|---|
| Module/entity name (singular/plural/PascalCase) | All derived from one input | `string` → use filters |
| List of struct fields with types | Iterable list | `field[]` |
| Sections present in some instances but not others | Boolean toggle | `bool` |
| A value chosen from a fixed set | Selection | `string` with description |

### Step 4: Build the generator

Create the jujo generator structure:

```
.jujo/templates/<pattern-name>/
  generator.toml     ← inputs + actions from Step 3
  _vars.tera         ← name derivations (singularize, pascal_case, etc.)
  <file1>.tera       ← template from Step 2 with {{ variables }}
  <file2>.tera
  ...
```

**For each file in the pattern:**

1. Start with a real instance (e.g., `src/orders/entity.rs`)
2. Replace the module-specific name with `{{ entity_name }}` / `{{ EntityName }}`
3. Replace the field list with `{% for field in fields %}...{% endfor %}`
4. Replace conditional sections with `{% if flag %}...{% endif %}`
5. Add `// <ai:customize hint="...">` markers where domain logic varies

**For injection points** (e.g., `mod X;` in `main.rs`):

1. Find where modules are registered (imports, route mounting, etc.)
2. Add `// </jujo:marker>` closing tags at those locations
3. Add `inject` actions to `generator.toml`

### Step 5: Validate

```bash
jujo validate
```

Then test by generating a new instance:

```bash
jujo generate <pattern> --var module_name=test_module --var "fields=name:string" --dry-run
```

Compare the dry-run output against a real instance to verify the template is correct.

### Step 6: Document

Add a description to `generator.toml` that explains:
- What the generator creates
- What inputs it expects (with examples)
- What markers need to exist in target files for injection

## Example: Extracting a "module" pattern

Given three existing modules (`orders`, `products`, `customers`), each with:
```
src/<name>/
  mod.rs        — re-exports
  entity.rs     — struct + DTOs
  service.rs    — business logic
  routes.rs     — HTTP handlers
```

**Diff reveals:**
- `Order` / `Product` / `Customer` → entity name (derived from module name)
- Different struct fields → `field[]` input
- Some modules have `tenant_id`, others don't → `tenant_scoped` bool

**Resulting generator.toml:**
```toml
[generator]
name = "module"
description = "Full module: entity, service, routes"

[[inputs]]
name = "module_name"
type = "string"
required = true

[[inputs]]
name = "fields"
type = "field[]"
required = true

[[inputs]]
name = "tenant_scoped"
type = "bool"
required = false
default = true

[[actions]]
type = "create"
template = "entity.tera"
output = "src/{{ module_name }}/entity.rs"

# ... more create actions ...

[[actions]]
type = "inject"
target = "src/main.rs"
marker = "modules"
content = "mod {{ module_name }};"
```

## Tips

- **Start with the simplest pattern.** Extract the pattern with fewest variations first.
- **3 instances minimum.** With only 2 instances, it's hard to tell which differences are
  structural vs incidental.
- **Don't over-parameterize.** If something varies between instances but is always filled
  in by an AI agent anyway, make it an `ai:customize` marker instead of an input variable.
- **Test with a real name.** Generate with a real module name and compare against existing
  instances. The output should be structurally identical.
- **Check injection markers.** After adding markers to existing files, run `jujo validate`
  to ensure they're found.
