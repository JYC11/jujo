use crate::types::{CommentStyle, MarkerName, RelativePath};
use anyhow::{Context, Result, bail};
use std::path::Path;

/// Result of creating a file.
#[derive(Debug)]
pub struct CreatedFile {
    pub path: RelativePath,
    pub template: String,
}

/// Result of injecting content into a file.
#[derive(Debug)]
pub struct InjectedContent {
    pub path: RelativePath,
    pub marker: MarkerName,
    pub content: String,
    pub skipped: bool,
}

/// How to handle injection conflicts (content already present near marker).
#[derive(Debug, Clone, Copy)]
pub enum ConflictMode {
    Error,
    #[allow(dead_code)]
    Force,
    Skip,
}

/// Write content to a file, creating parent directories as needed.
/// Errors if the file already exists unless `force` is true.
pub fn create_file(
    root: &Path,
    relative_path: &RelativePath,
    content: &str,
    template_name: &str,
    force: bool,
) -> Result<CreatedFile> {
    let full_path = root.join(relative_path.as_ref());

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
        path: relative_path.clone(),
        template: template_name.to_string(),
    })
}

/// Inject content before a closing marker in a file.
pub fn inject_before_marker(
    root: &Path,
    relative_path: &RelativePath,
    marker_name: &MarkerName,
    content: &str,
    comment_style: &CommentStyle,
    conflict_mode: ConflictMode,
) -> Result<InjectedContent> {
    let full_path = root.join(relative_path.as_ref());
    let source = std::fs::read_to_string(&full_path)
        .with_context(|| format!("failed to read {}", full_path.display()))?;

    let marker_tag = comment_style.marker_tag(marker_name);

    let Some(marker_pos) = source.find(&marker_tag) else {
        bail!(
            "marker \"{}\" not found in {}. Expected: {}",
            marker_name,
            relative_path,
            marker_tag
        );
    };

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
                    path: relative_path.clone(),
                    marker: marker_name.clone(),
                    content: content.to_string(),
                    skipped: true,
                });
            }
            ConflictMode::Force => {}
        }
    }

    let mut output = String::with_capacity(source.len() + content.len() + 1);
    output.push_str(&source[..marker_pos]);
    output.push_str(content_trimmed);
    output.push('\n');
    output.push_str(&source[marker_pos..]);

    std::fs::write(&full_path, output)
        .with_context(|| format!("failed to write {}", full_path.display()))?;

    Ok(InjectedContent {
        path: relative_path.clone(),
        marker: marker_name.clone(),
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
        let path = RelativePath::new("hello.txt").unwrap();
        let result = create_file(dir.path(), &path, "content", "hello.tera", false).unwrap();
        assert_eq!(result.path.as_ref(), "hello.txt");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("hello.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn create_file_nested_dirs() {
        let dir = TempDir::new().unwrap();
        let path = RelativePath::new("src/orders/mod.rs").unwrap();
        create_file(dir.path(), &path, "mod routes;", "mod.tera", false).unwrap();
        assert!(dir.path().join("src/orders/mod.rs").exists());
    }

    #[test]
    fn create_file_exists_no_force() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("existing.txt"), "old").unwrap();
        let path = RelativePath::new("existing.txt").unwrap();
        let err = create_file(dir.path(), &path, "new", "test.tera", false).unwrap_err();
        assert!(err.to_string().contains("already exists"));
        assert!(err.to_string().contains("--force"));
    }

    #[test]
    fn create_file_exists_with_force() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("existing.txt"), "old").unwrap();
        let path = RelativePath::new("existing.txt").unwrap();
        create_file(dir.path(), &path, "new", "test.tera", true).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("existing.txt")).unwrap(),
            "new"
        );
    }

    // --- Injection tests ---

    fn write_file_with_marker(dir: &TempDir, name: &str, prefix: &str, suffix: &str, marker: &str) {
        let style = CommentStyle::new(prefix, suffix);
        let marker_name = MarkerName::new(marker).unwrap();
        let tag = style.marker_tag(&marker_name);
        let content = format!("before\n{tag}\nafter\n");
        std::fs::write(dir.path().join(name), content).unwrap();
    }

    fn style(prefix: &str, suffix: &str) -> CommentStyle {
        CommentStyle::new(prefix, suffix)
    }

    #[test]
    fn inject_before_rust_marker() {
        let dir = TempDir::new().unwrap();
        write_file_with_marker(&dir, "main.rs", "//", "", "modules");
        let path = RelativePath::new("main.rs").unwrap();
        let marker = MarkerName::new("modules").unwrap();
        let result = inject_before_marker(
            dir.path(),
            &path,
            &marker,
            "mod orders;",
            &style("//", ""),
            ConflictMode::Error,
        )
        .unwrap();
        assert!(!result.skipped);
        let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
        assert!(content.contains("mod orders;\n// </jujo:modules>"));
    }

    #[test]
    fn inject_before_hash_marker() {
        let dir = TempDir::new().unwrap();
        write_file_with_marker(&dir, "config.py", "#", "", "imports");
        let path = RelativePath::new("config.py").unwrap();
        let marker = MarkerName::new("imports").unwrap();
        inject_before_marker(
            dir.path(),
            &path,
            &marker,
            "import os",
            &style("#", ""),
            ConflictMode::Error,
        )
        .unwrap();
        let content = std::fs::read_to_string(dir.path().join("config.py")).unwrap();
        assert!(content.contains("import os\n# </jujo:imports>"));
    }

    #[test]
    fn inject_before_html_marker() {
        let dir = TempDir::new().unwrap();
        write_file_with_marker(&dir, "index.html", "<!--", "-->", "scripts");
        let path = RelativePath::new("index.html").unwrap();
        let marker = MarkerName::new("scripts").unwrap();
        inject_before_marker(
            dir.path(),
            &path,
            &marker,
            "<script src=\"app.js\"></script>",
            &style("<!--", "-->"),
            ConflictMode::Error,
        )
        .unwrap();
        let content = std::fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(content.contains("<script src=\"app.js\"></script>\n<!-- </jujo:scripts> -->"));
    }

    #[test]
    fn inject_marker_not_found() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file.rs"), "no marker here\n").unwrap();
        let path = RelativePath::new("file.rs").unwrap();
        let marker = MarkerName::new("modules").unwrap();
        let err = inject_before_marker(
            dir.path(),
            &path,
            &marker,
            "mod x;",
            &style("//", ""),
            ConflictMode::Error,
        )
        .unwrap_err();
        assert!(err.to_string().contains("marker \"modules\" not found"));
    }

    #[test]
    fn inject_multiple_markers_same_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("main.rs"),
            "// </jujo:modules>\n// </jujo:routes>\n",
        )
        .unwrap();
        let path = RelativePath::new("main.rs").unwrap();
        let m1 = MarkerName::new("modules").unwrap();
        let m2 = MarkerName::new("routes").unwrap();
        inject_before_marker(
            dir.path(),
            &path,
            &m1,
            "mod orders;",
            &style("//", ""),
            ConflictMode::Error,
        )
        .unwrap();
        inject_before_marker(
            dir.path(),
            &path,
            &m2,
            ".nest(\"/orders\", orders::router())",
            &style("//", ""),
            ConflictMode::Error,
        )
        .unwrap();
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
        )
        .unwrap();
        let path = RelativePath::new("main.rs").unwrap();
        let marker = MarkerName::new("modules").unwrap();
        let err = inject_before_marker(
            dir.path(),
            &path,
            &marker,
            "mod orders;",
            &style("//", ""),
            ConflictMode::Error,
        )
        .unwrap_err();
        assert!(err.to_string().contains("content already present"));
    }

    #[test]
    fn inject_conflict_force() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("main.rs"),
            "mod orders;\n// </jujo:modules>\n",
        )
        .unwrap();
        let path = RelativePath::new("main.rs").unwrap();
        let marker = MarkerName::new("modules").unwrap();
        let result = inject_before_marker(
            dir.path(),
            &path,
            &marker,
            "mod orders;",
            &style("//", ""),
            ConflictMode::Force,
        )
        .unwrap();
        assert!(!result.skipped);
        let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
        assert_eq!(content.matches("mod orders;").count(), 2);
    }

    #[test]
    fn inject_conflict_skip() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("main.rs"),
            "mod orders;\n// </jujo:modules>\n",
        )
        .unwrap();
        let path = RelativePath::new("main.rs").unwrap();
        let marker = MarkerName::new("modules").unwrap();
        let result = inject_before_marker(
            dir.path(),
            &path,
            &marker,
            "mod orders;",
            &style("//", ""),
            ConflictMode::Skip,
        )
        .unwrap();
        assert!(result.skipped);
        let content = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
        assert_eq!(content.matches("mod orders;").count(), 1);
    }
}
