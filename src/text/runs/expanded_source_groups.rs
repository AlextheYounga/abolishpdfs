use std::{collections::BTreeSet, ops::Range};

use super::RunCharacter;

#[derive(Debug, Default)]
pub(super) struct ExpandedSourceGroups {
    ranges: Vec<Range<usize>>,
}

impl ExpandedSourceGroups {
    pub(super) fn identify(
        characters: &[RunCharacter<'_>],
        has_component_advance: impl Fn(RunCharacter<'_>) -> bool,
    ) -> Self {
        let mut ranges = Vec::new();
        let mut start = 0;

        while start + 1 < characters.len() {
            let end = maximal_same_source_end(characters, start);
            if end == start + 1 {
                start += 1;
                continue;
            }

            let group = &characters[start..end];
            if has_semantic_expansion(group) && group.iter().copied().all(&has_component_advance) {
                ranges.push(start..end);
            }
            start = end;
        }

        Self { ranges }
    }

    pub(super) fn contains_component(&self, character_index: usize) -> bool {
        self.ranges.iter().any(|range| range.contains(&character_index))
    }

    pub(super) fn contains_internal_transition(&self, transition_index: usize) -> bool {
        self.ranges.iter().any(|range| range.contains(&transition_index) && transition_index + 1 < range.end)
    }

    pub(super) fn ends_before(&self, character_index: usize) -> Option<&Range<usize>> {
        self.ranges.iter().find(|range| range.end == character_index)
    }
}

fn maximal_same_source_end(characters: &[RunCharacter<'_>], start: usize) -> usize {
    let first = characters[start];
    let mut end = start + 1;
    while end < characters.len() && same_expanded_source(first, characters[end]) {
        end += 1;
    }
    end
}

fn has_semantic_expansion(group: &[RunCharacter<'_>]) -> bool {
    group.iter().map(|component| component.character).collect::<BTreeSet<_>>().len() > 1
}

fn same_expanded_source(first: RunCharacter<'_>, next: RunCharacter<'_>) -> bool {
    !first.semantic_space
        && !next.semantic_space
        && first.glyph.font == next.glyph.font
        && first.glyph.source_code.is_some()
        && first.glyph.source_code == next.glyph.source_code
        && first.glyph.glyph_id.is_some()
        && first.glyph.glyph_id == next.glyph.glyph_id
        && first.glyph.origin == next.glyph.origin
        && first.glyph.tight_bounds == next.glyph.tight_bounds
        && first.glyph.loose_bounds == next.glyph.loose_bounds
        && first.glyph.transform == next.glyph.transform
        && first.glyph.font_size == next.glyph.font_size
        && first.glyph.fill == next.glyph.fill
        && first.glyph.stroke == next.glyph.stroke
        && first.glyph.generated_by_pdfium == next.glyph.generated_by_pdfium
}
