use crate::fonts::FontMetrics;

use super::{RunCharacter, TransformClass, natural_advance, observed_advance};

const INFERRED_SPACE_MIN_FACTOR: f32 = 0.75;
const INFERRED_SPACE_MAX_FACTOR: f32 = 1.75;

pub(super) fn reconstruct_word_spaces<'a>(
    characters: &[RunCharacter<'a>],
    transform: TransformClass,
    font: Option<&FontMetrics<'_>>,
) -> Vec<RunCharacter<'a>> {
    let mut reconstructed = Vec::with_capacity(characters.len());
    for (index, pair) in characters.windows(2).enumerate() {
        reconstructed.push(pair[0]);
        let follows_shared_source_glyph = index > 0 && shares_source_glyph(characters[index - 1], pair[0]);
        if !follows_shared_source_glyph && should_infer_space(pair[0], pair[1], transform, font) {
            reconstructed.push(RunCharacter { glyph: pair[0].glyph, character: ' ', semantic_space: true });
        }
    }
    if let Some(last) = characters.last() {
        reconstructed.push(*last);
    }
    reconstructed
}

fn shares_source_glyph(first: RunCharacter<'_>, next: RunCharacter<'_>) -> bool {
    first.glyph.source_code.is_some()
        && first.glyph.source_code == next.glyph.source_code
        && first.glyph.glyph_id.is_some()
        && first.glyph.glyph_id == next.glyph.glyph_id
        && first.glyph.origin == next.glyph.origin
        && first.glyph.tight_bounds == next.glyph.tight_bounds
}

fn should_infer_space(
    previous: RunCharacter<'_>,
    next: RunCharacter<'_>,
    transform: TransformClass,
    font: Option<&FontMetrics<'_>>,
) -> bool {
    if previous.semantic_space
        || next.semantic_space
        || previous.character.is_whitespace()
        || next.character.is_whitespace()
    {
        return false;
    }
    let Some(character_advance) = natural_advance(previous.glyph, previous.character, transform, font) else {
        return false;
    };
    let Some(space_advance) = natural_advance(previous.glyph, ' ', transform, font) else {
        return false;
    };
    let gap = observed_advance(previous.glyph, next.glyph, transform) - character_advance;
    gap >= space_advance * INFERRED_SPACE_MIN_FACTOR && gap <= space_advance * INFERRED_SPACE_MAX_FACTOR
}
