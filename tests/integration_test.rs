use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn jujo_cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path());
    cmd
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
