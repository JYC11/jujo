# Examples

Each directory is a self-contained jujo project with templates and **real generated output** produced by running the commands below.

| Example | Language | Comment Style | Command |
|---------|----------|---------------|---------|
| `rust-entity` | Rust | `//` | `jujo generate entity --var entity_name=order --var "fields=title:string,amount:decimal?,shipped:bool"` |
| `go-service` | Go | `//` | `jujo generate service --var service_name=order --var "fields=name:string,price:decimal,active:bool"` |
| `python-model` | Python | `#` | `jujo generate model --var model_name=order --var "fields=title:string,amount:decimal?,created_at:datetime"` |
| `typescript-component` | TypeScript | `//` | `jujo generate component --var component_name=order --var "fields=title:string,price:decimal,active:bool"` |
| `java-entity` | Java | `//` | `jujo generate entity --var entity_name=order --var "fields=title:string,price:decimal,active:bool"` |
| `html-page` | HTML | `<!-- -->` | `jujo generate page --var page_name=about --var title=About` |

## What to look at

Each example contains:

- `.jujo/config.toml` — language-specific type map and comment style
- `.jujo/templates/<name>/generator.toml` — input schema and actions
- `.jujo/templates/<name>/*.tera` — Tera templates with type mapping, casing filters, and `<ai:customize>` markers
- Generated output files (the actual code jujo produced)

The `rust-entity` and `html-page` examples also demonstrate **marker injection** — inserting generated content into existing files at `// </jujo:marker>` or `<!-- </jujo:marker> -->` points.

## Try it yourself

```bash
# Pick an example
cd examples/rust-entity

# See what generators are available
jujo list

# Preview what would be generated (no files written)
jujo generate entity --var entity_name=product --var "fields=name:string,price:decimal" --dry-run

# Generate (will error because order.rs already exists — use --force to overwrite)
jujo generate entity --var entity_name=product --var "fields=name:string,price:decimal"
```
