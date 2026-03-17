use crate::filters;
use anyhow::{Context, Result};
use std::path::Path;
use tera::Tera;

const VARS_FILE: &str = "_vars.tera";

/// Create a Tera instance loaded with templates from a generator directory.
/// If `_vars.tera` exists, its content is prepended to every other template
/// so that `{% set %}` variables are available in the same scope.
pub fn create_tera(gen_dir: &Path) -> Result<Tera> {
    let vars_path = gen_dir.join(VARS_FILE);
    let vars_prefix = if vars_path.exists() {
        std::fs::read_to_string(&vars_path)
            .with_context(|| format!("failed to read {}", vars_path.display()))?
    } else {
        String::new()
    };

    let mut tera = Tera::default();

    // Walk the directory and register each .tera file.
    for entry in std::fs::read_dir(gen_dir)
        .with_context(|| format!("failed to read directory {}", gen_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "tera") {
            let file_name = path
                .file_name()
                .unwrap()
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("non-UTF8 filename: {}", path.display()))?
                .to_string();

            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;

            // Prepend _vars.tera to non-underscore templates.
            let full_content = if file_name != VARS_FILE && !vars_prefix.is_empty() {
                format!("{vars_prefix}\n{content}")
            } else {
                content
            };

            tera.add_raw_template(&file_name, &full_content)
                .with_context(|| format!("failed to parse template \"{}\"", file_name))?;
        }
    }

    filters::register_filters(&mut tera);
    Ok(tera)
}

/// Render a named template with the given context.
pub fn render_template(
    tera: &Tera,
    template_name: &str,
    ctx: &tera::Context,
) -> Result<String> {
    tera.render(template_name, ctx).with_context(|| {
        format!("failed to render template \"{template_name}\"")
    })
}

/// Render a Tera expression string (e.g., an output path) with the given context.
pub fn render_expression(
    tera: &Tera,
    expr: &str,
    ctx: &tera::Context,
) -> Result<String> {
    // Use Tera's one-off rendering for inline expressions.
    let mut one_off = tera.clone();
    one_off
        .add_raw_template("__expr__", expr)
        .with_context(|| format!("invalid expression: \"{expr}\""))?;
    one_off
        .render("__expr__", ctx)
        .with_context(|| format!("failed to render expression: \"{expr}\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn render_simple_template() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.tera"), "Hello, {{ name }}!").unwrap();
        let tera = create_tera(dir.path()).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("name", "world");
        let output = render_template(&tera, "hello.tera", &ctx).unwrap();
        assert_eq!(output, "Hello, world!");
    }

    #[test]
    fn render_with_filters() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("test.tera"),
            "{{ name | pascal_case }} {{ name | singularize }}",
        )
        .unwrap();
        let tera = create_tera(dir.path()).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("name", "blog_posts");
        let output = render_template(&tera, "test.tera", &ctx).unwrap();
        assert_eq!(output, "BlogPosts blog_post");
    }

    #[test]
    fn render_with_vars_prepended() {
        let dir = TempDir::new().unwrap();
        // _vars.tera is automatically prepended to all other templates.
        std::fs::write(
            dir.path().join("_vars.tera"),
            "{% set entity = module_name | singularize %}",
        )
        .unwrap();
        std::fs::write(dir.path().join("main.tera"), "Entity: {{ entity }}").unwrap();
        let tera = create_tera(dir.path()).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("module_name", "orders");
        let output = render_template(&tera, "main.tera", &ctx).unwrap();
        assert_eq!(output.trim(), "Entity: order");
    }

    #[test]
    fn render_expression_with_vars() {
        let dir = TempDir::new().unwrap();
        // Need at least one .tera file for Tera to initialize.
        std::fs::write(dir.path().join("dummy.tera"), "").unwrap();
        let tera = create_tera(dir.path()).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("module_name", "orders");
        let output = render_expression(&tera, "src/{{ module_name }}/mod.rs", &ctx).unwrap();
        assert_eq!(output, "src/orders/mod.rs");
    }

    #[test]
    fn render_with_chained_vars() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("_vars.tera"),
            "{% set entity_name = module_name | singularize %}\n{% set EntityName = entity_name | pascal_case %}",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("routes.tera"),
            "fn list_{{ entity_name }}s() -> {{ EntityName }} {}",
        )
        .unwrap();
        let tera = create_tera(dir.path()).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("module_name", "orders");
        let output = render_template(&tera, "routes.tera", &ctx).unwrap();
        assert!(
            output.contains("list_orders"),
            "expected 'list_orders' in output: {output}"
        );
        assert!(
            output.contains("Order"),
            "expected 'Order' in output: {output}"
        );
    }

    #[test]
    fn template_syntax_error() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bad.tera"), "{{ unclosed").unwrap();
        let err = create_tera(dir.path()).unwrap_err();
        assert!(err.to_string().contains("failed to parse template"));
    }
}
