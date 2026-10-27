use std::mem;

use crate::model::{
    AffineTransform, FontCatalog, FontProcessingState, Glyph, PageModel, PreparedRun, ReconstructionDecision,
    RunOffset, RunPlacement, RunStyle, TextObject,
};
use ttf_parser::Face;

use super::projection::{self, Projection};

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
struct RunCharacter<'a> {
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
    let transform = transform_class(first.transform.as_ref());
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
        local_offsets: run_local_offsets(characters, transform, fonts),
        letter_spacing: run_letter_spacing(characters, transform, fonts),
        text: characters.iter().map(|character| character.character).collect(),
    }
}

fn run_letter_spacing(characters: &[RunCharacter<'_>], transform: TransformClass, fonts: &FontCatalog) -> f32 {
    let mut compensations = characters
        .windows(2)
        .filter_map(|pair| measured_compensation(pair[0], pair[1], transform, fonts))
        .collect::<Vec<_>>();
    if compensations.is_empty() {
        return 0.0;
    }
    compensations.sort_by(f32::total_cmp);
    compensations[compensations.len() / 2]
}

fn run_local_offsets(
    characters: &[RunCharacter<'_>],
    transform: TransformClass,
    fonts: &FontCatalog,
) -> Vec<RunOffset> {
    let letter_spacing = run_letter_spacing(characters, transform, fonts);
    characters
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let amount = measured_compensation(pair[0], pair[1], transform, fonts)? - letter_spacing;
            (amount.abs() > SPACING_TOLERANCE).then_some(RunOffset { character_index: index + 1, amount })
        })
        .collect()
}

fn measured_compensation(
    previous: RunCharacter<'_>,
    next: RunCharacter<'_>,
    transform: TransformClass,
    fonts: &FontCatalog,
) -> Option<f32> {
    let natural = natural_advance(previous.glyph, previous.character, transform, fonts)?;
    Some(observed_advance(previous.glyph, next.glyph, transform) - natural)
}

fn natural_advance(glyph: &Glyph, character: char, transform: TransformClass, fonts: &FontCatalog) -> Option<f32> {
    let font_id = glyph.font?;
    let font = fonts.fonts.get(&font_id)?;
    let horizontal_scale = match transform {
        TransformClass::Identity => 1.0,
        TransformClass::Projected(projection) => projection.a.hypot(projection.b),
    };
    if let FontProcessingState::Ready(processed) = &font.processing {
        if let Some(&units) = processed.advances_by_unicode.get(&character) {
            return Some(f32::from(units) / f32::from(processed.units_per_em) * glyph.font_size * horizontal_scale);
        }
    }

    let data = font.data.as_deref()?;
    let face = Face::parse(data, 0).ok()?;
    let glyph_id = face.glyph_index(character)?;
    let advance = f32::from(face.glyph_hor_advance(glyph_id)?);
    Some(advance / f32::from(face.units_per_em()) * glyph.font_size * horizontal_scale)
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
