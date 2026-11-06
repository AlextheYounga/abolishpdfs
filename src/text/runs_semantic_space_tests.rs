use super::*;
use crate::model::{Color, Point};

fn glyph(character: char) -> Glyph {
    Glyph {
        unicode: Some(character),
        source_code: None,
        glyph_id: None,
        font: None,
        origin: Point { x: 0.0, y: 0.0 },
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
fn balanced_offsets_around_a_semantic_space_need_no_html_elements() {
    let first = glyph('A');
    let space = glyph('A');
    let last = glyph('B');
    let characters = [
        RunCharacter { glyph: &first, character: 'A', semantic_space: false },
        RunCharacter { glyph: &space, character: ' ', semantic_space: true },
        RunCharacter { glyph: &last, character: 'B', semantic_space: false },
    ];
    let offsets = vec![RunOffset { character_index: 1, amount: -5.0 }, RunOffset { character_index: 2, amount: 5.0 }];

    assert!(collapse_offsets_around_semantic_spaces(&characters, offsets).is_empty());
}
