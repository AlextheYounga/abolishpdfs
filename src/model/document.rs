use serde::Serialize;

use crate::fonts::{FontForgeWorker, FontJobRequest};

use super::{FontCatalog, OutlineItem, PageModel};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocumentModel {
    pub pages: Vec<PageModel>,
    pub fonts: FontCatalog,
    pub outlines: Vec<OutlineItem>,
    pub diagnostics: Vec<DocumentDiagnostic>,
}

impl DocumentModel {
    pub fn prepare_fonts(&mut self, worker: &FontForgeWorker) {
        let font_ids = self.fonts.fonts.keys().copied().collect::<Vec<_>>();
        for id in font_ids {
            let result = self.fonts.fonts.get(&id).map(|source| worker.process(&FontJobRequest { id, source }));
            match result {
                Some(Ok(result)) => {
                    if let Some(source) = self.fonts.fonts.get_mut(&id) {
                        source.processed = Some(result.font);
                    }
                }
                Some(Err(error)) => {
                    self.diagnostics
                        .push(DocumentDiagnostic { scope: DiagnosticScope::Font(id), message: error.to_string() });
                }
                None => {}
            }
        }
    }
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
