use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::{
    FontCatalog, FontProcessingState, OutlineItem, PageModel, ReconstructionDecision, TextFailureReason,
    TextIntegrityFailure,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentModel {
    pub pages: Vec<PageModel>,
    pub fonts: FontCatalog,
    pub outlines: Vec<OutlineItem>,
    pub diagnostics: Vec<DocumentDiagnostic>,
    /// Additional failures supplied by callers. Object failures are derived from
    /// the current reconstruction and font states, not copied into this list.
    pub text_failures: Vec<TextIntegrityFailure>,
}

impl DocumentModel {
    pub fn discovered_text_failures(&self) -> Vec<TextIntegrityFailure> {
        let mut failures = self
            .pages
            .iter()
            .flat_map(|page| {
                page.text_objects.iter().filter_map(|text_object| {
                    let reason = match &text_object.reconstruction {
                        ReconstructionDecision::TextFailure(reason) => reason.clone(),
                        ReconstructionDecision::NativeText
                            if !text_object.glyphs.is_empty()
                                && matches!(
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
                        semantic_text_available: !matches!(reason, TextFailureReason::ExtractionError)
                            && text_object.glyphs.iter().all(|glyph| glyph.unicode.is_some()),
                        reason,
                    })
                })
            })
            .collect::<Vec<_>>();
        for failure in &self.text_failures {
            if !failures.iter().any(|reported| {
                reported.page == failure.page
                    && reported.paint_order == failure.paint_order
                    && reported.reason == failure.reason
            }) {
                failures.push(failure.clone());
            }
        }
        failures
    }
}

impl Serialize for DocumentModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut document = serializer.serialize_struct("DocumentModel", 5)?;
        document.serialize_field("pages", &self.pages)?;
        document.serialize_field("fonts", &self.fonts)?;
        document.serialize_field("outlines", &self.outlines)?;
        document.serialize_field("diagnostics", &self.diagnostics)?;
        document.serialize_field("text_failures", &self.discovered_text_failures())?;
        document.end()
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
