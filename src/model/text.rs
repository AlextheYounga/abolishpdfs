use serde::Serialize;

use super::font::FontId;
use super::geometry::{AffineTransform, Color, Point, Rect};
use super::page::ClipState;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextObject {
    pub source: usize,
    pub paint_order: usize,
    pub glyphs: Vec<Glyph>,
    pub font: FontId,
    pub render_mode: TextRenderMode,
    pub reconstruction: ReconstructionDecision,
    pub visibility: VisibilityDecision,
    pub clipping: ClipState,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Glyph {
    pub unicode: Option<char>,
    pub font: Option<FontId>,
    pub origin: Point,
    pub tight_bounds: Option<Rect>,
    pub loose_bounds: Option<Rect>,
    pub transform: Option<AffineTransform>,
    pub font_size: f32,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub generated_by_pdfium: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TextRenderMode {
    Fill,
    Stroke,
    FillStroke,
    Invisible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ReconstructionDecision {
    NativeText,
    Background(FallbackReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FallbackReason {
    MissingUnicode,
    UnprovenFontMapping,
    MissingGeometry,
    UnsupportedRenderMode,
    ExtractionError,
    CoveredByOpaquePaint,
    AmbiguousVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum VisibilityDecision {
    Visible,
    CoveredByOpaquePaint,
    AmbiguousVisibility,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_unicode_requires_background_fallback() {
        let decision = ReconstructionDecision::Background(FallbackReason::MissingUnicode);
        assert_eq!(decision, ReconstructionDecision::Background(FallbackReason::MissingUnicode));
        assert_ne!(decision, ReconstructionDecision::NativeText);
    }
}
