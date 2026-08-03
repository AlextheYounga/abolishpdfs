//! Integration test: detect duplicated state strings in production Rust code.
//!
//! Repeating string-backed state in multiple files makes later changes easy to
//! miss. Product-owned paths, config keys, and environment variables should have
//! one canonical constant.
//!
//! Syntax-only: scans string literals without type inference.

use std::collections::BTreeMap;
use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

const SOURCE_DIRS: &[&str] = &["src", "app", "lib"];
const IGNORE_DIRS: &[&str] = &["target", "vendor", "node_modules", ".git"];

#[derive(Debug)]
struct StateLiteral {
    value: String,
    kind: StateKind,
    location: String,
    context: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum StateKind {
    Config,
    Env,
    Path,
}

/// Detects duplicated string literals that carry product state.
#[test]
fn no_duplicated_state_literals() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let literals = collect_state_literals(project_root);

    assert!(!literals.is_empty(), "No production Rust state literals found under src/, app/, or lib/.");

    let mut by_value: BTreeMap<(StateKind, String), Vec<StateLiteral>> = BTreeMap::new();
    for literal in literals {
        by_value.entry((literal.kind, literal.value.clone())).or_default().push(literal);
    }

    let violations = by_value
        .into_iter()
        .filter(|(_, literals)| literals_are_duplicated_state(literals))
        .map(|((kind, value), literals)| format_violation(kind, &value, &literals))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "Duplicated state literals found. Review each path/config/env value and centralize intentional state:\n{}",
        violations.join("\n")
    );
}

fn collect_state_literals(project_root: &Path) -> Vec<StateLiteral> {
    let mut literals = Vec::new();
    let mut stack: Vec<PathBuf> =
        SOURCE_DIRS.iter().map(|dir| project_root.join(dir)).filter(|path| path.is_dir()).collect();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                if !IGNORE_DIRS.contains(&name.to_string_lossy().as_ref()) {
                    stack.push(path);
                }
                continue;
            }

            if path.extension().is_some_and(|ext| ext == "rs") {
                collect_file_literals(project_root, &path, &mut literals);
            }
        }
    }

    literals
}

fn collect_file_literals(project_root: &Path, path: &Path, literals: &mut Vec<StateLiteral>) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    let ignored_lines = ignored_test_line_ranges(&content);

    for (line_number, line) in content.lines().enumerate() {
        let line_number = line_number + 1;
        if ignored_lines.iter().any(|range| range.contains(&line_number)) {
            continue;
        }

        if line.contains("env!(\"") {
            continue;
        }

        let relative = path.strip_prefix(project_root).unwrap_or(path);
        for value in parse_string_literals(line) {
            let Some(kind) = state_kind(&value) else {
                continue;
            };

            if is_noise_value(&value) {
                continue;
            }

            literals.push(StateLiteral {
                value,
                kind,
                location: format!("{}:{line_number}", relative.display()),
                context: line.trim().to_string(),
            });
        }
    }
}

fn ignored_test_line_ranges(content: &str) -> Vec<RangeInclusive<usize>> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut ignored = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let trimmed = lines[index].trim();
        if !is_test_attribute(trimmed) {
            index += 1;
            continue;
        }

        let start = index + 1;
        index += 1;

        while index < lines.len() && is_attribute_line(lines[index].trim()) {
            index += 1;
        }

        let Some(end) = find_item_end_line(&lines, index) else {
            ignored.push(start..=start);
            continue;
        };

        ignored.push(start..=end);
        index = end;
    }

    ignored
}

fn is_test_attribute(line: &str) -> bool {
    matches!(line, "#[cfg(test)]" | "#[test]")
}

fn is_attribute_line(line: &str) -> bool {
    line.starts_with("#[")
}

fn find_item_end_line(lines: &[&str], start_index: usize) -> Option<usize> {
    if start_index >= lines.len() {
        return None;
    }

    let mut brace_depth = 0usize;
    let mut saw_open_brace = false;

    for (index, line) in lines.iter().enumerate().skip(start_index) {
        for ch in line.chars() {
            if ch == '{' {
                brace_depth += 1;
                saw_open_brace = true;
            } else if ch == '}' && brace_depth > 0 {
                brace_depth -= 1;
                if brace_depth == 0 && saw_open_brace {
                    return Some(index + 1);
                }
            }
        }

        if !saw_open_brace && line.trim_end().ends_with(';') {
            return Some(index + 1);
        }
    }

    if saw_open_brace { Some(lines.len()) } else { None }
}

fn parse_string_literals(line: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut value = String::new();
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if !in_string {
            if ch == '/' && chars.peek() == Some(&'/') {
                break;
            }
            if ch == '"' {
                in_string = true;
                value.clear();
            }
            continue;
        }

        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '"' {
            literals.push(value.clone());
            in_string = false;
            continue;
        }

        value.push(ch);
    }

    literals
}

fn state_kind(value: &str) -> Option<StateKind> {
    if is_path_like(value) {
        return Some(StateKind::Path);
    }
    if is_env_var_like(value) {
        return Some(StateKind::Env);
    }
    if is_config_like(value) {
        return Some(StateKind::Config);
    }

    None
}

fn is_path_like(value: &str) -> bool {
    value.contains('/')
        || value.starts_with('.')
        || [".gpg", ".service", ".sh", ".socket", ".toml"].iter().any(|suffix| value.ends_with(suffix))
}

fn is_env_var_like(value: &str) -> bool {
    value.len() >= 3
        && value.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        && value.chars().any(|ch| ch.is_ascii_uppercase())
}

fn is_config_like(value: &str) -> bool {
    value.contains('_')
        && value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        && value.chars().any(|ch| ch.is_ascii_lowercase())
}

fn literals_are_duplicated_state(literals: &[StateLiteral]) -> bool {
    literals.len() > 1
        && literals
            .iter()
            .map(|literal| file_path(&literal.location))
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| pair[0] != pair[1])
}

fn file_path(location: &str) -> &str {
    location.rsplit_once(':').map_or(location, |(path, _)| path)
}

fn is_noise_value(value: &str) -> bool {
    matches!(value, "." | "{}/" | "{}")
}

fn format_violation(kind: StateKind, value: &str, literals: &[StateLiteral]) -> String {
    let locations = literals
        .iter()
        .map(|literal| format!("{} `{}`", literal.location, literal.context))
        .collect::<Vec<_>>()
        .join("; ");

    format!("  {kind:?} {value:?}: {locations}")
}

#[test]
fn ignored_test_line_ranges_skip_cfg_test_modules_and_test_functions() {
    let content = r#"
const KEEP: &str = "runtime_user";

#[cfg(test)]
mod tests {
    #[test]
    fn local_test() {
        let _value = ".bones/bones.toml";
    }
}

#[test]
fn top_level_test() {
    let _value = "ssh_port";
}

const ALSO_KEEP: &str = ".bones/runtime.toml";
"#;

    let ignored = ignored_test_line_ranges(content);

    assert!(ignored.iter().any(|range| range.contains(&4) && range.contains(&10)));
    assert!(ignored.iter().any(|range| range.contains(&12) && range.contains(&15)));
    assert!(!ignored.iter().any(|range| range.contains(&2)));
    assert!(!ignored.iter().any(|range| range.contains(&17)));
}
