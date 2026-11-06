use std::{collections::BTreeMap, fs, path::PathBuf};

use super::*;
use crate::model::{
    Color, FontProcessingState, FontSource, GlyphIdentity, ProcessedFont, RunOffset, RunStyle, TextRenderMode,
};

fn run(text: &str) -> PreparedRun {
    PreparedRun {
        source: 1,
        style: RunStyle {
            font: None,
            font_size: 10.0,
            fill: Some(Color::BLACK),
            stroke: None,
            render_mode: TextRenderMode::Fill,
        },
        placement: RunPlacement::Bounded { left: 0.0, top: 0.0 },
        observed_advances: Vec::new(),
        local_offsets: Vec::new(),
        letter_spacing: 0.0,
        text: text.to_owned(),
    }
}

#[test]
fn word_ranges_follow_unicode_boundaries_around_punctuation_and_whitespace() {
    assert_eq!(word_ranges("Hello,  world!"), vec![0..5, 8..13]);
}

#[test]
fn words_without_positioning_adjustments_remain_plain_text() {
    let run = run("plain words");

    let plans = WordPlan::for_run(&run, &FontCatalog::new());

    assert!(plans.iter().all(|plan| plan.layout == WordLayout::Exact));
}

#[test]
fn uniform_word_error_can_be_represented_by_letter_spacing() {
    let run = run("word");
    let target = [0.0, 4.9, 9.8, 14.7, 19.6];
    let advances = [5.0; 4];

    let spacing = fitted_letter_spacing(&run, &(0..4), &target, &advances);

    assert!(spacing.is_some_and(|spacing| (spacing + 0.1).abs() < 1e-5));
}

#[test]
fn irregular_word_error_keeps_exact_offsets() {
    let run = run("word");
    let target = [0.0, 4.0, 10.0, 14.0, 20.0];
    let advances = [5.0; 4];

    assert_eq!(fitted_letter_spacing(&run, &(0..4), &target, &advances), None);
}

#[test]
fn source_font_shaping_replaces_matching_pair_offsets() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts/LiberationSans-Regular.ttf");
    let Ok(data) = fs::read(path) else { return };
    let Ok(face) = Face::parse(&data, 0) else { return };
    let characters = ['T', 'o'];
    let mut advances = BTreeMap::new();
    let mut identities = Vec::new();
    for character in characters {
        let Some(glyph) = face.glyph_index(character) else { return };
        let Some(advance) = face.glyph_hor_advance(glyph) else { return };
        advances.insert(character, advance);
        identities.push(GlyphIdentity { unicode: character, source_code: None, glyph_id: Some(glyph.0) });
    }
    let units_per_em = face.units_per_em();
    let mut fonts = FontCatalog::new();
    fonts.insert(FontSource {
        name: "Liberation Sans".to_owned(),
        embedded: Some(true),
        data: Some(data),
        used_unicode: characters.to_vec(),
        used_identities: identities,
        mapping_proven: true,
        processing: FontProcessingState::Ready(ProcessedFont {
            asset_name: "fixture.woff2".to_owned(),
            family_name: "fixture".to_owned(),
            data: Vec::new(),
            glyph_count: 2,
            advance_widths: advances.values().copied().collect(),
            units_per_em,
            advances_by_unicode: advances,
        }),
    });
    let mut run = run("To");
    run.style.font = Some(0);
    let Some(natural) = natural_advances(&run, &fonts, &characters, &(0..2)) else { return };
    let Some(shaped) = shaped_positions(&run, &fonts, &characters, &(0..2)) else { return };
    let pair_adjustment = shaped[1] - natural[0];
    assert!(pair_adjustment.abs() > 0.01);
    run.local_offsets.push(RunOffset { character_index: 1, amount: pair_adjustment });

    assert_eq!(choose_layout(&run, &fonts, &characters, &(0..2)), WordLayout::Shaped);
}
