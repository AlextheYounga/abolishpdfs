use std::{mem, ops::Range};

use crate::fonts::FontMetrics;
use crate::model::{
    AffineTransform, FontCatalog, FontProcessingState, Glyph, PageModel, PreparedRun, ReconstructionDecision,
    RunOffset, RunPlacement, RunStyle, TextObject,
};

use super::projection::{self, Projection};

mod expanded_source_groups;
use expanded_source_groups::ExpandedSourceGroups;
mod word_spaces;
use word_spaces::reconstruct_word_spaces;

const BASELINE_TOLERANCE: f32 = 0.5;
const FONT_SIZE_RELATIVE_TOLERANCE: f32 = 0.05;
const LAYOUT_GAP_FACTOR: f32 = 2.0;
const GEOMETRY_TOLERANCE: f32 = 1e-3;
const SPACING_TOLERANCE: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
enum TransformClass {
    Identity,
    Projected(Projection),
}

pub fn prepare_page(page: &PageModel, fonts: &FontCatalog) -> Vec<PreparedRun> {
    let mut runs = Vec::new();
    let mut builder = RunBuilder::default();

    for object in &page.text_objects {
        if !matches!(object.reconstruction, ReconstructionDecision::NativeText) {
            if let Some(run) = builder.finish(page, fonts) {
                runs.push(run);
            }
            continue;
        }
        if object.glyphs.is_empty() {
            builder.pending_space = !builder.characters.is_empty();
            continue;
        }

        for glyph in object.glyphs.iter().filter(|glyph| glyph.unicode.is_some()) {
            if builder.continues(glyph, object) {
                if builder.pending_space {
                    builder.push_semantic_space();
                }
                builder.push_glyph(glyph);
            } else {
                if let Some(run) = builder.finish(page, fonts) {
                    runs.push(run);
                }
                builder.start(glyph, object);
            }
            builder.pending_space = false;
        }
    }

    if let Some(run) = builder.finish(page, fonts) {
        runs.push(run);
    }
    runs
}

#[derive(Default)]
struct RunBuilder<'a> {
    object: Option<&'a TextObject>,
    characters: Vec<RunCharacter<'a>>,
    pending_space: bool,
}

#[derive(Clone, Copy)]
pub(super) struct RunCharacter<'a> {
    glyph: &'a Glyph,
    character: char,
    semantic_space: bool,
}

impl<'a> RunBuilder<'a> {
    fn continues(&self, glyph: &Glyph, object: &TextObject) -> bool {
        let Some(first) = self.characters.first() else {
            return false;
        };
        let Some(previous) = self.characters.iter().rev().find(|character| !character.semantic_space) else {
            return false;
        };
        self.object.is_some_and(|first_object| first_object.render_mode == object.render_mode)
            && continues(first.glyph, previous.glyph, glyph)
    }

    fn start(&mut self, glyph: &'a Glyph, object: &'a TextObject) {
        self.object = Some(object);
        self.characters =
            vec![RunCharacter { glyph, character: glyph.unicode.unwrap_or_default(), semantic_space: false }];
    }

    fn push_glyph(&mut self, glyph: &'a Glyph) {
        self.characters.push(RunCharacter {
            glyph,
            character: glyph.unicode.unwrap_or_default(),
            semantic_space: false,
        });
    }

    fn push_semantic_space(&mut self) {
        if let Some(previous) = self.characters.last().copied() {
            self.characters.push(RunCharacter { glyph: previous.glyph, character: ' ', semantic_space: true });
        }
    }

    fn finish(&mut self, page: &PageModel, fonts: &FontCatalog) -> Option<PreparedRun> {
        self.pending_space = false;
        let object = self.object.take()?;
        let characters = mem::take(&mut self.characters);
        Some(build_run(page, object, &characters, fonts))
    }
}

fn continues(first: &Glyph, previous: &Glyph, next: &Glyph) -> bool {
    if !same_style(first, next) {
        return false;
    }

    match (transform_class(first.transform.as_ref()), transform_class(next.transform.as_ref())) {
        (TransformClass::Identity, TransformClass::Identity) => {
            if (next.origin.y - first.origin.y).abs() > BASELINE_TOLERANCE {
                return false;
            }
            let advance = next.origin.x - previous.origin.x;
            advance >= -GEOMETRY_TOLERANCE && advance <= first.font_size * LAYOUT_GAP_FACTOR
        }
        (TransformClass::Projected(first_projection), TransformClass::Projected(next_projection)) => {
            projections_match(first_projection, next_projection)
                && transformed_progression(first, previous, next, first_projection)
        }
        _ => false,
    }
}

fn same_style(first: &Glyph, next: &Glyph) -> bool {
    first.font == next.font
        && (first.font_size - next.font_size).abs() <= first.font_size.abs().max(1.0) * FONT_SIZE_RELATIVE_TOLERANCE
        && first.fill == next.fill
        && first.stroke == next.stroke
        && first.generated_by_pdfium == next.generated_by_pdfium
}

fn transformed_progression(first: &Glyph, previous: &Glyph, next: &Glyph, projection: Projection) -> bool {
    let direction_length = projection.a.hypot(projection.b);
    if direction_length <= GEOMETRY_TOLERANCE {
        return true;
    }
    let direction = (projection.a / direction_length, projection.b / direction_length);
    let delta = (next.origin.x - previous.origin.x, next.origin.y - previous.origin.y);
    let advance = delta.0 * direction.0 + delta.1 * direction.1;
    let cross_track = delta.0 * direction.1 - delta.1 * direction.0;
    advance >= -GEOMETRY_TOLERANCE
        && advance <= first.font_size * LAYOUT_GAP_FACTOR
        && cross_track.abs() <= BASELINE_TOLERANCE
}

fn transform_class(transform: Option<&AffineTransform>) -> TransformClass {
    match transform {
        None => TransformClass::Identity,
        Some(matrix) if projection::is_identity(matrix) => TransformClass::Identity,
        Some(matrix) => TransformClass::Projected(projection::project(matrix).unwrap_or(Projection {
            scale: 1.0,
            a: matrix.a,
            b: matrix.b,
            c: matrix.c,
            d: matrix.d,
        })),
    }
}

fn projections_match(first: Projection, next: Projection) -> bool {
    (first.a - next.a).abs() <= GEOMETRY_TOLERANCE
        && (first.b - next.b).abs() <= GEOMETRY_TOLERANCE
        && (first.c - next.c).abs() <= GEOMETRY_TOLERANCE
        && (first.d - next.d).abs() <= GEOMETRY_TOLERANCE
}

fn build_run(
    page: &PageModel,
    object: &TextObject,
    characters: &[RunCharacter<'_>],
    fonts: &FontCatalog,
) -> PreparedRun {
    let first = characters[0].glyph;
    let font_metrics = first.font.and_then(|id| fonts.fonts.get(&id)).map(FontMetrics::new);
    let transform = transform_class(first.transform.as_ref());
    let reconstructed_characters = reconstruct_word_spaces(characters, transform, font_metrics.as_ref());
    let characters = &reconstructed_characters;
    let expanded_source_groups = ExpandedSourceGroups::identify(characters, |component| {
        has_rendered_component_metric(component.glyph, component.character, fonts)
    });
    let letter_spacing = run_letter_spacing(characters, transform, font_metrics.as_ref(), &expanded_source_groups);
    let placement = match transform {
        TransformClass::Identity => {
            let bounds = first.tight_bounds.or(first.loose_bounds);
            RunPlacement::Bounded {
                left: bounds.map_or(first.origin.x, |value| value.left) - page.crop_box.left,
                top: bounds.map_or(page.crop_box.top - first.origin.y - first.font_size, |value| {
                    page.crop_box.top - value.top
                }),
            }
        }
        TransformClass::Projected(projection) => RunPlacement::Transformed {
            left: first.origin.x - page.crop_box.left,
            bottom: first.origin.y - page.crop_box.bottom,
            matrix: [projection.a, projection.b, projection.c, projection.d],
        },
    };

    PreparedRun {
        source: object.source,
        style: RunStyle {
            font: first.font,
            font_size: first.font_size,
            fill: first.fill,
            stroke: first.stroke,
            render_mode: object.render_mode,
        },
        placement,
        observed_advances: characters
            .windows(2)
            .map(|pair| observed_advance(pair[0].glyph, pair[1].glyph, transform))
            .collect(),
        local_offsets: run_local_offsets(
            characters,
            &expanded_source_groups,
            RunMetrics { transform, font: font_metrics.as_ref(), letter_spacing },
        ),
        letter_spacing,
        text: characters.iter().map(|character| character.character).collect(),
    }
}

fn has_rendered_component_metric(glyph: &Glyph, character: char, fonts: &FontCatalog) -> bool {
    glyph.font.and_then(|font_id| fonts.fonts.get(&font_id)).is_some_and(|font| {
        matches!(&font.processing, FontProcessingState::Ready(processed) if processed.advances_by_unicode.contains_key(&character))
    })
}

fn run_letter_spacing(
    characters: &[RunCharacter<'_>],
    transform: TransformClass,
    font: Option<&FontMetrics<'_>>,
    expanded_source_groups: &ExpandedSourceGroups,
) -> f32 {
    let mut compensations = characters
        .windows(2)
        .enumerate()
        .filter(|(index, _)| !expanded_source_groups.contains_component(*index))
        .filter_map(|(_, pair)| measured_compensation(pair[0], pair[1], transform, font))
        .collect::<Vec<_>>();
    if compensations.is_empty() {
        return 0.0;
    }
    compensations.sort_by(f32::total_cmp);
    compensations[compensations.len() / 2]
}

fn run_local_offsets(
    characters: &[RunCharacter<'_>],
    expanded_source_groups: &ExpandedSourceGroups,
    metrics: RunMetrics<'_>,
) -> Vec<RunOffset> {
    let offsets = characters
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            if expanded_source_groups.contains_internal_transition(index) {
                return None;
            }
            let next_index = index + 1;
            let amount = if let Some(group) = expanded_source_groups.ends_before(next_index) {
                expanded_group_exit_offset(characters, group, pair[1], metrics)?
            } else {
                measured_compensation(pair[0], pair[1], metrics.transform, metrics.font)? - metrics.letter_spacing
            };
            (amount.abs() > SPACING_TOLERANCE).then_some(RunOffset { character_index: index + 1, amount })
        })
        .collect();
    collapse_offsets_around_semantic_spaces(characters, offsets)
}

fn collapse_offsets_around_semantic_spaces(characters: &[RunCharacter<'_>], offsets: Vec<RunOffset>) -> Vec<RunOffset> {
    let semantic_spaces = characters
        .iter()
        .enumerate()
        .filter_map(|(index, character)| character.semantic_space.then_some(index))
        .collect::<Vec<_>>();
    let mut collapsed = Vec::<RunOffset>::new();

    for offset in offsets {
        let character_index = semantic_spaces
            .iter()
            .find(|&&space_index| offset.character_index == space_index || offset.character_index == space_index + 1)
            .map_or(offset.character_index, |space_index| space_index + 1);
        if let Some(existing) = collapsed.iter_mut().find(|existing| existing.character_index == character_index) {
            existing.amount += offset.amount;
        } else {
            collapsed.push(RunOffset { character_index, amount: offset.amount });
        }
    }

    collapsed.retain(|offset| offset.amount.abs() > SPACING_TOLERANCE);
    collapsed.sort_by_key(|offset| offset.character_index);
    collapsed
}

#[derive(Clone, Copy)]
struct RunMetrics<'a> {
    transform: TransformClass,
    font: Option<&'a FontMetrics<'a>>,
    letter_spacing: f32,
}

fn expanded_group_exit_offset(
    characters: &[RunCharacter<'_>],
    group: &Range<usize>,
    next: RunCharacter<'_>,
    metrics: RunMetrics<'_>,
) -> Option<f32> {
    let component_advances = characters[group.clone()]
        .iter()
        .map(|component| natural_advance(component.glyph, component.character, metrics.transform, metrics.font))
        .collect::<Option<Vec<_>>>()?;
    // Each component receives the run's letter spacing before the next physical glyph.
    let rendered_group_advance =
        component_advances.into_iter().sum::<f32>() + metrics.letter_spacing * group.len() as f32;
    Some(observed_advance(characters[group.start].glyph, next.glyph, metrics.transform) - rendered_group_advance)
}

fn measured_compensation(
    previous: RunCharacter<'_>,
    next: RunCharacter<'_>,
    transform: TransformClass,
    font: Option<&FontMetrics<'_>>,
) -> Option<f32> {
    let natural = natural_advance(previous.glyph, previous.character, transform, font)?;
    Some(observed_advance(previous.glyph, next.glyph, transform) - natural)
}

fn natural_advance(
    glyph: &Glyph,
    character: char,
    transform: TransformClass,
    font: Option<&FontMetrics<'_>>,
) -> Option<f32> {
    let horizontal_scale = match transform {
        TransformClass::Identity => 1.0,
        TransformClass::Projected(projection) => projection.a.hypot(projection.b),
    };
    Some(font?.advance(character)? * glyph.font_size * horizontal_scale)
}

fn observed_advance(previous: &Glyph, next: &Glyph, transform: TransformClass) -> f32 {
    let delta = (next.origin.x - previous.origin.x, next.origin.y - previous.origin.y);
    match transform {
        TransformClass::Identity => delta.0,
        TransformClass::Projected(projection) => {
            let length = projection.a.hypot(projection.b);
            if length <= GEOMETRY_TOLERANCE {
                delta.0.hypot(delta.1)
            } else {
                (delta.0 * projection.a + delta.1 * projection.b) / length
            }
        }
    }
}

#[cfg(test)]
#[path = "runs_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "runs_expanded_source_groups_tests.rs"]
mod expanded_source_groups_tests;

#[cfg(test)]
#[path = "runs_semantic_space_tests.rs"]
mod semantic_space_tests;

#[cfg(test)]
#[path = "runs_word_spaces_tests.rs"]
mod word_spaces_tests;
