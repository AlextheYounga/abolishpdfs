use std::fs;

use super::{HtmlWriter, OutputError, tests::model_with_text};
use crate::model::{
    ClipState, FontProcessingFailure, FontProcessingState, FontSource, GraphicsKind, GraphicsObject, PaintOpacity,
    ReconstructionDecision, TextFailureReason,
};

#[test]
fn diagnostics_and_output_report_current_font_failure_without_a_cached_copy() {
    let mut model = model_with_text("A");
    model.fonts.insert(FontSource {
        name: "Embedded".to_owned(),
        embedded: Some(true),
        data: None,
        used_unicode: vec!['A'],
        used_identities: Vec::new(),
        mapping_proven: true,
        processing: FontProcessingState::Failed(FontProcessingFailure { message: "worker failed".to_owned() }),
    });
    assert!(model.text_failures.is_empty());
    let json = serde_json::to_value(&model).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let Err(OutputError::TextIntegrity(error)) = HtmlWriter::write_to(&model, directory.path()) else {
        panic!("failed embedded font must prevent output");
    };
    assert_eq!(json["text_failures"], serde_json::to_value(error.failures).unwrap());

    model.fonts.fonts.get_mut(&0).unwrap().processing = FontProcessingState::NotRequired;
    assert_eq!(serde_json::to_value(&model).unwrap()["text_failures"], serde_json::json!([]));
}

#[test]
fn extraction_failure_is_not_reported_as_successful_empty_text() {
    let mut model = model_with_text("A");
    model.pages[0].text_objects[0].glyphs.clear();
    model.pages[0].text_objects[0].reconstruction =
        ReconstructionDecision::TextFailure(TextFailureReason::ExtractionError);
    let failures = model.discovered_text_failures();
    assert_eq!(failures.len(), 1);
    assert!(!failures[0].semantic_text_available);
    assert_eq!(failures[0].reason, TextFailureReason::ExtractionError);
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("index.html"), "previous conversion").unwrap();
    assert!(matches!(HtmlWriter::write_to(&model, directory.path()), Err(OutputError::TextIntegrity(_))));
    assert_eq!(fs::read_to_string(directory.path().join("index.html")).unwrap(), "previous conversion");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn explicit_failures_are_enforced_without_duplicating_derived_failures() {
    let mut model = model_with_text("A");
    model.pages[0].text_objects[0].reconstruction =
        ReconstructionDecision::TextFailure(TextFailureReason::MissingGeometry);
    model.text_failures = model.discovered_text_failures();
    model.text_failures[0].semantic_text_available = false;
    assert_eq!(model.discovered_text_failures().len(), 1);
    assert!(model.discovered_text_failures()[0].semantic_text_available);
    model.pages.clear();
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(HtmlWriter::write_to(&model, directory.path()), Err(OutputError::TextIntegrity(_))));
}

#[test]
fn required_background_failure_leaves_existing_artifacts_untouched() {
    let mut model = model_with_text("A");
    model.pages[0].graphics.push(GraphicsObject {
        paint_order: 0,
        kind: GraphicsKind::Path,
        bounds: None,
        active: Some(true),
        opacity: PaintOpacity::Opaque,
        clipping: ClipState::Unclipped,
        children: Vec::new(),
    });
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("index.html"), "previous conversion").unwrap();
    assert!(matches!(HtmlWriter::write_to(&model, directory.path()), Err(OutputError::MissingBackground { page: 1 })));
    assert_eq!(fs::read_to_string(directory.path().join("index.html")).unwrap(), "previous conversion");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}
