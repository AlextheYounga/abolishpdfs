use crate::model::{
    AffineTransform, FontCatalog, Glyph, PageModel, PreparedRun, ReconstructionDecision, RunOffset, RunPlacement,
    RunStyle, TextObject,
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
    page.text_objects.iter().flat_map(|object| prepare_object(page, object, fonts)).collect()
}

fn prepare_object(page: &PageModel, object: &TextObject, fonts: &FontCatalog) -> Vec<PreparedRun> {
    if !matches!(object.reconstruction, ReconstructionDecision::NativeText) {
        return Vec::new();
    }

    let glyphs = object.glyphs.iter().filter(|glyph| glyph.unicode.is_some()).collect::<Vec<_>>();
    if glyphs.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut start = 0;
    for end in 1..=glyphs.len() {
        if end == glyphs.len() || !continues(&glyphs[start..end], glyphs[end]) {
            runs.push(build_run(page, object, &glyphs[start..end], fonts));
            start = end;
        }
    }
    runs
}

fn continues(run: &[&Glyph], next: &Glyph) -> bool {
    let first = run[0];
    let previous = run[run.len() - 1];
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

fn build_run(page: &PageModel, object: &TextObject, glyphs: &[&Glyph], fonts: &FontCatalog) -> PreparedRun {
    let first = glyphs[0];
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
        observed_advances: glyphs.windows(2).map(|pair| observed_advance(pair[0], pair[1], transform)).collect(),
        local_offsets: run_local_offsets(glyphs, transform, fonts),
        letter_spacing: run_letter_spacing(glyphs, transform, fonts),
        text: glyphs.iter().filter_map(|glyph| glyph.unicode).collect(),
    }
}

fn run_letter_spacing(glyphs: &[&Glyph], transform: TransformClass, fonts: &FontCatalog) -> f32 {
    let mut compensations = glyphs
        .windows(2)
        .filter_map(|pair| measured_compensation(pair[0], pair[1], transform, fonts))
        .collect::<Vec<_>>();
    if compensations.is_empty() {
        return 0.0;
    }
    compensations.sort_by(f32::total_cmp);
    compensations[compensations.len() / 2]
}

fn run_local_offsets(glyphs: &[&Glyph], transform: TransformClass, fonts: &FontCatalog) -> Vec<RunOffset> {
    let letter_spacing = run_letter_spacing(glyphs, transform, fonts);
    glyphs
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let amount = measured_compensation(pair[0], pair[1], transform, fonts)? - letter_spacing;
            (amount.abs() > SPACING_TOLERANCE).then_some(RunOffset { character_index: index + 1, amount })
        })
        .collect()
}

fn measured_compensation(
    previous: &Glyph,
    next: &Glyph,
    transform: TransformClass,
    fonts: &FontCatalog,
) -> Option<f32> {
    let character = previous.unicode?;
    let natural = natural_advance(previous, character, transform, fonts)?;
    Some(observed_advance(previous, next, transform) - natural)
}

fn natural_advance(glyph: &Glyph, character: char, transform: TransformClass, fonts: &FontCatalog) -> Option<f32> {
    let font_id = glyph.font?;
    let data = fonts.fonts.get(&font_id)?.data.as_deref()?;
    let face = Face::parse(data, 0).ok()?;
    let glyph_id = face.glyph_index(character)?;
    let advance = f32::from(face.glyph_hor_advance(glyph_id)?);
    let units_per_em = f32::from(face.units_per_em());
    let horizontal_scale = match transform {
        TransformClass::Identity => 1.0,
        TransformClass::Projected(projection) => projection.a.hypot(projection.b),
    };
    Some(advance / units_per_em * glyph.font_size * horizontal_scale)
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
mod tests {
    use super::*;
    use crate::model::{Color, FallbackReason, Point, Rect, Size, TextRenderMode};

    fn glyph(character: char, x: f32, y: f32) -> Glyph {
        Glyph {
            unicode: Some(character),
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
        }
    }

    fn page(text_object: TextObject) -> PageModel {
        PageModel {
            number: 1,
            size: Size { width: 100.0, height: 100.0 },
            crop_box: Rect { left: 0.0, bottom: 0.0, right: 100.0, top: 100.0 },
            text_objects: vec![text_object],
            prepared_runs: Vec::new(),
            graphics: Vec::new(),
            links: Vec::new(),
            background: None,
        }
    }

    #[test]
    fn compatible_glyphs_form_one_run_in_source_order() {
        let runs =
            prepare_page(&page(object(vec![glyph('A', 10.0, 80.0), glyph('B', 18.0, 80.0)])), &FontCatalog::new());
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "AB");
        assert_eq!(runs[0].observed_advances, vec![8.0]);
    }

    #[test]
    fn baseline_change_starts_a_new_run() {
        let runs =
            prepare_page(&page(object(vec![glyph('A', 10.0, 80.0), glyph('B', 18.0, 82.0)])), &FontCatalog::new());
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn large_horizontal_gap_starts_a_new_run() {
        let runs =
            prepare_page(&page(object(vec![glyph('A', 10.0, 80.0), glyph('B', 40.0, 80.0)])), &FontCatalog::new());
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
    fn fallback_objects_do_not_form_native_runs() {
        let mut text_object = object(vec![glyph('A', 10.0, 80.0)]);
        text_object.reconstruction = ReconstructionDecision::Background(FallbackReason::MissingUnicode);
        assert!(prepare_page(&page(text_object), &FontCatalog::new()).is_empty());
    }
}
