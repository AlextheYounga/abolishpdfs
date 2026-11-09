use ttf_parser::{Face, RawFace, Tag, cmap};

use crate::model::GlyphIdentity;

/// Proves that every used Unicode value has a nonzero glyph in a Unicode cmap.
pub fn mapping_is_proven(data: &[u8], used: &[char]) -> bool {
    let Some(table) =
        RawFace::parse(data, 0).ok().and_then(|face| face.table(Tag::from_bytes(b"cmap"))).and_then(cmap::Table::parse)
    else {
        return false;
    };
    // Any Unicode subtable may provide coverage: a BMP subtable can contain
    // characters omitted by a supplementary-plane subtable in the same font.
    used.iter().all(|character| {
        (0..table.subtables.len()).filter_map(|index| table.subtables.get(index)).any(|subtable| {
            subtable.is_unicode() && subtable.glyph_index(u32::from(*character)).is_some_and(|glyph| glyph.0 != 0)
        })
    })
}

/// Proves that source glyph identities address real slots, even when the
/// semantic Unicode values intentionally differ from the source cmap.
pub fn identity_mapping_is_proven(data: &[u8], identities: &[GlyphIdentity]) -> bool {
    let Ok(face) = Face::parse(data, 0) else { return false };
    !identities.is_empty()
        && identities
            .iter()
            .all(|identity| identity.glyph_id.is_some_and(|glyph| glyph > 0 && glyph < face.number_of_glyphs()))
}

#[cfg(test)]
#[path = "mapping_tests.rs"]
mod tests;
