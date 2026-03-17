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

## Core Principle — Observe, Don't Judge

You are a **pattern extractor**, not a code reviewer. The existing codebase is the source of truth.
Your job is to faithfully capture how the codebase already works — including quirks, legacy conventions,
and inconsistencies that exist for reasons you don't know. **Never** silently "fix", "improve", or
normalize code patterns you find. If something looks wrong or inconsistent, escalate to the user.

## When to Use

- Project has 3+ modules/components that follow the same file structure
- Developer says "every time I add a module I copy-paste these files"
- Onboarding new developers who need to create components matching existing patterns
- Migrating from ad-hoc codegen to structured jujo generators

## Escalation Rules

These rules apply throughout the entire workflow:

### CRITICAL — When to escalate

Escalate with AskUserQuestion at every decision point listed below. **Never** make judgment calls
about legacy code autonomously. Specifically:

1. **Pattern grouping** (Step 1) — Which directory groups constitute real patterns vs coincidence.
2. **Intentional vs incidental differences** (Step 2) — Whether a variation between instances is
   deliberate (keep both forms) or incidental (one is the "correct" form).
3. **Variable classification** (Step 3) — What becomes a template variable vs `ai:customize` marker
   vs hardcoded value. This is a design decision the user must make.
4. **Code anomalies** — Any code that looks inconsistent, outdated, or "wrong" compared to other
   instances. **Do not assume it's a mistake.** It may be intentional for business, historical,
   or compatibility reasons you cannot see.
5. **Injection points** (Step 4) — Which registration sites to add markers to. Adding markers
   modifies existing files — the user must approve each one.

### How to escalate

Every AskUserQuestion MUST:
1. Present 2-3 concrete **lettered options** (A, B, C)
2. State your **recommendation first** — "We recommend B: ..."
3. Explain **why** in 1-2 sentences
4. **One decision per question** — never batch multiple decisions

### What NOT to do

- **Do not "clean up" patterns.** If every existing module has a trailing comma after the last field,
  the template should too.
- **Do not judge code quality.** If error handling is inconsistent across instances, present the
  inconsistency and ask — don't pick the "better" one.
- **Do not add improvements.** If existing modules lack validation, the generator should not add it.
  That's a separate engineering decision.
- **Do not skip escalation because "it's obvious."** What's obvious to an LLM reviewing code may
  not match the team's historical context or business constraints.

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

**STOP.** Present each candidate pattern group to the user via AskUserQuestion:
- Show the file structure and instance count
- Recommend whether to proceed with extraction
- Options: **A)** Extract this pattern **B)** Skip — not a real pattern **C)** Merge with another group
- If there are multiple candidate groups, present each individually — one question per group

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

**STOP.** For each difference found between instances, escalate individually:
- Show the exact diff with file and line references
- Options: **A)** Intentional variation — parameterize it (propose variable name + type)
  **B)** Incidental — use instance X as canonical **C)** Unclear — need more context from user
- **Never assume a difference is a mistake.** If `orders/service.rs` handles errors differently
  from `products/service.rs`, present both forms neutrally and ask which to template.

### Step 3: Classify variables

For each variable part found in the diff:

| What changes | Variable type | Input type |
|---|---|---|
| Module/entity name (singular/plural/PascalCase) | All derived from one input | `string` → use filters |
| List of struct fields with types | Iterable list | `field[]` |
| Sections present in some instances but not others | Boolean toggle | `bool` |
| A value chosen from a fixed set | Selection | `string` with description |

**STOP.** Present the full variable classification table to the user via AskUserQuestion.
For each variable, confirm:
- Options: **A)** Template variable (user provides at generation time) **B)** `ai:customize` marker
  (agent fills in after generation) **C)** Hardcode to a specific value
- Pay special attention to domain logic sections — these almost always should be `ai:customize`,
  not template variables. But **ask, don't assume**.

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

**STOP.** Injection points modify existing files. For each proposed injection site, escalate:
- Show the file, line, and surrounding context
- Options: **A)** Add marker here **B)** Different location (suggest alternative) **C)** Skip — not needed
- If a file has multiple potential injection sites (e.g., imports AND route registration), present each separately.

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

## Required Output — Decisions Summary

At the end of the workflow, display:

```
Pattern Analysis Complete
- Candidate groups found: ___
- Patterns accepted: ___
- Variables classified: ___ (template vars: ___, ai:customize: ___, hardcoded: ___)
- Injection points added: ___
- Anomalies escalated: ___ (resolved: ___, deferred: ___)
- Unresolved decisions: ___ (list below if any)
```

If any decisions were skipped or left unresolved, list them as
**"Unresolved decisions that may affect the generator"** with enough context to revisit later.

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
