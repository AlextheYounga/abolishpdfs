use ttf_parser::{Face, GlyphId};

#[derive(Debug, thiserror::Error)]
pub enum PsNameError {
    #[error("font data is too small")]
    DataTooSmall,
    #[error("could not parse font data: {0}")]
    Parse(String),
}

pub fn glyph_name(data: &[u8], glyph_id: u16) -> Result<Option<&str>, PsNameError> {
    if data.len() < 4 {
        return Err(PsNameError::DataTooSmall);
    }
    let face = Face::parse(data, 0).map_err(|error| PsNameError::Parse(error.to_string()))?;
    Ok(face.tables().post.and_then(|post| post.glyph_name(GlyphId(glyph_id))))
}

pub fn glyph_names(data: &[u8]) -> Result<Vec<Option<String>>, PsNameError> {
    if data.len() < 4 {
        return Err(PsNameError::DataTooSmall);
    }
    let face = Face::parse(data, 0).map_err(|error| PsNameError::Parse(error.to_string()))?;
    let count = face.number_of_glyphs();
    Ok((0..count)
        .map(|id| face.tables().post.and_then(|post| post.glyph_name(GlyphId(id))).map(str::to_owned))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn resolves_liberation_sans_names() {
        let data = fs::read("tests/fixtures/fonts/LiberationSans-Regular.ttf").expect("fixture is present");
        let names = glyph_names(&data).expect("fixture parses");
        assert!(names.iter().filter(|name| name.is_some()).count() > 0);
        assert_eq!(glyph_name(&data, 36).expect("fixture parses"), Some("A"));
        assert_eq!(glyph_name(&data, 37).expect("fixture parses"), Some("B"));
    }

    #[test]
    fn rejects_unparsable_data() {
        assert!(matches!(glyph_names(&[0, 1, 2]), Err(PsNameError::DataTooSmall)));
        assert!(matches!(glyph_names(&[0; 32]), Err(PsNameError::Parse(_))));
    }
}
