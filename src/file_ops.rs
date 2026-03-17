use anyhow::{bail, Context, Result};
use std::path::Path;

/// Result of creating a file.
#[derive(Debug)]
pub struct CreatedFile {
    pub path: String,
    pub template: String,
}

/// Result of injecting content into a file.
#[derive(Debug)]
pub struct InjectedContent {
    pub path: String,
    pub marker: String,
    pub content: String,
    pub skipped: bool,
}

/// How to handle injection conflicts (content already present near marker).
#[derive(Debug, Clone, Copy)]
pub enum ConflictMode {
    Error,
    Force,
    Skip,
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

/// Inject content before a closing marker in a file.
/// Marker format: `{prefix} </jujo:{marker}> {suffix}`
pub fn inject_before_marker(
    root: &Path,
    relative_path: &str,
    marker_name: &str,
    content: &str,
    comment_prefix: &str,
    comment_suffix: &str,
    conflict_mode: ConflictMode,
) -> Result<InjectedContent> {
    let full_path = root.join(relative_path);
    let source = std::fs::read_to_string(&full_path)
        .with_context(|| format!("failed to read {}", full_path.display()))?;

    let marker_tag = if comment_suffix.is_empty() {
        format!("{comment_prefix} </jujo:{marker_name}>")
    } else {
        format!("{comment_prefix} </jujo:{marker_name}> {comment_suffix}")
    };

    let Some(marker_pos) = source.find(&marker_tag) else {
        bail!(
            "marker \"{}\" not found in {}. Expected: {}",
            marker_name,
            relative_path,
            marker_tag
        );
    };

    // Check for conflict: is the content already present between start of file and marker?
    let region_before_marker = &source[..marker_pos];
    let content_trimmed = content.trim();
    if region_before_marker.contains(content_trimmed) {
        match conflict_mode {
            ConflictMode::Error => {
                bail!(
                    "content already present near marker \"{}\" in {}. \
                     Use --force to inject anyway or --skip-existing to skip.",
                    marker_name,
                    relative_path
                );
            }
            ConflictMode::Skip => {
                return Ok(InjectedContent {
                    path: relative_path.to_string(),
                    marker: marker_name.to_string(),
                    content: content.to_string(),
                    skipped: true,
                });
            }
            ConflictMode::Force => {
                // Fall through to inject anyway.
            }
        }
    }

    // Insert content before the marker line.
    let mut output = String::with_capacity(source.len() + content.len() + 1);
    output.push_str(&source[..marker_pos]);
    output.push_str(content_trimmed);
    output.push('\n');
    output.push_str(&source[marker_pos..]);

    std::fs::write(&full_path, output)
        .with_context(|| format!("failed to write {}", full_path.display()))?;

    Ok(InjectedContent {
        path: relative_path.to_string(),
        marker: marker_name.to_string(),
        content: content.to_string(),
        skipped: false,
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
        create_file(dir.path(), "src/orders/mod.rs", "mod routes;", "mod.tera", false).unwrap();
        assert!(dir.path().join("src/orders/mod.rs").exists());
    }

    #[test]
    fn create_file_exists_no_force() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("existing.txt"), "old").unwrap();
        let err = create_file(dir.path(), "existing.txt", "new", "test.tera", false).unwrap_err();
        assert!(err.to_string().contains("already exists"));
        assert!(err.to_string().contains("--force"));
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

    // --- Injection tests ---

    fn write_file_with_marker(dir: &TempDir, name: &str, prefix: &str, suffix: &str, marker: &str) {
        let tag = if suffix.is_empty() {
            format!("{prefix} </jujo:{marker}>")
        } else {
            format!("{prefix} </jujo:{marker}> {suffix}")
        };
        let content = format!("before\n{tag}\nafter\n");
        std::fs::write(dir.path().join(name), content).unwrap();
    }

    #[test]
    fn inject_before_rust_marker() {
        let dir = TempDir::new().unwrap();
        write_file_with_marker(&dir, "main.rs", "//", "", "modules");
        let result = inject_before_marker(
            dir.path(), "main.rs", "modules", "mod orders;", "//", "", ConflictMode::Error,
        ).unwrap();
        assert!(!result.skipped);
        let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
        assert!(content.contains("mod orders;\n// </jujo:modules>"));
    }

    #[test]
    fn inject_before_hash_marker() {
        let dir = TempDir::new().unwrap();
        write_file_with_marker(&dir, "config.py", "#", "", "imports");
        inject_before_marker(
            dir.path(), "config.py", "imports", "import os", "#", "", ConflictMode::Error,
        ).unwrap();
        let content = std::fs::read_to_string(dir.path().join("config.py")).unwrap();
        assert!(content.contains("import os\n# </jujo:imports>"));
    }

    #[test]
    fn inject_before_html_marker() {
        let dir = TempDir::new().unwrap();
        write_file_with_marker(&dir, "index.html", "<!--", "-->", "scripts");
        inject_before_marker(
            dir.path(), "index.html", "scripts", "<script src=\"app.js\"></script>",
            "<!--", "-->", ConflictMode::Error,
        ).unwrap();
        let content = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(content.contains("<script src=\"app.js\"></script>\n<!-- </jujo:scripts> -->"));
    }

    #[test]
    fn inject_marker_not_found() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file.rs"), "no marker here\n").unwrap();
        let err = inject_before_marker(
            dir.path(), "file.rs", "modules", "mod x;", "//", "", ConflictMode::Error,
        ).unwrap_err();
        assert!(err.to_string().contains("marker \"modules\" not found"));
    }

    #[test]
    fn inject_multiple_markers_same_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("main.rs"),
            "// </jujo:modules>\n// </jujo:routes>\n",
        ).unwrap();
        inject_before_marker(
            dir.path(), "main.rs", "modules", "mod orders;", "//", "", ConflictMode::Error,
        ).unwrap();
        inject_before_marker(
            dir.path(), "main.rs", "routes", ".nest(\"/orders\", orders::router())",
            "//", "", ConflictMode::Error,
        ).unwrap();
        let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
        assert!(content.contains("mod orders;\n// </jujo:modules>"));
        assert!(content.contains(".nest(\"/orders\", orders::router())\n// </jujo:routes>"));
    }

    #[test]
    fn inject_conflict_error_by_default() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("main.rs"),
            "mod orders;\n// </jujo:modules>\n",
        ).unwrap();
        let err = inject_before_marker(
            dir.path(), "main.rs", "modules", "mod orders;", "//", "", ConflictMode::Error,
        ).unwrap_err();
        assert!(err.to_string().contains("content already present"));
        assert!(err.to_string().contains("--force"));
        assert!(err.to_string().contains("--skip-existing"));
    }

    #[test]
    fn inject_conflict_force() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("main.rs"),
            "mod orders;\n// </jujo:modules>\n",
        ).unwrap();
        let result = inject_before_marker(
            dir.path(), "main.rs", "modules", "mod orders;", "//", "", ConflictMode::Force,
        ).unwrap();
        assert!(!result.skipped);
        // Content is duplicated (user asked for force).
        let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
        assert_eq!(content.matches("mod orders;").count(), 2);
    }

    #[test]
    fn inject_conflict_skip() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("main.rs"),
            "mod orders;\n// </jujo:modules>\n",
        ).unwrap();
        let result = inject_before_marker(
            dir.path(), "main.rs", "modules", "mod orders;", "//", "", ConflictMode::Skip,
        ).unwrap();
        assert!(result.skipped);
        // Content not duplicated.
        let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
        assert_eq!(content.matches("mod orders;").count(), 1);
    }
}
