//! Integration test: detect vague catch-all names in Rust source.
//!
//! Flags names and comments that lean on fuzzy terms like `misc`, `stuff`,
//! `thing`, `generic`, `helper`, or `util` instead of describing the actual job.
//!
//! Syntax-only: scans text line by line without type inference.

use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_DIRS: &[&str] = &["src", "app", "lib"];
const IGNORE_DIRS: &[&str] = &["target", "vendor", "node_modules", ".git"];
const VAGUE_TERMS: &[&str] = &[
    "misc", "stuff", "thing", "things", "various", "generic", "helper", "helpers", "util", "utils", "other", "others",
];

fn collect_source_files(project_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack: Vec<PathBuf> =
        SOURCE_DIRS.iter().map(|dir| project_root.join(dir)).filter(|path| path.is_dir()).collect();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !IGNORE_DIRS.contains(&name.as_str()) {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }

    files
}

fn vague_terms_in_line(line: &str) -> Vec<&'static str> {
    let normalized = line
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>();

    let mut matches: Vec<&'static str> = Vec::new();
    for term in VAGUE_TERMS {
        if normalized.split_whitespace().any(|token| token == *term) {
            matches.push(*term);
        }
    }

    matches
}

#[test]
fn no_vague_naming() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = collect_source_files(project_root);

    assert!(!files.is_empty(), "No source files found in src/, app/, or lib/.");

    let mut all_violations = Vec::new();
    let mut files_scanned = 0;

    for file in &files {
        let Ok(code) = fs::read_to_string(file) else {
            continue;
        };
        files_scanned += 1;

        for (line_number, line) in code.lines().enumerate() {
            let terms = vague_terms_in_line(line);
            if terms.is_empty() {
                continue;
            }

            let relative = file.strip_prefix(project_root).unwrap_or(file);
            all_violations.push(format!(
                "  {}:{}: {} [{}]",
                relative.display(),
                line_number + 1,
                line.trim(),
                terms.join(", ")
            ));
        }
    }

    assert!(files_scanned > 0, "No readable Rust files were found to check.");

    assert!(all_violations.is_empty(), "Vague naming terms found:\n{}", all_violations.join("\n"));
}
