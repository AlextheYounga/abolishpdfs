use std::collections::BTreeMap;

use super::*;
use crate::model::{Color, FontProcessingState, FontSource, Point, ProcessedFont};

fn glyph(character: char, x: f32) -> Glyph {
    Glyph {
        unicode: Some(character),
        source_code: None,
        glyph_id: None,
        font: Some(0),
        origin: Point { x, y: 80.0 },
        tight_bounds: None,
        loose_bounds: None,
        transform: None,
        font_size: 12.0,
        fill: Some(Color::BLACK),
        stroke: None,
        generated_by_pdfium: Some(false),
    }
}

#[test]
fn natural_space_sized_glyph_gap_reconstructs_a_word_boundary() {
    let processed = ProcessedFont {
        asset_name: "fixture.woff2".to_owned(),
        family_name: "fixture".to_owned(),
        data: Vec::new(),
        glyph_count: 4,
        advance_widths: vec![1000; 4],
        units_per_em: 1000,
        advances_by_unicode: BTreeMap::from([('A', 1000), ('B', 1000), ('C', 1000), (' ', 1000)]),
    };
    let mut fonts = FontCatalog::new();
    fonts.insert(FontSource {
        name: "Fixture".to_owned(),
        embedded: Some(true),
        data: None,
        used_unicode: vec!['A', 'B', 'C', ' '],
        used_identities: Vec::new(),
        mapping_proven: true,
        processing: FontProcessingState::Ready(processed),
    });
    let first = glyph('A', 0.0);
    let second = glyph('B', 24.0);
    let third = glyph('C', 36.0);
    let characters = [
        RunCharacter { glyph: &first, character: 'A', semantic_space: false },
        RunCharacter { glyph: &second, character: 'B', semantic_space: false },
        RunCharacter { glyph: &third, character: 'C', semantic_space: false },
    ];

    let reconstructed = reconstruct_word_spaces(&characters, TransformClass::Identity, &fonts);

    assert_eq!(reconstructed.iter().map(|character| character.character).collect::<String>(), "A BC");
}

#[test]
fn expanded_source_glyph_does_not_create_a_false_word_boundary() {
    let processed = ProcessedFont {
        asset_name: "fixture.woff2".to_owned(),
        family_name: "fixture".to_owned(),
        data: Vec::new(),
        glyph_count: 4,
        advance_widths: vec![1000; 4],
        units_per_em: 1000,
        advances_by_unicode: BTreeMap::from([('f', 500), ('l', 250), ('o', 500), (' ', 250)]),
    };
    let mut fonts = FontCatalog::new();
    fonts.insert(FontSource {
        name: "Fixture".to_owned(),
        embedded: Some(true),
        data: None,
        used_unicode: vec!['f', 'l', 'o', ' '],
        used_identities: Vec::new(),
        mapping_proven: true,
        processing: FontProcessingState::Ready(processed),
    });
    let mut first = glyph('f', 0.0);
    first.source_code = Some(10);
    first.glyph_id = Some(10);
    let mut second = glyph('l', 0.0);
    second.source_code = Some(10);
    second.glyph_id = Some(10);
    let third = glyph('o', 6.0);
    let characters = [
        RunCharacter { glyph: &first, character: 'f', semantic_space: false },
        RunCharacter { glyph: &second, character: 'l', semantic_space: false },
        RunCharacter { glyph: &third, character: 'o', semantic_space: false },
    ];

    let reconstructed = reconstruct_word_spaces(&characters, TransformClass::Identity, &fonts);

    assert_eq!(reconstructed.iter().map(|character| character.character).collect::<String>(), "flo");
}
