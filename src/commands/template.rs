use anyhow::{bail, Context, Result};
use std::path::Path;

/// Copy a template set from a local path into .jujo/templates/.
pub fn add(jujo_root: &Path, name: &str, from: &str, force: bool) -> Result<()> {
    let source = Path::new(from);
    if !source.is_dir() {
        bail!("source path \"{}\" is not a directory", from);
    }
    if !source.join("generator.toml").exists() {
        bail!(
            "source path \"{}\" does not contain a generator.toml",
            from
        );
    }

    let dest = jujo_root.join(".jujo/templates").join(name);
    if dest.exists() && !force {
        bail!(
            "template \"{}\" already exists. Use --force to overwrite.",
            name
        );
    }

    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .with_context(|| format!("failed to remove existing template \"{}\"", name))?;
    }

    copy_dir_recursive(source, &dest)?;

    println!("Added template \"{name}\" from {from}");
    Ok(())
}

/// Remove a template set from .jujo/templates/.
pub fn remove(jujo_root: &Path, name: &str) -> Result<()> {
    let dir = jujo_root.join(".jujo/templates").join(name);
    if !dir.exists() {
        bail!("template \"{name}\" not found");
    }

    std::fs::remove_dir_all(&dir)
        .with_context(|| format!("failed to remove template \"{name}\""))?;

    println!("Removed template \"{name}\"");
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_source(dir: &TempDir) -> std::path::PathBuf {
        let src = dir.path().join("source-gen");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("generator.toml"),
            "[generator]\nname = \"test\"\ndescription = \"test\"\n",
        )
        .unwrap();
        std::fs::write(src.join("hello.tera"), "Hello {{ name }}!").unwrap();
        src
    }

    #[test]
    fn add_template() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo/templates")).unwrap();
        let src = setup_source(&dir);

        add(dir.path(), "mygen", src.to_str().unwrap(), false).unwrap();

        assert!(dir.path().join(".jujo/templates/mygen/generator.toml").exists());
        assert!(dir.path().join(".jujo/templates/mygen/hello.tera").exists());
    }

    #[test]
    fn add_template_already_exists() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join(".jujo/templates/mygen");
        std::fs::create_dir_all(&dest).unwrap();
        let src = setup_source(&dir);

        let err = add(dir.path(), "mygen", src.to_str().unwrap(), false).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn add_template_force_overwrite() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join(".jujo/templates/mygen");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("old.txt"), "old").unwrap();
        let src = setup_source(&dir);

        add(dir.path(), "mygen", src.to_str().unwrap(), true).unwrap();

        assert!(dir.path().join(".jujo/templates/mygen/generator.toml").exists());
        assert!(!dir.path().join(".jujo/templates/mygen/old.txt").exists());
    }

    #[test]
    fn remove_template() {
        let dir = TempDir::new().unwrap();
        let gen_dir = dir.path().join(".jujo/templates/mygen");
        std::fs::create_dir_all(&gen_dir).unwrap();
        std::fs::write(gen_dir.join("generator.toml"), "").unwrap();

        remove(dir.path(), "mygen").unwrap();
        assert!(!gen_dir.exists());
    }

    #[test]
    fn remove_template_not_found() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo/templates")).unwrap();

        let err = remove(dir.path(), "nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
