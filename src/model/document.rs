use serde::Serialize;

use super::{FontCatalog, PageModel};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocumentModel {
    pub pages: Vec<PageModel>,
    pub fonts: FontCatalog,
    pub diagnostics: Vec<DocumentDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentDiagnostic {
    pub scope: DiagnosticScope,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DiagnosticScope {
    Document,
    Page(usize),
    Object { page: usize, paint_order: usize },
    Font(usize),
}
