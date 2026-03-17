use anyhow::{bail, Context, Result};
use std::path::Path;

/// Result of creating a file.
#[derive(Debug)]
pub struct CreatedFile {
    pub path: String,
    pub template: String,
}

/// Write content to a file, creating parent directories as needed.
/// Errors if the file already exists unless `force` is true.
pub fn create_file(
    root: &Path,
    relative_path: &str,
    content: &str,
    template_name: &str,
    force: bool,
) -> Result<CreatedFile> {
    let full_path = root.join(relative_path);

    if full_path.exists() && !force {
        bail!(
            "file already exists: {}. Use --force to overwrite.",
            relative_path
        );
    }

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    std::fs::write(&full_path, content)
        .with_context(|| format!("failed to write {}", full_path.display()))?;

    Ok(CreatedFile {
        path: relative_path.to_string(),
        template: template_name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_file_basic() {
        let dir = TempDir::new().unwrap();
        let result = create_file(dir.path(), "hello.txt", "content", "hello.tera", false).unwrap();
        assert_eq!(result.path, "hello.txt");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("hello.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn create_file_nested_dirs() {
        let dir = TempDir::new().unwrap();
        create_file(
            dir.path(),
            "src/orders/mod.rs",
            "mod routes;",
            "mod.tera",
            false,
        )
        .unwrap();
        assert!(dir.path().join("src/orders/mod.rs").exists());
    }

    #[test]
    fn create_file_exists_no_force() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("existing.txt"), "old").unwrap();
        let err =
            create_file(dir.path(), "existing.txt", "new", "test.tera", false).unwrap_err();
        assert!(err.to_string().contains("already exists"));
        assert!(err.to_string().contains("--force"));
        // Original content preserved.
        assert_eq!(
            std::fs::read_to_string(dir.path().join("existing.txt")).unwrap(),
            "old"
        );
    }

    #[test]
    fn create_file_exists_with_force() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("existing.txt"), "old").unwrap();
        create_file(dir.path(), "existing.txt", "new", "test.tera", true).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("existing.txt")).unwrap(),
            "new"
        );
    }
}
