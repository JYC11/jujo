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
    let mod_content = std::fs::read_to_string(dir.path().join("src/orders/mod.rs")).unwrap();
    assert_eq!(mod_content.trim(), "pub mod routes;");

    let routes_content = std::fs::read_to_string(dir.path().join("src/orders/routes.rs")).unwrap();
    assert!(routes_content.contains("list_orders"), "singularize failed");
    assert!(routes_content.contains("Vec<Order>"), "pascal_case failed");
    assert!(
        routes_content.contains("// hello"),
        "default value not applied"
    );
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
        .args([
            "generate",
            "example",
            "--var",
            "module_name=orders",
            "--force",
        ])
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
    assert!(
        content.contains("pub struct Order"),
        "EntityName not rendered"
    );
    assert!(
        content.contains("pub title: String"),
        "string field missing"
    );
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
        .args([
            "generate",
            "module",
            "--var",
            "module_name=orders",
            "--force",
        ])
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
            "generate",
            "module",
            "--var",
            "module_name=orders",
            "--force",
            "--skip-existing",
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
        .args([
            "generate",
            "module",
            "--var",
            "module_name=orders",
            "--dry-run",
        ])
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
        .args([
            "generate",
            "module",
            "--var",
            "module_name=orders",
            "--json",
        ])
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
            "generate",
            "module",
            "--var",
            "module_name=orders",
            "--dry-run",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["generator"], "module");
    assert!(!json["created"].as_array().unwrap().is_empty());
    assert!(!json["injected"].as_array().unwrap().is_empty());

    // No files written.
    assert!(!dir.path().join("src/orders/mod.rs").exists());
}

// --- Phase 4: Discovery commands ---

#[test]
fn init_creates_jujo_dir() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path())
        .env("NO_COLOR", "1")
        .args(["init", "--lang", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized .jujo/"))
        .stdout(predicate::str::contains("rust"));

    assert!(dir.path().join(".jujo/config.toml").exists());
    assert!(
        dir.path()
            .join(".jujo/templates/example/generator.toml")
            .exists()
    );

    // Config has Rust type map.
    let config = std::fs::read_to_string(dir.path().join(".jujo/config.toml")).unwrap();
    assert!(config.contains("string = \"String\""));
    assert!(config.contains("int = \"i64\""));
}

#[test]
fn init_go_language() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path())
        .env("NO_COLOR", "1")
        .args(["init", "--lang", "go"])
        .assert()
        .success();

    let config = std::fs::read_to_string(dir.path().join(".jujo/config.toml")).unwrap();
    assert!(config.contains("string = \"string\""));
    assert!(config.contains("int = \"int64\""));
}

#[test]
fn init_already_exists() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();

    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path())
        .env("NO_COLOR", "1")
        .args(["init", "--lang", "rust"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_unknown_language() {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path())
        .env("NO_COLOR", "1")
        .args(["init", "--lang", "brainfuck"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown language"));
}

#[test]
fn init_creates_languages_toml() {
    let dir = TempDir::new().unwrap();

    jujo_cmd(&dir)
        .args(["init", "--lang", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("languages.toml"));

    let langs_file = dir.path().join(".jujo/languages.toml");
    assert!(langs_file.exists());

    let content = std::fs::read_to_string(&langs_file).unwrap();
    // Should contain all built-in languages.
    assert!(content.contains("[rust]"));
    assert!(content.contains("[go]"));
    assert!(content.contains("[python]"));
    assert!(content.contains("[typescript]"));
    assert!(content.contains("[css]"));
    assert!(content.contains("[csharp]"));
    assert!(content.contains("[html]"));
    assert!(content.contains("[kotlin]"));
    assert!(content.contains("[ruby]"));
    assert!(content.contains("[swift]"));
    assert!(content.contains("[elixir]"));
    assert!(content.contains("[php]"));
}

#[test]
fn init_new_languages() {
    // Test each new language produces a valid config.
    for lang in [
        "css", "csharp", "html", "kotlin", "ruby", "php", "swift", "elixir",
    ] {
        let dir = TempDir::new().unwrap();
        jujo_cmd(&dir)
            .args(["init", "--lang", lang])
            .assert()
            .success()
            .stdout(predicate::str::contains(lang));

        let config = std::fs::read_to_string(dir.path().join(".jujo/config.toml")).unwrap();
        assert!(config.contains("[type_map]"), "no type_map for {lang}");
        assert!(config.contains("string ="), "no string type for {lang}");
        assert!(config.contains("int ="), "no int type for {lang}");
    }
}

#[test]
fn init_user_preset_from_global_file() {
    let dir = TempDir::new().unwrap();

    // Create a fake home dir with ~/.jujo/languages.toml.
    let fake_home = TempDir::new().unwrap();
    let jujo_home = fake_home.path().join(".jujo");
    std::fs::create_dir_all(&jujo_home).unwrap();
    std::fs::write(
        jujo_home.join("languages.toml"),
        r#"[zig]
comment_prefix = "//"
[zig.type_map]
string = "[]const u8"
text = "[]const u8"
int = "i64"
bool = "bool"
float = "f64"
decimal = "f128"
uuid = "[]const u8"
date = "i64"
datetime = "i64"
json = "std.json.Value"
"#,
    )
    .unwrap();

    jujo_cmd(&dir)
        .env("HOME", fake_home.path())
        .args(["init", "--lang", "zig"])
        .assert()
        .success()
        .stdout(predicate::str::contains("zig"));

    let config = std::fs::read_to_string(dir.path().join(".jujo/config.toml")).unwrap();
    assert!(config.contains(r#"string = "[]const u8""#));
    assert!(config.contains(r#"int = "i64""#));

    // languages.toml should include both built-in and user-defined.
    let langs = std::fs::read_to_string(dir.path().join(".jujo/languages.toml")).unwrap();
    assert!(langs.contains("[zig]"));
    assert!(langs.contains("[rust]"));
}

#[test]
fn list_generators() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);
    seed_inject_generator(&dir);

    jujo_cmd(&dir)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("example"))
        .stdout(predicate::str::contains("module"));
}

#[test]
fn list_generators_json() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    let output = jujo_cmd(&dir)
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr.iter().any(|g| g["name"] == "example"));
}

#[test]
fn describe_generator_json() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    let output = jujo_cmd(&dir)
        .args(["describe", "example", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["name"], "example");
    assert!(!json["inputs"].as_array().unwrap().is_empty());
    assert!(!json["actions"].as_array().unwrap().is_empty());
    assert_eq!(json["inputs"][0]["name"], "module_name");
    assert_eq!(json["inputs"][0]["type"], "string");
}

#[test]
fn validate_all_ok() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    jujo_cmd(&dir)
        .args(["validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("All OK"));
}

#[test]
fn validate_catches_bad_template() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Add a generator with a broken template.
    let bad_dir = dir.path().join(".jujo/templates/broken");
    std::fs::create_dir_all(&bad_dir).unwrap();
    std::fs::write(
        bad_dir.join("generator.toml"),
        "[generator]\nname = \"broken\"\ndescription = \"bad\"\n\n[[actions]]\ntype = \"create\"\ntemplate = \"bad.tera\"\noutput = \"out.txt\"\n",
    ).unwrap();
    std::fs::write(bad_dir.join("bad.tera"), "{{ unclosed").unwrap();

    jujo_cmd(&dir)
        .args(["validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("error"));
}

#[test]
fn validate_catches_missing_template() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Add a generator referencing a nonexistent template.
    let bad_dir = dir.path().join(".jujo/templates/missing");
    std::fs::create_dir_all(&bad_dir).unwrap();
    std::fs::write(
        bad_dir.join("generator.toml"),
        "[generator]\nname = \"missing\"\ndescription = \"bad\"\n\n[[actions]]\ntype = \"create\"\ntemplate = \"nonexistent.tera\"\noutput = \"out.txt\"\n",
    ).unwrap();

    jujo_cmd(&dir)
        .args(["validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("not found"));
}

#[test]
fn init_then_validate_then_generate() {
    let dir = TempDir::new().unwrap();

    // Init.
    let mut cmd = Command::cargo_bin("jujo").unwrap();
    cmd.current_dir(dir.path())
        .env("NO_COLOR", "1")
        .args(["init", "--lang", "rust"])
        .assert()
        .success();

    // Validate.
    jujo_cmd(&dir).args(["validate"]).assert().success();

    // Generate.
    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "name=world"])
        .assert()
        .success();

    assert!(dir.path().join("world.txt").exists());
    let content = std::fs::read_to_string(dir.path().join("world.txt")).unwrap();
    assert!(content.contains("World"));
}

#[test]
fn full_agent_protocol() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Phase 1: Discover.
    let list_out = jujo_cmd(&dir)
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let generators: serde_json::Value = serde_json::from_slice(&list_out).unwrap();
    let gen_name = generators[0]["name"].as_str().unwrap();

    // Phase 2: Schema.
    let describe_out = jujo_cmd(&dir)
        .args(["describe", gen_name, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let schema: serde_json::Value = serde_json::from_slice(&describe_out).unwrap();
    assert!(!schema["inputs"].as_array().unwrap().is_empty());

    // Phase 3: Preview.
    let preview_out = jujo_cmd(&dir)
        .args([
            "generate",
            gen_name,
            "--var",
            "module_name=test",
            "--dry-run",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let preview: serde_json::Value = serde_json::from_slice(&preview_out).unwrap();
    assert!(!preview["created"].as_array().unwrap().is_empty());

    // Phase 4: Execute.
    let exec_out = jujo_cmd(&dir)
        .args(["generate", gen_name, "--var", "module_name=test", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let result: serde_json::Value = serde_json::from_slice(&exec_out).unwrap();
    assert!(!result["created"].as_array().unwrap().is_empty());
}

// --- Phase 5: AI markers + template management ---

fn seed_ai_marker_generator(dir: &TempDir) {
    write_config(
        dir,
        "comment_prefix = \"//\"\ncomment_suffix = \"\"\n\n[type_map]\nstring = \"String\"\n",
    );

    let gen_dir = dir.path().join(".jujo/templates/service");
    std::fs::create_dir_all(&gen_dir).unwrap();

    std::fs::write(
        gen_dir.join("generator.toml"),
        r#"
[generator]
name = "service"
description = "Service with AI markers"

[[inputs]]
name = "name"
type = "string"
required = true

[[actions]]
type = "create"
template = "service.tera"
output = "src/{{ name }}_service.rs"
"#,
    )
    .unwrap();

    std::fs::write(
        gen_dir.join("service.tera"),
        r#"pub fn create(req: CreateReq) -> Result<Response> {
    // <ai:customize hint="Add validation logic">
    let validated = req;
    // </ai:customize>
    save(validated)
}

pub fn list() -> Vec<Response> {
    // <ai:customize hint="Add pagination and filtering">
    todo!()
    // </ai:customize>
}
"#,
    )
    .unwrap();
}

#[test]
fn ai_markers_in_manifest() {
    let dir = TempDir::new().unwrap();
    seed_ai_marker_generator(&dir);

    let output = jujo_cmd(&dir)
        .args(["generate", "service", "--var", "name=order", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let customize = json["customize"].as_array().unwrap();
    assert_eq!(customize.len(), 2);
    assert_eq!(customize[0]["path"], "src/order_service.rs");
    assert_eq!(customize[0]["hint"], "Add validation logic");
    assert_eq!(customize[1]["hint"], "Add pagination and filtering");
    // Line numbers should be positive integers.
    assert!(customize[0]["line"].as_u64().unwrap() > 0);
    assert!(customize[1]["line"].as_u64().unwrap() > customize[0]["line"].as_u64().unwrap());
}

#[test]
fn ai_markers_in_dry_run() {
    let dir = TempDir::new().unwrap();
    seed_ai_marker_generator(&dir);

    let output = jujo_cmd(&dir)
        .args([
            "generate",
            "service",
            "--var",
            "name=order",
            "--dry-run",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(!json["customize"].as_array().unwrap().is_empty());
}

#[test]
fn template_add_and_remove() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join(".jujo/templates")).unwrap();

    // Create a source template.
    let src = dir.path().join("my-source");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("generator.toml"),
        "[generator]\nname = \"imported\"\ndescription = \"An imported generator\"\n",
    )
    .unwrap();
    std::fs::write(src.join("hello.tera"), "Hello!").unwrap();

    // Add it.
    jujo_cmd(&dir)
        .args([
            "template",
            "add",
            "imported",
            "--from",
            src.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added template"));

    assert!(
        dir.path()
            .join(".jujo/templates/imported/generator.toml")
            .exists()
    );
    assert!(
        dir.path()
            .join(".jujo/templates/imported/hello.tera")
            .exists()
    );

    // Shows up in list.
    jujo_cmd(&dir)
        .args(["template", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("imported"));

    // Remove it.
    jujo_cmd(&dir)
        .args(["template", "remove", "imported"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed template"));

    assert!(!dir.path().join(".jujo/templates/imported").exists());
}

#[test]
fn template_add_already_exists() {
    let dir = TempDir::new().unwrap();
    let dest = dir.path().join(".jujo/templates/existing");
    std::fs::create_dir_all(&dest).unwrap();

    let src = dir.path().join("src-gen");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("generator.toml"),
        "[generator]\nname = \"x\"\ndescription = \"x\"\n",
    )
    .unwrap();

    jujo_cmd(&dir)
        .args([
            "template",
            "add",
            "existing",
            "--from",
            src.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// --- Post-generate hook tests ---

#[test]
fn hook_runs_on_created_files() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Config with a hook that creates a .formatted marker file.
    write_config(
        &dir,
        "comment_prefix = \"//\"\ncomment_suffix = \"\"\n\n[hooks]\npost_generate = \"touch {file}.formatted\"\n\n[type_map]\nstring = \"String\"\n",
    );

    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=orders"])
        .assert()
        .success();

    // The hook should have created .formatted marker files.
    assert!(
        dir.path().join("src/orders/mod.rs.formatted").exists(),
        "hook did not run on mod.rs"
    );
    assert!(
        dir.path().join("src/orders/routes.rs.formatted").exists(),
        "hook did not run on routes.rs"
    );
}

#[test]
fn hook_failure_does_not_crash() {
    let dir = TempDir::new().unwrap();
    seed_jujo(&dir);

    // Hook that always fails.
    write_config(
        &dir,
        "comment_prefix = \"//\"\ncomment_suffix = \"\"\n\n[hooks]\npost_generate = \"false\"\n\n[type_map]\nstring = \"String\"\n",
    );

    // Generate should still succeed even though hook fails.
    jujo_cmd(&dir)
        .args(["generate", "example", "--var", "module_name=orders"])
        .assert()
        .success();

    // Files should still be created.
    assert!(dir.path().join("src/orders/mod.rs").exists());
}

// --- HTML/CSS comment style injection tests ---

#[test]
fn html_comment_style_injection() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        "comment_prefix = \"<!--\"\ncomment_suffix = \"-->\"\n\n[type_map]\nstring = \"string\"\n",
    );

    let gen_dir = dir.path().join(".jujo/templates/component");
    std::fs::create_dir_all(&gen_dir).unwrap();

    std::fs::write(
        gen_dir.join("generator.toml"),
        r#"
[generator]
name = "component"
description = "HTML component"

[[inputs]]
name = "name"
type = "string"
required = true

[[actions]]
type = "create"
template = "component.tera"
output = "components/{{ name }}.html"

[[actions]]
type = "inject"
target = "index.html"
marker = "components"
content = "<link rel=\"import\" href=\"components/{{ name }}.html\">"
"#,
    )
    .unwrap();

    std::fs::write(
        gen_dir.join("component.tera"),
        "<div class=\"{{ name }}\">{{ name | pascal_case }}</div>\n",
    )
    .unwrap();

    // Create target file with HTML-style marker.
    std::fs::write(
        dir.path().join("index.html"),
        "<head>\n<!-- </jujo:components> -->\n</head>\n",
    )
    .unwrap();

    jujo_cmd(&dir)
        .args(["generate", "component", "--var", "name=sidebar"])
        .assert()
        .success();

    let index = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
    assert!(
        index.contains(
            "<link rel=\"import\" href=\"components/sidebar.html\">\n<!-- </jujo:components> -->"
        ),
        "HTML marker injection failed: {index}"
    );

    let component = std::fs::read_to_string(dir.path().join("components/sidebar.html")).unwrap();
    assert!(component.contains("Sidebar"));
}

#[test]
fn css_comment_style_injection() {
    let dir = TempDir::new().unwrap();
    write_config(
        &dir,
        "comment_prefix = \"/*\"\ncomment_suffix = \"*/\"\n\n[type_map]\nstring = \"string\"\n",
    );

    let gen_dir = dir.path().join(".jujo/templates/theme");
    std::fs::create_dir_all(&gen_dir).unwrap();

    std::fs::write(
        gen_dir.join("generator.toml"),
        r#"
[generator]
name = "theme"
description = "CSS theme variables"

[[inputs]]
name = "name"
type = "string"
required = true

[[actions]]
type = "inject"
target = "styles.css"
marker = "themes"
content = "@import 'themes/{{ name }}.css';"
"#,
    )
    .unwrap();

    // Create target file with CSS-style marker.
    std::fs::write(
        dir.path().join("styles.css"),
        "/* Base styles */\n/* </jujo:themes> */\n",
    )
    .unwrap();

    jujo_cmd(&dir)
        .args(["generate", "theme", "--var", "name=dark"])
        .assert()
        .success();

    let css = std::fs::read_to_string(dir.path().join("styles.css")).unwrap();
    assert!(
        css.contains("@import 'themes/dark.css';\n/* </jujo:themes> */"),
        "CSS marker injection failed: {css}"
    );
}
