use serde::Serialize;

use super::font::FontId;
use super::geometry::{AffineTransform, Color, Point, Rect};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextObject {
    pub source: usize,
    pub paint_order: usize,
    pub glyphs: Vec<Glyph>,
    pub font: FontId,
    pub render_mode: TextRenderMode,
    pub reconstruction: ReconstructionDecision,
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
    TextFailure(TextFailureReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TextFailureReason {
    MissingUnicode,
    UnprovenFontMapping,
    MissingGeometry,
    UnsupportedRenderMode,
    ExtractionError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextIntegrityFailure {
    pub page: usize,
    pub paint_order: usize,
    pub reason: TextFailureReason,
    pub semantic_text_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_unicode_is_a_text_failure() {
        let decision = ReconstructionDecision::TextFailure(TextFailureReason::MissingUnicode);
        assert_eq!(decision, ReconstructionDecision::TextFailure(TextFailureReason::MissingUnicode));
        assert_ne!(decision, ReconstructionDecision::NativeText);
    }
}
