use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

const JUJO_DIR: &str = ".jujo";

/// Walk up from `start` to find the nearest `.jujo/` directory.
pub fn find_jujo_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(JUJO_DIR);
        if candidate.is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            bail!(
                "no .jujo/ directory found (searched up from {}). Run `jujo init` first.",
                start.display()
            );
        }
    }
}

/// Resolve the template directory for a named generator.
pub fn generator_dir(jujo_root: &Path, name: &str) -> Result<PathBuf> {
    let dir = jujo_root.join(JUJO_DIR).join("templates").join(name);
    if !dir.is_dir() {
        bail!(
            "unknown generator \"{name}\". No directory at {}",
            dir.display()
        );
    }
    Ok(dir)
}

/// Path to the manifest file.
pub fn manifest_path(jujo_root: &Path) -> PathBuf {
    jujo_root.join(JUJO_DIR).join("last-generate.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn find_jujo_root_in_current_dir() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();
        let root = find_jujo_root(dir.path()).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn find_jujo_root_in_parent_dir() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo")).unwrap();
        let child = dir.path().join("src").join("nested");
        std::fs::create_dir_all(&child).unwrap();
        let root = find_jujo_root(&child).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn find_jujo_root_not_found() {
        let dir = TempDir::new().unwrap();
        let err = find_jujo_root(dir.path()).unwrap_err();
        assert!(err.to_string().contains("no .jujo/ directory found"));
        assert!(err.to_string().contains("jujo init"));
    }

    #[test]
    fn generator_dir_not_found() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".jujo/templates")).unwrap();
        let err = generator_dir(dir.path(), "nonexistent").unwrap_err();
        assert!(err.to_string().contains("unknown generator"));
    }
}
