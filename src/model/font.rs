use std::collections::BTreeMap;

use serde::Serialize;

pub type FontId = usize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontCatalog {
    pub fonts: BTreeMap<FontId, FontSource>,
}

impl FontCatalog {
    pub fn new() -> Self {
        Self { fonts: BTreeMap::new() }
    }

    pub fn insert(&mut self, font: FontSource) -> FontId {
        let id = self.fonts.len();
        self.fonts.insert(id, font);
        id
    }

    pub fn insert_or_get(&mut self, font: FontSource) -> FontId {
        if let Some((id, _)) = self.fonts.iter().find(|(_, existing)| existing.name == font.name) {
            *id
        } else {
            self.insert(font)
        }
    }
}

impl Default for FontCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FontSource {
    pub name: String,
    pub embedded: Option<bool>,
    pub data: Option<Vec<u8>>,
    pub used_unicode: Vec<char>,
    pub used_identities: Vec<GlyphIdentity>,
    pub mapping_proven: bool,
    pub processing: FontProcessingState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlyphIdentity {
    pub unicode: char,
    pub source_code: Option<u32>,
    pub glyph_id: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FontProcessingState {
    NotRequired,
    Pending,
    Ready(ProcessedFont),
    Failed(FontProcessingFailure),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontProcessingFailure {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessedFont {
    pub asset_name: String,
    pub family_name: String,
    pub data: Vec<u8>,
    pub glyph_count: usize,
    pub advance_widths: Vec<u16>,
    pub units_per_em: u16,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub advances_by_unicode: BTreeMap<char, u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_font_catalog_deduplicates_names() {
        let mut catalog = FontCatalog::new();
        let source = FontSource {
            name: "Example".to_owned(),
            embedded: Some(true),
            data: Some(vec![1, 2, 3]),
            used_unicode: vec!['A'],
            used_identities: Vec::new(),
            mapping_proven: false,
            processing: FontProcessingState::Pending,
        };
        let first = catalog.insert_or_get(source.clone());
        let second = catalog.insert_or_get(source);

        assert_eq!(first, second);
        assert_eq!(catalog.fonts.len(), 1);
    }

    #[test]
    fn font_catalog_accumulates_identity_records() {
        let mut catalog = FontCatalog::new();
        let id = catalog.insert(FontSource {
            name: "Example".to_owned(),
            embedded: Some(true),
            data: None,
            used_unicode: Vec::new(),
            used_identities: Vec::new(),
            mapping_proven: false,
            processing: FontProcessingState::Pending,
        });
        if let Some(font) = catalog.fonts.get_mut(&id) {
            font.used_identities.push(GlyphIdentity { unicode: 'B', source_code: Some(0x41), glyph_id: Some(7) });
        }
        assert_eq!(catalog.fonts[&id].used_identities[0].source_code, Some(0x41));
    }
}
