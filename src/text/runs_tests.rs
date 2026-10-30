use std::{collections::BTreeMap, fs, path::PathBuf};

use super::*;
use crate::model::{
    ClipState, Color, FontSource, Point, ProcessedFont, Rect, Size, TextFailureReason, TextRenderMode,
    VisibilityDecision,
};

fn glyph(character: char, x: f32, y: f32) -> Glyph {
    Glyph {
        unicode: Some(character),
        source_code: None,
        glyph_id: None,
        font: Some(1),
        origin: Point { x, y },
        tight_bounds: Some(Rect { left: x, bottom: y - 10.0, right: x + 8.0, top: y + 2.0 }),
        loose_bounds: None,
        transform: None,
        font_size: 12.0,
        fill: Some(Color::BLACK),
        stroke: None,
        generated_by_pdfium: Some(false),
    }
}

fn object(glyphs: Vec<Glyph>) -> TextObject {
    TextObject {
        source: 4,
        paint_order: 4,
        glyphs,
        font: 1,
        render_mode: TextRenderMode::Fill,
        reconstruction: ReconstructionDecision::NativeText,
        visibility: VisibilityDecision::Visible,
        clipping: ClipState::Unclipped,
    }
}

fn page(text_object: TextObject) -> PageModel {
    page_objects(vec![text_object])
}

fn page_objects(text_objects: Vec<TextObject>) -> PageModel {
    PageModel {
        number: 1,
        size: Size { width: 100.0, height: 100.0 },
        crop_box: Rect { left: 0.0, bottom: 0.0, right: 100.0, top: 100.0 },
        text_objects,
        prepared_runs: Vec::new(),
        graphics: Vec::new(),
        links: Vec::new(),
        background: None,
    }
}

fn fixture_data() -> Option<Vec<u8>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts/LiberationSans-Regular.ttf");
    fs::read(path).ok()
}

fn catalog_with_font(data: Option<Vec<u8>>, processing: FontProcessingState) -> FontCatalog {
    let mut fonts = FontCatalog::new();
    fonts.insert(FontSource {
        name: "Fixture".to_owned(),
        embedded: Some(true),
        data,
        used_unicode: vec!['A'],
        used_identities: Vec::new(),
        mapping_proven: true,
        processing,
    });
    fonts
}

#[test]
fn compatible_glyphs_form_one_run_in_source_order() {
    let runs = prepare_page(&page(object(vec![glyph('A', 10.0, 80.0), glyph('B', 18.0, 80.0)])), &FontCatalog::new());
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "AB");
    assert_eq!(runs[0].observed_advances, vec![8.0]);
}

#[test]
fn compatible_glyphs_merge_across_text_object_boundaries() {
    let runs = prepare_page(
        &page_objects(vec![object(vec![glyph('A', 10.0, 80.0)]), object(vec![glyph('B', 18.0, 80.0)])]),
        &FontCatalog::new(),
    );

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "AB");
    assert_eq!(runs[0].source, 4);
}

#[test]
fn baseline_change_starts_a_new_run() {
    let runs = prepare_page(&page(object(vec![glyph('A', 10.0, 80.0), glyph('B', 18.0, 82.0)])), &FontCatalog::new());
    assert_eq!(runs.len(), 2);
}

#[test]
fn large_horizontal_gap_starts_a_new_run() {
    let runs = prepare_page(&page(object(vec![glyph('A', 10.0, 80.0), glyph('B', 40.0, 80.0)])), &FontCatalog::new());
    assert_eq!(runs.len(), 2);
}

#[test]
fn style_change_starts_a_new_run() {
    let mut second = glyph('B', 18.0, 80.0);
    second.fill = Some(Color { red: 255, green: 0, blue: 0, alpha: 255 });
    let runs = prepare_page(&page(object(vec![glyph('A', 10.0, 80.0), second])), &FontCatalog::new());
    assert_eq!(runs.len(), 2);
}

#[test]
fn protected_cross_object_boundaries_start_new_runs() {
    let mut style = glyph('B', 18.0, 80.0);
    style.fill = Some(Color { red: 255, green: 0, blue: 0, alpha: 255 });
    let baseline = glyph('B', 18.0, 82.0);
    let mut transformed = glyph('B', 18.0, 80.0);
    transformed.transform = Some(AffineTransform { a: 0.0, b: 1.0, c: -1.0, d: 0.0, e: 0.0, f: 0.0 });
    let mut stroke = object(vec![glyph('B', 18.0, 80.0)]);
    stroke.render_mode = TextRenderMode::Stroke;
    let large_gap = object(vec![glyph('B', 40.0, 80.0)]);

    for second in [object(vec![style]), object(vec![baseline]), object(vec![transformed]), stroke, large_gap] {
        let runs = prepare_page(&page_objects(vec![object(vec![glyph('A', 10.0, 80.0)]), second]), &FontCatalog::new());
        assert_eq!(runs.len(), 2);
    }

    let backwards = prepare_page(
        &page_objects(vec![object(vec![glyph('A', 18.0, 80.0)]), object(vec![glyph('B', 10.0, 80.0)])]),
        &FontCatalog::new(),
    );
    assert_eq!(backwards.len(), 2);
}

#[test]
fn transformed_reverse_progression_across_objects_starts_a_new_run() {
    let transform = Some(AffineTransform { a: 0.0, b: 1.0, c: -1.0, d: 0.0, e: 0.0, f: 0.0 });
    let mut first = glyph('A', 10.0, 18.0);
    first.transform = transform;
    let mut second = glyph('B', 10.0, 10.0);
    second.transform = transform;

    let runs = prepare_page(&page_objects(vec![object(vec![first]), object(vec![second])]), &FontCatalog::new());

    assert_eq!(runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);
}

#[test]
fn non_native_objects_terminate_runs_and_discard_pending_spaces() {
    let mut non_native = object(vec![glyph('?', 14.0, 80.0)]);
    non_native.reconstruction = ReconstructionDecision::TextFailure(TextFailureReason::MissingUnicode);
    let runs = prepare_page(
        &page_objects(vec![
            object(vec![glyph('A', 10.0, 80.0)]),
            non_native.clone(),
            object(vec![glyph('B', 18.0, 80.0)]),
        ]),
        &FontCatalog::new(),
    );
    assert_eq!(runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);

    let runs = prepare_page(
        &page_objects(vec![
            object(vec![glyph('A', 10.0, 80.0)]),
            object(Vec::new()),
            non_native,
            object(vec![glyph('B', 18.0, 80.0)]),
        ]),
        &FontCatalog::new(),
    );
    assert_eq!(runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);
}

#[test]
fn empty_native_object_recovers_one_space_and_preserves_next_glyph_position() {
    let processed = ProcessedFont {
        asset_name: "fixture.woff2".to_owned(),
        family_name: "fixture".to_owned(),
        data: Vec::new(),
        glyph_count: 3,
        advance_widths: vec![1000, 1000],
        units_per_em: 1000,
        advances_by_unicode: BTreeMap::from([('A', 1000), (' ', 1000)]),
    };
    let mut first = glyph('A', 0.0, 80.0);
    first.font = Some(0);
    let mut second = glyph('B', 24.0, 80.0);
    second.font = Some(0);
    let runs = prepare_page(
        &page_objects(vec![object(vec![first]), object(Vec::new()), object(vec![second])]),
        &catalog_with_font(None, FontProcessingState::Ready(processed)),
    );

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "A B");
    assert_eq!(runs[0].letter_spacing, 12.0);
    assert_eq!(runs[0].local_offsets, vec![RunOffset { character_index: 1, amount: -24.0 }]);
    assert_eq!(runs[0].observed_advances, vec![0.0, 24.0]);
}

#[test]
fn empty_native_objects_at_run_boundaries_do_not_create_spaces() {
    let mut styled = glyph('B', 18.0, 80.0);
    styled.fill = Some(Color { red: 255, green: 0, blue: 0, alpha: 255 });
    let runs = prepare_page(
        &page_objects(vec![
            object(Vec::new()),
            object(vec![glyph('A', 10.0, 80.0)]),
            object(Vec::new()),
            object(vec![styled]),
            object(Vec::new()),
        ]),
        &FontCatalog::new(),
    );

    assert_eq!(runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);
}

#[test]
fn fallback_objects_do_not_form_native_runs() {
    let mut text_object = object(vec![glyph('A', 10.0, 80.0)]);
    text_object.reconstruction = ReconstructionDecision::TextFailure(TextFailureReason::MissingUnicode);
    assert!(prepare_page(&page(text_object), &FontCatalog::new()).is_empty());
}

#[test]
fn processed_font_advance_takes_precedence_over_source_font() {
    let data = fixture_data();
    assert!(data.is_some(), "Liberation Sans fixture is required");
    let Some(data) = data else { return };
    let processed = ProcessedFont {
        asset_name: "fixture.woff2".to_owned(),
        family_name: "fixture".to_owned(),
        data: Vec::new(),
        glyph_count: 1,
        advance_widths: vec![2000],
        units_per_em: 1000,
        advances_by_unicode: BTreeMap::from([('A', 2000)]),
    };
    let mut test_glyph = glyph('A', 0.0, 0.0);
    test_glyph.font = Some(0);
    let advance = natural_advance(
        &test_glyph,
        'A',
        TransformClass::Identity,
        &catalog_with_font(Some(data), FontProcessingState::Ready(processed)),
    );
    assert_eq!(advance, Some(24.0));
}

#[test]
fn empty_processed_metrics_fall_back_to_source_font_advance() {
    let data = fixture_data();
    assert!(data.is_some(), "Liberation Sans fixture is required");
    let Some(data) = data else { return };
    let Some(face) = Face::parse(&data, 0).ok() else { return };
    let Some(glyph_id) = face.glyph_index('A') else { return };
    let Some(source_advance) = face.glyph_hor_advance(glyph_id) else { return };
    let expected = f32::from(source_advance) / f32::from(face.units_per_em()) * 12.0;
    let mut test_glyph = glyph('A', 0.0, 0.0);
    test_glyph.font = Some(0);
    let actual = natural_advance(
        &test_glyph,
        'A',
        TransformClass::Identity,
        &catalog_with_font(
            Some(data),
            FontProcessingState::Ready(ProcessedFont {
                asset_name: "fixture.woff2".to_owned(),
                family_name: "fixture".to_owned(),
                data: Vec::new(),
                glyph_count: 1,
                advance_widths: Vec::new(),
                units_per_em: 1000,
                advances_by_unicode: BTreeMap::new(),
            }),
        ),
    );
    assert_eq!(actual, Some(expected));
}

#[test]
fn fonts_without_processed_metrics_or_source_data_have_no_advance() {
    let mut test_glyph = glyph('A', 0.0, 0.0);
    test_glyph.font = Some(0);
    let fonts = catalog_with_font(None, FontProcessingState::Pending);
    assert_eq!(natural_advance(&test_glyph, 'A', TransformClass::Identity, &fonts), None);
}
