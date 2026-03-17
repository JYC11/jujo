use crate::manifest::ManifestCustomizeMarker;

/// Scan rendered content for `// <ai:customize hint="...">` markers
/// and return their locations with hints.
pub fn extract_ai_markers(file_path: &str, content: &str) -> Vec<ManifestCustomizeMarker> {
    let mut markers = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(hint) = parse_ai_marker(trimmed) {
            markers.push(ManifestCustomizeMarker {
                path: file_path.to_string(),
                line: line_idx + 1,
                hint,
            });
        }
    }

    markers
}

/// Try to parse an AI customize marker from a line.
/// Supports any comment prefix: `// <ai:customize hint="...">`, `# <ai:customize hint="...">`,
/// `<!-- <ai:customize hint="..."> -->`, etc.
fn parse_ai_marker(line: &str) -> Option<String> {
    let marker_start = "<ai:customize";
    let pos = line.find(marker_start)?;
    let after_marker = &line[pos + marker_start.len()..];

    // Find hint="..."
    let hint_start = after_marker.find("hint=\"")?;
    let hint_value_start = hint_start + 6; // len of `hint="`
    let remaining = &after_marker[hint_value_start..];
    let hint_end = remaining.find('"')?;
    let hint = &remaining[..hint_end];

    Some(hint.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_rust_markers() {
        let content = r#"fn create_order() {
    // <ai:customize hint="Add order validation logic">
    let validated = validate(req)?;
    // </ai:customize>
    save(validated)
}"#;
        let markers = extract_ai_markers("src/orders/service.rs", content);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].path, "src/orders/service.rs");
        assert_eq!(markers[0].line, 2);
        assert_eq!(markers[0].hint, "Add order validation logic");
    }

    #[test]
    fn extract_python_markers() {
        let content = "# <ai:customize hint=\"Implement business rules\">\npass\n# </ai:customize>";
        let markers = extract_ai_markers("app/service.py", content);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].hint, "Implement business rules");
    }

    #[test]
    fn extract_html_markers() {
        let content = "<!-- <ai:customize hint=\"Add page content\"> -->\n<div></div>\n<!-- </ai:customize> -->";
        let markers = extract_ai_markers("index.html", content);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].hint, "Add page content");
    }

    #[test]
    fn extract_multiple_markers() {
        let content = "line1\n// <ai:customize hint=\"First\">\nline3\n// </ai:customize>\nline5\n// <ai:customize hint=\"Second\">\nline7\n// </ai:customize>";
        let markers = extract_ai_markers("file.rs", content);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].line, 2);
        assert_eq!(markers[0].hint, "First");
        assert_eq!(markers[1].line, 6);
        assert_eq!(markers[1].hint, "Second");
    }

    #[test]
    fn no_markers() {
        let content = "fn main() {\n    println!(\"hello\");\n}";
        let markers = extract_ai_markers("main.rs", content);
        assert!(markers.is_empty());
    }

    #[test]
    fn closing_tag_ignored() {
        let content = "// </ai:customize>";
        let markers = extract_ai_markers("file.rs", content);
        assert!(markers.is_empty());
    }
}
