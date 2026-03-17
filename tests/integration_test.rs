use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn jujo_cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path());
    cmd.env("NO_COLOR", "1");
    cmd
}

fn write_config(dir: &TempDir, content: &str) {
    std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();
    std::fs::write(dir.path().join(".jujo/config.toml"), content).unwrap();
}

fn seed_jujo(dir: &TempDir) {
    let gen_dir = dir.path().join(".jujo/templates/example");
    std::fs::create_dir_all(&gen_dir).unwrap();

    std::fs::write(
        gen_dir.join("generator.toml"),
        r#"
[generator]
name = "example"
description = "Example generator"

[[inputs]]
name = "module_name"
type = "string"
description = "Module name"
required = true

[[inputs]]
name = "greeting"
type = "string"
required = false
default = "hello"

[[actions]]
type = "create"
template = "mod.tera"
output = "src/{{ module_name }}/mod.rs"

[[actions]]
type = "create"
template = "routes.tera"
output = "src/{{ module_name }}/routes.rs"
"#,
    )
    .unwrap();

    std::fs::write(
        gen_dir.join("_vars.tera"),
        "{% set entity_name = module_name | singularize %}\n{% set EntityName = entity_name | pascal_case %}",
    )
    .unwrap();

    std::fs::write(gen_dir.join("mod.tera"), "pub mod routes;\n").unwrap();

    std::fs::write(
        gen_dir.join("routes.tera"),
        "// {{ greeting }}\npub fn list_{{ entity_name }}s() -> Vec<{{ EntityName }}> {\n    todo!()\n}\n",
    )
    .unwrap();
}

#[test]
fn generate_creates_files() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=orders"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create src/orders/mod.rs"))
        .stdout(predicate::str::contains("create src/orders/routes.rs"))
        .stdout(predicate::str::contains("manifest"));

    // Files exist with correct content.
    let mod_content =
        std::fs::read_to_string(dir.path().join("src/orders/mod.rs")).unwrap();
    assert_eq!(mod_content.trim(), "pub mod routes;");

    let routes_content =
        std::fs::read_to_string(dir.path().join("src/orders/routes.rs")).unwrap();
    assert!(routes_content.contains("list_orders"), "singularize failed");
    assert!(routes_content.contains("Vec<Order>"), "pascal_case failed");
    assert!(routes_content.contains("// hello"), "default value not applied");
}

#[test]
fn generate_writes_manifest() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=tenants"])
        .assert()
        .success();

    let manifest_path = dir.path().join(".jujo/last-generate.json");
    assert!(manifest_path.exists(), "manifest not written");

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["generator"], "example");
    assert_eq!(manifest["inputs"]["module_name"], "tenants");
    assert_eq!(manifest["created"].as_array().unwrap().len(), 2);
    assert_eq!(manifest["created"][0]["path"], "src/tenants/mod.rs");
}

#[test]
fn generate_unknown_generator() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".jujo/templates")).unwrap();

    jujo_cmd(&dir)
        .args(["generate", "nonexistent", "--var", "x=y"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown generator"));
}

#[test]
fn generate_missing_required_input() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    jujo_cmd(&dir)
        .args(["generate", "example"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required input"))
        .stderr(predicate::str::contains("--var module_name="));
}

#[test]
fn generate_file_exists_no_force() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Run once.
    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=orders"])
        .assert()
        .success();

    // Run again without --force.
    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=orders"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"))
        .stderr(predicate::str::contains("--force"));
}

#[test]
fn generate_file_exists_with_force() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Run once.
    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=orders"])
        .assert()
        .success();

    // Run again with --force.
    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=orders", "--force"])
        .assert()
        .success();
}

#[test]
fn generate_no_jujo_dir() {
    let dir = TempDir::new().unwrap();
    // No .jujo/ directory.

    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "x=y"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no .jujo/ directory found"))
        .stderr(predicate::str::contains("jujo init"));
}

#[test]
fn generate_from_subdirectory() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Create a subdirectory and run from there.
    let sub = dir.path().join("src/deep/nested");
    std::fs::create_dir_all(&sub).unwrap();

    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(&sub)
        .args(["generate", "example", "--var", "module_name=widgets"])
        .assert()
        .success();

    // Files created relative to the jujo root, not the cwd.
    assert!(dir.path().join("src/widgets/mod.rs").exists());
}

// --- Phase 2: Field parsing + type map tests ---

fn seed_entity_generator(dir: &TempDir) {
    write_config(
        dir,
        "comment_prefix = \"//\"\ncomment_suffix = \"\"\n\n[type_map]\nstring = \"String\"\nint = \"i64\"\nbool = \"bool\"\ndecimal = \"rust_decimal::Decimal\"\n",
    );

    let gen_dir = dir.path().join(".jujo/templates/entity");
    std::fs::create_dir_all(&gen_dir).unwrap();

    std::fs::write(
        gen_dir.join("generator.toml"),
        r#"
[generator]
name = "entity"
description = "Generate an entity struct from fields"

[[inputs]]
name = "entity_name"
type = "string"
required = true

[[inputs]]
name = "fields"
type = "field[]"
description = "Fields as name:type"
required = true

[[actions]]
type = "create"
template = "entity.tera"
output = "src/{{ entity_name }}.rs"
"#,
    )
    .unwrap();

    std::fs::write(
        gen_dir.join("entity.tera"),
        r#"{% set EntityName = entity_name | pascal_case %}
pub struct {{ EntityName }} {
    pub id: String,
{% for field in fields %}
{% if field.nullable %}    pub {{ field.name }}: Option<{{ field.mapped_type }}>,
{% else %}    pub {{ field.name }}: {{ field.mapped_type }},
{% endif %}
{% endfor %}
}
"#,
    )
    .unwrap();
}

#[test]
fn generate_with_fields_comma_separated() {
    let dir = TempDir::new().unwrap();
    seed_entity_generator(&dir);

    jujo_cmd(&dir)
        .args([
            "generate",
            "entity",
            "--var",
            "entity_name=order",
            "--var",
            "fields=title:string,price:decimal?,active:bool",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("create src/order.rs"));

    let content = std::fs::read_to_string(dir.path().join("src/order.rs")).unwrap();
    assert!(content.contains("pub struct Order"), "EntityName not rendered");
    assert!(content.contains("pub title: String"), "string field missing");
    assert!(
        content.contains("Option<rust_decimal::Decimal>"),
        "nullable decimal not rendered"
    );
    assert!(content.contains("pub active: bool"), "bool field missing");
}

#[test]
fn generate_with_fields_repeated_var() {
    let dir = TempDir::new().unwrap();
    seed_entity_generator(&dir);

    jujo_cmd(&dir)
        .args([
            "generate",
            "entity",
            "--var",
            "entity_name=product",
            "--var",
            "fields=name:string",
            "--var",
            "fields=price:decimal?",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(dir.path().join("src/product.rs")).unwrap();
    assert!(content.contains("pub struct Product"));
    assert!(content.contains("pub name: String"));
    assert!(content.contains("Option<rust_decimal::Decimal>"));
}

#[test]
fn generate_fields_unknown_type() {
    let dir = TempDir::new().unwrap();
    seed_entity_generator(&dir);

    jujo_cmd(&dir)
        .args([
            "generate",
            "entity",
            "--var",
            "entity_name=order",
            "--var",
            "fields=amount:money",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown type \"money\""))
        .stderr(predicate::str::contains("Valid types:"));
}

#[test]
fn generate_fields_no_type_map() {
    let dir = TempDir::new().unwrap();
    // Create generator but no config.toml (empty type map).
    let gen_dir = dir.path().join(".jujo/templates/entity");
    std::fs::create_dir_all(&gen_dir).unwrap();
    std::fs::write(
        gen_dir.join("generator.toml"),
        r#"
[generator]
name = "entity"
description = "test"

[[inputs]]
name = "fields"
type = "field[]"
required = true

[[actions]]
type = "create"
template = "entity.tera"
output = "entity.rs"
"#,
    )
    .unwrap();
    std::fs::write(gen_dir.join("entity.tera"), "placeholder").unwrap();

    jujo_cmd(&dir)
        .args(["generate", "entity", "--var", "fields=name:string"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no mapping in config.toml"));
}

// --- Phase 3: Injection + Dry-Run + JSON tests ---

fn seed_inject_generator(dir: &TempDir) {
    write_config(
        dir,
        "comment_prefix = \"//\"\ncomment_suffix = \"\"\n\n[type_map]\nstring = \"String\"\n",
    );

    let gen_dir = dir.path().join(".jujo/templates/module");
    std::fs::create_dir_all(&gen_dir).unwrap();

    std::fs::write(
        gen_dir.join("generator.toml"),
        r#"
[generator]
name = "module"
description = "Module with injection"

[[inputs]]
name = "module_name"
type = "string"
required = true

[[actions]]
type = "create"
template = "mod.tera"
output = "src/{{ module_name }}/mod.rs"

[[actions]]
type = "inject"
target = "src/main.rs"
marker = "modules"
content = "mod {{ module_name }};"

[[actions]]
type = "inject"
target = "src/main.rs"
marker = "routes"
content = ".nest(\"/{{ module_name }}\", {{ module_name }}::router())"
"#,
    )
    .unwrap();

    std::fs::write(gen_dir.join("mod.tera"), "pub mod routes;\n").unwrap();

    // Create the target file with markers.
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.rs"),
        "// </jujo:modules>\n\nfn router() -> Router {\n    Router::new()\n// </jujo:routes>\n}\n",
    )
    .unwrap();
}

#[test]
fn generate_with_inject() {
    let dir = TempDir::new().unwrap();
    seed_inject_generator(&dir);

    jujo_cmd(&dir)
        .args(["generate", "module", "--var", "module_name=orders"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("inject"));

    let main = std::fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
    assert!(main.contains("mod orders;\n// </jujo:modules>"));
    assert!(main.contains(".nest(\"/orders\", orders::router())\n// </jujo:routes>"));
}

#[test]
fn inject_conflict_default_error() {
    let dir = TempDir::new().unwrap();
    seed_inject_generator(&dir);

    // First run succeeds.
    jujo_cmd(&dir)
        .args(["generate", "module", "--var", "module_name=orders"])
        .assert()
        .success();

    // Second run: --force allows file overwrite but injection still conflicts.
    jujo_cmd(&dir)
        .args(["generate", "module", "--var", "module_name=orders", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("content already present"));
}

#[test]
fn inject_conflict_skip_existing() {
    let dir = TempDir::new().unwrap();
    seed_inject_generator(&dir);

    jujo_cmd(&dir)
        .args(["generate", "module", "--var", "module_name=orders"])
        .assert()
        .success();

    // Second run with --skip-existing + --force (force for file, skip for inject).
    jujo_cmd(&dir)
        .args([
            "generate", "module", "--var", "module_name=orders",
            "--force", "--skip-existing",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("skip"));

    // Content not duplicated.
    let main = std::fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
    assert_eq!(main.matches("mod orders;").count(), 1);
}

#[test]
fn dry_run_no_files_written() {
    let dir = TempDir::new().unwrap();
    seed_inject_generator(&dir);

    jujo_cmd(&dir)
        .args(["generate", "module", "--var", "module_name=orders", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("inject"));

    // No files created.
    assert!(!dir.path().join("src/orders/mod.rs").exists());
    // main.rs unchanged.
    let main = std::fs::read_to_string(dir.path().join("src/main.rs")).unwrap();
    assert!(!main.contains("mod orders;"));
    // No manifest written.
    assert!(!dir.path().join(".jujo/last-generate.json").exists());
}

#[test]
fn json_output_valid() {
    let dir = TempDir::new().unwrap();
    seed_inject_generator(&dir);

    let output = jujo_cmd(&dir)
        .args(["generate", "module", "--var", "module_name=orders", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["generator"], "module");
    assert_eq!(json["created"][0]["path"], "src/orders/mod.rs");
    assert_eq!(json["injected"][0]["marker"], "modules");
    assert_eq!(json["injected"][1]["marker"], "routes");
}

#[test]
fn dry_run_json_output() {
    let dir = TempDir::new().unwrap();
    seed_inject_generator(&dir);

    let output = jujo_cmd(&dir)
        .args([
            "generate", "module", "--var", "module_name=orders",
            "--dry-run", "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["generator"], "module");
    assert!(json["created"].as_array().unwrap().len() > 0);
    assert!(json["injected"].as_array().unwrap().len() > 0);

    // No files written.
    assert!(!dir.path().join("src/orders/mod.rs").exists());
}
