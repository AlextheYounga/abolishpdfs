use pdfium_render::prelude::*;

use crate::model::{ClipState, PaintOpacity};

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
    if object.get_clip_path().is_some() { ClipState::Clipped } else { ClipState::Unclipped }
}
