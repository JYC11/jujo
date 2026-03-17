use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use pluralizer::pluralize;
use std::collections::HashMap;
use tera::{Result, Tera, Value};

/// Register all custom Tera filters.
pub fn register_filters(tera: &mut Tera) {
    tera.register_filter("pascal_case", filter_pascal_case);
    tera.register_filter("snake_case", filter_snake_case);
    tera.register_filter("camel_case", filter_camel_case);
    tera.register_filter("kebab_case", filter_kebab_case);
    tera.register_filter("upper_case", filter_upper_case);
    tera.register_filter("singularize", filter_singularize);
    tera.register_filter("pluralize", filter_pluralize);
}

fn filter_pascal_case(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let s = tera::try_get_value!("pascal_case", "value", String, value);
    Ok(Value::String(s.to_pascal_case()))
}

fn filter_snake_case(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let s = tera::try_get_value!("snake_case", "value", String, value);
    Ok(Value::String(s.to_snake_case()))
}

fn filter_camel_case(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let s = tera::try_get_value!("camel_case", "value", String, value);
    Ok(Value::String(s.to_lower_camel_case()))
}

fn filter_kebab_case(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let s = tera::try_get_value!("kebab_case", "value", String, value);
    Ok(Value::String(s.to_kebab_case()))
}

fn filter_upper_case(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let s = tera::try_get_value!("upper_case", "value", String, value);
    Ok(Value::String(s.to_uppercase()))
}

fn filter_singularize(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let s = tera::try_get_value!("singularize", "value", String, value);
    Ok(Value::String(pluralize(&s, 1, false)))
}

fn filter_pluralize(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let s = tera::try_get_value!("pluralize", "value", String, value);
    Ok(Value::String(pluralize(&s, 2, false)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_filter(
        filter: fn(&Value, &HashMap<String, Value>) -> Result<Value>,
        input: &str,
    ) -> String {
        let val = Value::String(input.to_string());
        let args = HashMap::new();
        match filter(&val, &args).unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(apply_filter(filter_pascal_case, "blog_posts"), "BlogPosts");
        assert_eq!(apply_filter(filter_pascal_case, "tenant"), "Tenant");
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(apply_filter(filter_snake_case, "BlogPosts"), "blog_posts");
    }

    #[test]
    fn test_camel_case() {
        assert_eq!(apply_filter(filter_camel_case, "blog_posts"), "blogPosts");
    }

    #[test]
    fn test_kebab_case() {
        assert_eq!(apply_filter(filter_kebab_case, "blog_posts"), "blog-posts");
    }

    #[test]
    fn test_upper_case() {
        assert_eq!(apply_filter(filter_upper_case, "tenant"), "TENANT");
    }

    #[test]
    fn test_singularize() {
        assert_eq!(apply_filter(filter_singularize, "tenants"), "tenant");
        assert_eq!(apply_filter(filter_singularize, "categories"), "category");
        assert_eq!(apply_filter(filter_singularize, "addresses"), "address");
        assert_eq!(apply_filter(filter_singularize, "people"), "person");
    }

    #[test]
    fn test_pluralize() {
        assert_eq!(apply_filter(filter_pluralize, "tenant"), "tenants");
        assert_eq!(apply_filter(filter_pluralize, "category"), "categories");
        assert_eq!(apply_filter(filter_pluralize, "person"), "people");
    }
}
