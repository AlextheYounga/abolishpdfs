use std::collections::BTreeMap;

use super::*;
use crate::model::{
    ClipState, Color, FontSource, Point, ProcessedFont, Rect, Size, TextRenderMode, VisibilityDecision,
};

fn glyph(character: char, x: f32) -> Glyph {
    Glyph {
        unicode: Some(character),
        source_code: None,
        glyph_id: None,
        font: Some(0),
        origin: Point { x, y: 80.0 },
        tight_bounds: Some(Rect { left: x, bottom: 70.0, right: x + 8.0, top: 82.0 }),
        loose_bounds: None,
        transform: None,
        font_size: 12.0,
        fill: Some(Color::BLACK),
        stroke: None,
        generated_by_pdfium: Some(false),
    }
}

fn expanded_component(character: char, source_glyph_id: u16) -> Glyph {
    let mut glyph = glyph(character, 0.0);
    glyph.source_code = Some(u32::from(source_glyph_id));
    glyph.glyph_id = Some(source_glyph_id);
    glyph
}

fn page(glyphs: Vec<Glyph>) -> PageModel {
    PageModel {
        number: 1,
        size: Size { width: 100.0, height: 100.0 },
        crop_box: Rect { left: 0.0, bottom: 0.0, right: 100.0, top: 100.0 },
        text_objects: vec![TextObject {
            source: 4,
            paint_order: 4,
            glyphs,
            font: 0,
            render_mode: TextRenderMode::Fill,
            reconstruction: ReconstructionDecision::NativeText,
            visibility: VisibilityDecision::Visible,
            clipping: ClipState::Unclipped,
        }],
        prepared_runs: Vec::new(),
        graphics: Vec::new(),
        links: Vec::new(),
        background: None,
    }
}

fn catalog(metrics: &[(char, u16)]) -> FontCatalog {
    let mut fonts = FontCatalog::new();
    fonts.insert(FontSource {
        name: "Fixture".to_owned(),
        embedded: Some(true),
        data: None,
        used_unicode: metrics.iter().map(|(character, _)| *character).collect(),
        used_identities: Vec::new(),
        mapping_proven: true,
        processing: FontProcessingState::Ready(ProcessedFont {
            asset_name: "fixture.woff2".to_owned(),
            family_name: "fixture".to_owned(),
            data: Vec::new(),
            glyph_count: metrics.len() + 1,
            advance_widths: metrics.iter().map(|(_, advance)| *advance).collect(),
            units_per_em: 1000,
            advances_by_unicode: metrics.iter().copied().collect::<BTreeMap<_, _>>(),
        }),
    });
    fonts
}

#[test]
fn expanded_ffl_uses_all_component_advances_before_the_following_glyph() {
    let mut following = glyph('o', 9.6);
    following.tight_bounds = Some(Rect { left: 9.6, bottom: 70.0, right: 17.6, top: 82.0 });
    let run = prepare_page(
        &page(vec![
            expanded_component('f', 264),
            expanded_component('f', 264),
            expanded_component('l', 264),
            following,
        ]),
        &catalog(&[('f', 300), ('l', 200), ('o', 400)]),
    );

    assert_eq!(run[0].text, "fflo");
    assert!(run[0].local_offsets.is_empty());
    assert_eq!(run[0].observed_advances, vec![0.0, 0.0, 9.6]);
}

#[test]
fn nonmatching_middle_glyph_does_not_bridge_an_expanded_source_group() {
    let run = prepare_page(
        &page(vec![expanded_component('f', 264), expanded_component('x', 265), expanded_component('l', 264)]),
        &catalog(&[('f', 300), ('x', 400), ('l', 200)]),
    );

    assert_eq!(run[0].text, "fxl");
    assert_eq!(run[0].local_offsets, vec![RunOffset { character_index: 2, amount: -1.2 }]);
}

#[test]
fn identical_same_source_overprint_remains_ordinary_geometry() {
    let run =
        prepare_page(&page(vec![expanded_component('f', 264), expanded_component('f', 264)]), &catalog(&[('f', 300)]));

    assert_eq!(run[0].text, "ff");
    assert!((run[0].letter_spacing + 3.6).abs() < 1e-5);
}

#[test]
fn missing_component_metric_declines_expanded_source_group_treatment() {
    let mut first = glyph('A', 0.0);
    let mut f = expanded_component('f', 264);
    let l = expanded_component('l', 264);
    let mut following = glyph('o', 11.0);
    first.tight_bounds = Some(Rect { left: 0.0, bottom: 70.0, right: 8.0, top: 82.0 });
    f.origin.x = 5.8;
    f.tight_bounds = Some(Rect { left: 5.8, bottom: 70.0, right: 13.8, top: 82.0 });
    let mut l = l;
    l.origin.x = 5.8;
    l.tight_bounds = f.tight_bounds;
    following.tight_bounds = Some(Rect { left: 11.0, bottom: 70.0, right: 19.0, top: 82.0 });
    let run = prepare_page(&page(vec![first, f, l, following]), &catalog(&[('A', 400), ('f', 300), ('o', 400)]));

    assert_eq!(run[0].text, "Aflo");
    assert_eq!(run[0].letter_spacing, 1.0);
    assert_eq!(run[0].local_offsets.len(), 1);
    assert_eq!(run[0].local_offsets[0].character_index, 2);
    assert!((run[0].local_offsets[0].amount + 4.6).abs() < 1e-5);
}
