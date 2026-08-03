//! Integration test: all `.rs` source files must stay at or below 400 lines.
//!
//! This is a structural scan — no type inference needed.
//! Skips `target/`, `vendor/`, and `node_modules/` by convention.

use std::fs;
use std::path::Path;

const MAX_LINES: usize = 400;

#[test]
fn source_files_stay_under_400_lines() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations: Vec<String> = Vec::new();
    let mut file_count = 0;

    visit_dirs(project_root, &mut file_count, &mut violations);

    assert!(file_count > 0, "No source files found. This test should be run from the project root.");

    assert!(violations.is_empty(), "File(s) exceed {} line(s):\n{}", MAX_LINES, violations.join("\n"),);
}

fn visit_dirs(dir: &Path, file_count: &mut usize, violations: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if matches!(name_str.as_ref(), "target" | "vendor" | "node_modules" | ".git") {
                continue;
            }
            visit_dirs(&path, file_count, violations);
            continue;
        }

        if path.extension().is_some_and(|ext| ext == "rs") {
            *file_count += 1;
            if let Ok(content) = fs::read_to_string(&path) {
                let line_count = content.lines().count();
                if line_count > MAX_LINES {
                    let relative = path.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap_or(&path);
                    violations.push(format!("  {}: {} lines (max {MAX_LINES})", relative.display(), line_count));
                }
            }
        }
    }
}
