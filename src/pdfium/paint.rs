use pdfium_render::prelude::*;

use crate::model::{ClipState, PaintOpacity, Rect};

const GEOMETRY_EPSILON: f32 = 1e-3;

pub(super) fn paint_opacity(object: &PdfPageObject<'_>) -> PaintOpacity {
    let PdfPageObject::Path(path) = object else {
        return if matches!(object, PdfPageObject::XObjectForm(_)) {
            PaintOpacity::Opaque
        } else {
            PaintOpacity::Unknown
        };
    };

    let has_fill = path.fill_mode().is_ok_and(|mode| !matches!(mode, PdfPathFillMode::None));
    let has_stroke = path.is_stroked().unwrap_or(false);
    let fill_alpha = has_fill.then(|| path.fill_color().ok().map(|color| color.alpha()));
    let stroke_alpha = has_stroke.then(|| path.stroke_color().ok().map(|color| color.alpha()));
    let alphas = [fill_alpha.flatten(), stroke_alpha.flatten()].into_iter().flatten().collect::<Vec<_>>();
    if alphas.is_empty()
        || (has_fill && fill_alpha.is_some_and(|alpha| alpha.is_none()))
        || (has_stroke && stroke_alpha.is_some_and(|alpha| alpha.is_none()))
    {
        return PaintOpacity::Unknown;
    }
    if alphas.iter().any(|alpha| *alpha < 255) { PaintOpacity::Transparent } else { PaintOpacity::Opaque }
}

pub(super) fn clip_state(object: &PdfPageObject<'_>) -> ClipState {
    let object_bounds = object.bounds().ok().map(|bounds| rect(bounds.to_rect()));
    clip_state_with_bounds(object, object_bounds)
}

pub(super) fn clip_state_with_bounds(object: &PdfPageObject<'_>, object_bounds: Option<Rect>) -> ClipState {
    let Some(clip_path) = object.get_clip_path() else {
        return ClipState::Unclipped;
    };
    if clip_path.is_empty() {
        return ClipState::Unclipped;
    }
    let Some(object_bounds) = object_bounds else {
        return ClipState::Clipped;
    };

    let components = clip_path
        .iter()
        .map(|component| {
            rectangular_bounds(&component).map(|clip_bounds| (clip_bounds, contains(clip_bounds, object_bounds)))
        })
        .collect::<Vec<_>>();
    if components.iter().all(|component| component.is_some_and(|(_, contains)| contains)) {
        ClipState::Unclipped
    } else {
        ClipState::Clipped
    }
}

fn rectangular_bounds<'a>(segments: &'a impl PdfPathSegments<'a>) -> Option<Rect> {
    let segments = segments
        .iter()
        .map(|segment| {
            if !matches!(segment.segment_type(), PdfPathSegmentType::MoveTo | PdfPathSegmentType::LineTo) {
                return None;
            }
            Some((segment.x().value, segment.y().value, segment.is_close()))
        })
        .collect::<Option<Vec<_>>>()?;
    if segments.len() < 4 || !segments.last().is_some_and(|segment| segment.2) {
        return None;
    }

    let left = segments.iter().map(|segment| segment.0).reduce(f32::min)?;
    let right = segments.iter().map(|segment| segment.0).reduce(f32::max)?;
    let bottom = segments.iter().map(|segment| segment.1).reduce(f32::min)?;
    let top = segments.iter().map(|segment| segment.1).reduce(f32::max)?;
    if right - left <= GEOMETRY_EPSILON || top - bottom <= GEOMETRY_EPSILON {
        return None;
    }

    let corners = [(left, bottom), (left, top), (right, bottom), (right, top)];
    if segments.iter().any(|segment| {
        !corners
            .iter()
            .any(|corner| approximately_equal(segment.0, corner.0) && approximately_equal(segment.1, corner.1))
    }) {
        return None;
    }
    if corners.iter().any(|corner| {
        !segments
            .iter()
            .any(|segment| approximately_equal(segment.0, corner.0) && approximately_equal(segment.1, corner.1))
    }) {
        return None;
    }
    Some(Rect { left, bottom, right, top })
}

fn approximately_equal(first: f32, second: f32) -> bool {
    (first - second).abs() <= GEOMETRY_EPSILON
}

fn contains(container: Rect, content: Rect) -> bool {
    container.left <= content.left + GEOMETRY_EPSILON
        && container.bottom <= content.bottom + GEOMETRY_EPSILON
        && container.right >= content.right - GEOMETRY_EPSILON
        && container.top >= content.top - GEOMETRY_EPSILON
}

fn rect(value: PdfRect) -> Rect {
    Rect { left: value.left().value, bottom: value.bottom().value, right: value.right().value, top: value.top().value }
}
