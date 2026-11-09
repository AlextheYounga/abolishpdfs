use crate::model::{DiagnosticScope, DocumentDiagnostic, DocumentModel, FontProcessingFailure, FontProcessingState};

use super::{FontForgeWorker, FontJobRequest};

/// Processes embedded fonts and records failures without discarding worker diagnostics.
pub fn prepare_document(model: &mut DocumentModel, worker: &FontForgeWorker) {
    for (&id, source) in &mut model.fonts.fonts {
        if source.embedded != Some(true) {
            source.processing = FontProcessingState::NotRequired;
            continue;
        }
        source.processing = match worker.process(&FontJobRequest { id, source }) {
            Ok(result) => FontProcessingState::Ready(result.font),
            Err(error) => {
                let message = error.to_string();
                model
                    .diagnostics
                    .push(DocumentDiagnostic { scope: DiagnosticScope::Font(id), message: message.clone() });
                FontProcessingState::Failed(FontProcessingFailure { message })
            }
        };
    }
}

impl DocumentModel {
    /// Compatibility entry point; new callers should use [`prepare_document`].
    pub fn prepare_fonts(&mut self, worker: &FontForgeWorker) {
        prepare_document(self, worker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fonts::WorkerConfig,
        model::{FontCatalog, FontSource},
    };

    #[test]
    fn preparation_preserves_worker_errors_and_skips_non_embedded_fonts() {
        let mut model = DocumentModel {
            pages: Vec::new(),
            fonts: FontCatalog::new(),
            outlines: Vec::new(),
            diagnostics: Vec::new(),
            text_failures: Vec::new(),
        };
        let source = FontSource {
            name: "Embedded".to_owned(),
            embedded: Some(true),
            data: Some(vec![0, 1, 0, 0]),
            used_unicode: vec!['A'],
            used_identities: Vec::new(),
            mapping_proven: true,
            processing: FontProcessingState::Pending,
        };
        model.fonts.insert(source.clone());
        model.fonts.insert(FontSource { embedded: Some(false), ..source });
        prepare_document(&mut model, &FontForgeWorker::new(WorkerConfig::new("missing-fontforge")));
        assert!(matches!(model.fonts.fonts[&0].processing, FontProcessingState::Failed(_)));
        assert_eq!(model.fonts.fonts[&1].processing, FontProcessingState::NotRequired);
        assert_eq!(model.diagnostics.len(), 1);
        assert_eq!(model.diagnostics[0].scope, DiagnosticScope::Font(0));
        assert!(model.diagnostics[0].message.contains("could not start FontForge"));
    }
}
