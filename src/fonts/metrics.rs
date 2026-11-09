use ttf_parser::Face;

use crate::model::{FontProcessingState, FontSource};

/// Advances in em units, measured from processed metrics first and source cmap
/// metrics second. A missing metric stays unknown rather than becoming zero.
pub(crate) struct FontMetrics<'a> {
    source: &'a FontSource,
    face: Option<Face<'a>>,
}

impl<'a> FontMetrics<'a> {
    pub(crate) fn new(source: &'a FontSource) -> Self {
        Self { source, face: source.data.as_deref().and_then(|data| Face::parse(data, 0).ok()) }
    }

    pub(crate) fn advance(&self, character: char) -> Option<f32> {
        if let FontProcessingState::Ready(processed) = &self.source.processing
            && processed.units_per_em > 0
            && let Some(&advance) = processed.advances_by_unicode.get(&character)
        {
            return Some(f32::from(advance) / f32::from(processed.units_per_em));
        }
        let face = self.face.as_ref()?;
        let glyph = face.glyph_index(character)?;
        Some(f32::from(face.glyph_hor_advance(glyph)?) / f32::from(face.units_per_em()))
    }
}
