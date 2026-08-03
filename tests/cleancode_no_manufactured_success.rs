//! Integration test: detect suspicious fallback patterns.
//!
//! Flags `match` arms on `Option` or `Result` where the failure variant
//! (`None` / `Err`) produces a success value (`Some(…)` / `Ok(…)`),
//! indicating silent error recovery (the AI anti-pattern).
//!
//! Syntax-only: works on the AST without type inference.

use std::fs;
use std::path::{Path, PathBuf};

use syn::{
    Expr, ExprCall, ExprMatch,
    visit::{self, Visit},
};

const IGNORE_DIRS: &[&str] = &["target", "vendor", "node_modules", ".git"];
const SOURCE_DIRS: &[&str] = &["src", "app", "lib"];

struct SuspiciousMatchVisitor {
    violations: Vec<String>,
}

impl<'ast> Visit<'ast> for SuspiciousMatchVisitor {
    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        let has_success_arm = node.arms.iter().any(|arm| is_success_pattern(&arm.pat));
        let produces_success =
            node.arms.iter().any(|arm| is_failure_pattern(&arm.pat) && arm_constructs_success(arm.body.as_ref()));

        if has_success_arm && produces_success {
            self.violations.push("suspicious match arm constructs `Ok(…)`/`Some(…)` from error path".to_string());
        }

        visit::visit_expr_match(self, node);
    }
}

fn is_success_pattern(pat: &syn::Pat) -> bool {
    matches!(pat, syn::Pat::TupleStruct(pts) if pts.path.get_ident().is_some_and(|i| matches!(i.to_string().as_str(), "Ok" | "Some")))
}

fn is_failure_pattern(pat: &syn::Pat) -> bool {
    matches!(pat, syn::Pat::TupleStruct(pts) if pts.path.get_ident().is_some_and(|i| i == "Err"))
        || matches!(pat, syn::Pat::Path(pp) if pp.path.get_ident().is_some_and(|i| i == "None"))
}

fn arm_constructs_success(expr: &Expr) -> bool {
    match expr {
        Expr::Call(ExprCall { func, .. }) => {
            matches!(func.as_ref(), Expr::Path(p) if p.path.get_ident().is_some_and(|i| matches!(i.to_string().as_str(), "Ok" | "Some")))
        }
        _ => false,
    }
}

fn collect_source_files(project_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack: Vec<PathBuf> =
        SOURCE_DIRS.iter().map(|dir| project_root.join(dir)).filter(|path| path.is_dir()).collect();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !IGNORE_DIRS.contains(&name.as_str()) {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|e| e == "rs") {
                files.push(p);
            }
        }
    }

    files
}

#[test]
fn no_suspicious_fallback() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = collect_source_files(project_root);

    assert!(!files.is_empty(), "No source files found in src/, app/, or lib/.");

    let mut all_violations = Vec::new();
    let mut files_scanned = 0;

    for file in &files {
        let Ok(code) = fs::read_to_string(file) else {
            continue;
        };
        let Ok(syntax) = syn::parse_file(&code) else {
            continue;
        };
        files_scanned += 1;

        let mut visitor = SuspiciousMatchVisitor { violations: Vec::new() };
        visitor.visit_file(&syntax);

        for msg in &visitor.violations {
            let relative = file.strip_prefix(project_root).unwrap_or(file);
            all_violations.push(format!("  {} — {msg}", relative.display()));
        }
    }

    assert!(files_scanned > 0, "No parsable Rust files were found to check.");

    assert!(all_violations.is_empty(), "Suspicious fallback(s) found:\n{}", all_violations.join("\n"),);
}
