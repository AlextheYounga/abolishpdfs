use serde::Serialize;

use crate::fonts::{FontForgeWorker, FontJobRequest};

use super::{
    FontCatalog, FontProcessingFailure, FontProcessingState, OutlineItem, PageModel, ReconstructionDecision,
    TextFailureReason, TextIntegrityFailure,
};

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
                    let reason = match &text_object.reconstruction {
                        ReconstructionDecision::TextFailure(reason) => reason.clone(),
                        ReconstructionDecision::NativeText
                            if matches!(
                                self.fonts.fonts.get(&text_object.font).map(|font| &font.processing),
                                Some(FontProcessingState::Failed(_))
                            ) =>
                        {
                            TextFailureReason::FontProcessingFailed
                        }
                        ReconstructionDecision::NativeText => return None,
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
            let requires_processing = self.fonts.fonts.get(&id).is_some_and(|source| source.embedded == Some(true));
            if !requires_processing {
                if let Some(source) = self.fonts.fonts.get_mut(&id) {
                    source.processing = FontProcessingState::NotRequired;
                }
                continue;
            }

            let result = self.fonts.fonts.get(&id).map(|source| worker.process(&FontJobRequest { id, source }));
            match result {
                Some(Ok(result)) => {
                    if let Some(source) = self.fonts.fonts.get_mut(&id) {
                        source.processing = FontProcessingState::Ready(result.font);
                    }
                }
                Some(Err(error)) => {
                    let message = error.to_string();
                    self.diagnostics
                        .push(DocumentDiagnostic { scope: DiagnosticScope::Font(id), message: message.clone() });
                    if let Some(source) = self.fonts.fonts.get_mut(&id) {
                        source.processing = FontProcessingState::Failed(FontProcessingFailure { message });
                    }
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
    use crate::{
        fonts::WorkerConfig,
        model::{FontSource, TextFailureReason, TextIntegrityFailure},
    };

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

    #[test]
    fn failed_embedded_font_preserves_worker_error_and_state() {
        let mut model = DocumentModel {
            pages: Vec::new(),
            fonts: FontCatalog::new(),
            outlines: Vec::new(),
            diagnostics: Vec::new(),
            text_failures: Vec::new(),
        };
        model.fonts.insert(FontSource {
            name: "Embedded".to_owned(),
            embedded: Some(true),
            data: Some(vec![0, 1, 0, 0]),
            used_unicode: vec!['A'],
            mapping_proven: true,
            processing: FontProcessingState::Pending,
        });

        model.prepare_fonts(&FontForgeWorker::new(WorkerConfig::new("missing-fontforge")));

        assert!(matches!(model.fonts.fonts[&0].processing, FontProcessingState::Failed(FontProcessingFailure { .. })));
        assert_eq!(model.diagnostics.len(), 1);
        assert_eq!(model.diagnostics[0].scope, DiagnosticScope::Font(0));
        assert!(model.diagnostics[0].message.contains("could not start FontForge"));
    }
}
