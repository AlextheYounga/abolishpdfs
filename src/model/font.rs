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
    pub mapping_proven: bool,
    pub processed: Option<ProcessedFont>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessedFont {
    pub asset_name: String,
    pub family_name: String,
    pub data: Vec<u8>,
    pub glyph_count: usize,
    pub advance_widths: Vec<u16>,
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
            mapping_proven: false,
            processed: None,
        };
        let first = catalog.insert_or_get(source.clone());
        let second = catalog.insert_or_get(source);

        assert_eq!(first, second);
        assert_eq!(catalog.fonts.len(), 1);
    }
}
