use super::{GraphicsKind, GraphicsObject, needs_raster_background};
use crate::model::{ClipState, PaintOpacity};

#[test]
fn analysis_preserves_extraction_failure_but_allows_legitimate_empty_objects() {
    use super::analyze_text_objects;
    use crate::model::{
        DocumentModel, FontCatalog, ReconstructionDecision, TextFailureReason, TextObject, TextRenderMode,
        VisibilityDecision,
    };

    let model = DocumentModel {
        pages: Vec::new(),
        fonts: FontCatalog::new(),
        outlines: Vec::new(),
        diagnostics: Vec::new(),
        text_failures: Vec::new(),
    };
    let empty = TextObject {
        source: 0,
        paint_order: 0,
        glyphs: Vec::new(),
        font: 0,
        render_mode: TextRenderMode::Fill,
        reconstruction: ReconstructionDecision::NativeText,
        visibility: VisibilityDecision::Visible,
        clipping: ClipState::Unclipped,
    };
    let failed = TextObject {
        reconstruction: ReconstructionDecision::TextFailure(TextFailureReason::ExtractionError),
        ..empty.clone()
    };
    let mut objects = vec![empty, failed];
    analyze_text_objects(&mut objects, &[], &model);
    assert_eq!(objects[0].reconstruction, ReconstructionDecision::NativeText);
    assert_eq!(objects[1].reconstruction, ReconstructionDecision::TextFailure(TextFailureReason::ExtractionError));
}

#[test]
fn graphics_require_a_raster_background() {
    let graphics = vec![GraphicsObject {
        paint_order: 0,
        kind: GraphicsKind::Path,
        bounds: None,
        active: Some(true),
        opacity: PaintOpacity::Unknown,
        clipping: ClipState::Unclipped,
        children: Vec::new(),
    }];
    assert!(needs_raster_background(&[], &graphics));
}

#[test]
fn text_only_pages_do_not_require_a_raster_background() {
    assert!(!needs_raster_background(&[], &[]));
}
