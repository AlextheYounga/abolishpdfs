use serde::Serialize;

use crate::fonts::{FontForgeWorker, FontJobRequest};

use super::{FontCatalog, OutlineItem, PageModel, ReconstructionDecision, TextIntegrityFailure};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocumentModel {
    pub pages: Vec<PageModel>,
    pub fonts: FontCatalog,
    pub outlines: Vec<OutlineItem>,
    pub diagnostics: Vec<DocumentDiagnostic>,
    pub text_failures: Vec<TextIntegrityFailure>,
}

impl DocumentModel {
    pub fn discovered_text_failures(&self) -> Vec<TextIntegrityFailure> {
        self.pages
            .iter()
            .flat_map(|page| {
                page.text_objects.iter().filter_map(|text_object| {
                    let ReconstructionDecision::TextFailure(reason) = text_object.reconstruction.clone() else {
                        return None;
                    };
                    Some(TextIntegrityFailure {
                        page: page.number,
                        paint_order: text_object.paint_order,
                        reason,
                        semantic_text_available: text_object.glyphs.iter().all(|glyph| glyph.unicode.is_some()),
                    })
                })
            })
            .collect()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{TextFailureReason, TextIntegrityFailure};

    #[test]
    fn serializes_text_integrity_context() {
        let model = DocumentModel {
            pages: Vec::new(),
            fonts: FontCatalog::new(),
            outlines: Vec::new(),
            diagnostics: Vec::new(),
            text_failures: vec![TextIntegrityFailure {
                page: 2,
                paint_order: 7,
                reason: TextFailureReason::MissingUnicode,
                semantic_text_available: false,
            }],
        };

        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("\"page\":2"));
        assert!(json.contains("\"paint_order\":7"));
        assert!(json.contains("MissingUnicode"));
    }
}
