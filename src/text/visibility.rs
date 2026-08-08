use crate::model::{
    ClipState, FallbackReason, GraphicsKind, GraphicsObject, PaintOpacity, ReconstructionDecision, Rect, TextObject,
    VisibilityDecision,
};

const BOUNDS_EPSILON: f32 = 1e-4;

/// Applies the conservative visibility policy to extracted text objects.
pub fn analyze(text_objects: &mut [TextObject], graphics: &[GraphicsObject]) {
    let mut paint_objects = Vec::new();
    for graphics_object in graphics {
        collect_paint_objects(graphics_object, false, &mut paint_objects);
    }
    paint_objects.sort_by_key(|object| object.paint_order);

    for text_object in text_objects {
        let visibility = text_visibility(text_object, &paint_objects);
        text_object.visibility = visibility;
        if !matches!(text_object.reconstruction, ReconstructionDecision::NativeText) {
            continue;
        }
        text_object.reconstruction = match visibility {
            VisibilityDecision::Visible => ReconstructionDecision::NativeText,
            VisibilityDecision::CoveredByOpaquePaint => {
                ReconstructionDecision::Background(FallbackReason::CoveredByOpaquePaint)
            }
            VisibilityDecision::AmbiguousVisibility => {
                ReconstructionDecision::Background(FallbackReason::AmbiguousVisibility)
            }
        };
    }
}

#[derive(Debug, Clone, Copy)]
struct PaintObject {
    paint_order: usize,
    bounds: Option<Rect>,
    active: Option<bool>,
    opacity: PaintOpacity,
    clipping: ClipState,
}

fn collect_paint_objects(object: &GraphicsObject, inherited_clipping: bool, output: &mut Vec<PaintObject>) {
    let clipping = inherited_clipping || matches!(object.clipping, ClipState::Clipped);
    if matches!(object.kind, GraphicsKind::Form) {
        for child in &object.children {
            collect_paint_objects(child, clipping, output);
        }
        return;
    }
    output.push(PaintObject {
        paint_order: object.paint_order,
        bounds: object.bounds,
        active: object.active,
        opacity: object.opacity,
        clipping: if clipping { ClipState::Clipped } else { ClipState::Unclipped },
    });
    for child in &object.children {
        collect_paint_objects(child, clipping, output);
    }
}

fn text_visibility(text_object: &TextObject, paint_objects: &[PaintObject]) -> VisibilityDecision {
    if matches!(text_object.clipping, ClipState::Clipped) {
        return VisibilityDecision::AmbiguousVisibility;
    }
    let Some(text_bounds) = text_bounds(text_object) else {
        return VisibilityDecision::AmbiguousVisibility;
    };

    for object in paint_objects.iter().filter(|object| object.paint_order > text_object.paint_order) {
        if object.active == Some(false) {
            continue;
        }
        let Some(bounds) = object.bounds else {
            return VisibilityDecision::AmbiguousVisibility;
        };
        if !intersects(text_bounds, bounds) {
            continue;
        }
        if matches!(object.opacity, PaintOpacity::Opaque)
            && matches!(object.clipping, ClipState::Unclipped)
            && contains(bounds, text_bounds)
        {
            return VisibilityDecision::CoveredByOpaquePaint;
        }
        return VisibilityDecision::AmbiguousVisibility;
    }
    VisibilityDecision::Visible
}

fn text_bounds(text_object: &TextObject) -> Option<Rect> {
    text_object.glyphs.iter().filter_map(|glyph| glyph.tight_bounds.or(glyph.loose_bounds)).fold(
        None,
        |bounds, glyph_bounds| {
            Some(match bounds {
                Some(current) => Rect {
                    left: current.left.min(glyph_bounds.left),
                    bottom: current.bottom.min(glyph_bounds.bottom),
                    right: current.right.max(glyph_bounds.right),
                    top: current.top.max(glyph_bounds.top),
                },
                None => glyph_bounds,
            })
        },
    )
}

fn intersects(first: Rect, second: Rect) -> bool {
    first.left < second.right - BOUNDS_EPSILON
        && first.right > second.left + BOUNDS_EPSILON
        && first.bottom < second.top - BOUNDS_EPSILON
        && first.top > second.bottom + BOUNDS_EPSILON
}

fn contains(container: Rect, content: Rect) -> bool {
    container.left <= content.left + BOUNDS_EPSILON
        && container.bottom <= content.bottom + BOUNDS_EPSILON
        && container.right >= content.right - BOUNDS_EPSILON
        && container.top >= content.top - BOUNDS_EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Color, FontId, Glyph, Point, Rect, TextRenderMode};

    fn text(paint_order: usize, bounds: Option<Rect>) -> TextObject {
        TextObject {
            source: paint_order,
            paint_order,
            glyphs: vec![Glyph {
                unicode: Some('A'),
                font: Some(FontId::default()),
                origin: Point { x: 10.0, y: 20.0 },
                tight_bounds: bounds,
                loose_bounds: None,
                transform: None,
                font_size: 12.0,
                fill: Some(Color::BLACK),
                stroke: None,
                generated_by_pdfium: Some(false),
            }],
            font: FontId::default(),
            render_mode: TextRenderMode::Fill,
            reconstruction: ReconstructionDecision::NativeText,
            visibility: VisibilityDecision::Visible,
            clipping: ClipState::Unclipped,
        }
    }

    fn graphics(
        paint_order: usize,
        bounds: Option<Rect>,
        opacity: PaintOpacity,
        clipping: ClipState,
    ) -> GraphicsObject {
        GraphicsObject {
            paint_order,
            kind: GraphicsKind::Path,
            bounds,
            active: Some(true),
            opacity,
            clipping,
            children: Vec::new(),
        }
    }

    fn bounds(left: f32, bottom: f32, right: f32, top: f32) -> Rect {
        Rect { left, bottom, right, top }
    }

    #[test]
    fn only_complete_opaque_containment_marks_text_covered() {
        let mut text_objects = vec![text(1, Some(bounds(10.0, 10.0, 20.0, 20.0)))];
        analyze(
            &mut text_objects,
            &[graphics(2, Some(bounds(0.0, 0.0, 30.0, 30.0)), PaintOpacity::Opaque, ClipState::Unclipped)],
        );
        assert_eq!(text_objects[0].visibility, VisibilityDecision::CoveredByOpaquePaint);
        assert_eq!(
            text_objects[0].reconstruction,
            ReconstructionDecision::Background(FallbackReason::CoveredByOpaquePaint)
        );
    }

    #[test]
    fn partial_overlap_is_ambiguous_not_covered() {
        let mut text_objects = vec![text(1, Some(bounds(10.0, 10.0, 20.0, 20.0)))];
        analyze(
            &mut text_objects,
            &[graphics(2, Some(bounds(15.0, 0.0, 30.0, 30.0)), PaintOpacity::Opaque, ClipState::Unclipped)],
        );
        assert_eq!(text_objects[0].visibility, VisibilityDecision::AmbiguousVisibility);
    }

    #[test]
    fn transparent_and_clipped_paint_is_ambiguous() {
        for (opacity, clipping) in
            [(PaintOpacity::Transparent, ClipState::Unclipped), (PaintOpacity::Opaque, ClipState::Clipped)]
        {
            let mut text_objects = vec![text(1, Some(bounds(10.0, 10.0, 20.0, 20.0)))];
            analyze(&mut text_objects, &[graphics(2, Some(bounds(0.0, 0.0, 30.0, 30.0)), opacity, clipping)]);
            assert_eq!(text_objects[0].visibility, VisibilityDecision::AmbiguousVisibility);
        }
    }

    #[test]
    fn clipped_text_is_not_emitted_as_unrestricted_native_text() {
        let mut text_objects = vec![text(1, Some(bounds(10.0, 10.0, 20.0, 20.0)))];
        text_objects[0].clipping = ClipState::Clipped;
        analyze(&mut text_objects, &[]);
        assert_eq!(text_objects[0].visibility, VisibilityDecision::AmbiguousVisibility);
        assert_eq!(
            text_objects[0].reconstruction,
            ReconstructionDecision::Background(FallbackReason::AmbiguousVisibility)
        );
    }

    #[test]
    fn nested_form_children_keep_paint_order() {
        let mut text_objects = vec![text(1, Some(bounds(10.0, 10.0, 20.0, 20.0)))];
        let form = GraphicsObject {
            paint_order: 2,
            kind: GraphicsKind::Form,
            bounds: None,
            active: Some(true),
            opacity: PaintOpacity::Unknown,
            clipping: ClipState::Unclipped,
            children: vec![graphics(3, Some(bounds(0.0, 0.0, 30.0, 30.0)), PaintOpacity::Opaque, ClipState::Unclipped)],
        };
        analyze(&mut text_objects, &[form]);
        assert_eq!(text_objects[0].visibility, VisibilityDecision::CoveredByOpaquePaint);
    }

    #[test]
    fn missing_bounds_are_reported_as_ambiguous() {
        let mut text_objects = vec![text(1, Some(bounds(10.0, 10.0, 20.0, 20.0)))];
        analyze(&mut text_objects, &[graphics(2, None, PaintOpacity::Unknown, ClipState::Unclipped)]);
        assert_eq!(text_objects[0].visibility, VisibilityDecision::AmbiguousVisibility);
    }

    #[test]
    fn missing_text_bounds_are_ambiguous() {
        let mut text_objects = vec![text(1, None)];
        analyze(&mut text_objects, &[]);
        assert_eq!(text_objects[0].visibility, VisibilityDecision::AmbiguousVisibility);
    }
}
