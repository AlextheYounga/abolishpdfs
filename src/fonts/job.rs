use serde::{Deserialize, Serialize};
use ttf_parser::{Face, GlyphId};

use crate::model::{FontId, FontSource, GlyphIdentity};

#[derive(Debug, Clone)]
pub struct FontJobRequest<'a> {
    pub id: FontId,
    pub source: &'a FontSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontJobSpec {
    pub family_name: String,
    pub records: Vec<GlyphJobRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlyphJobRecord {
    pub unicode: char,
    pub source_glyph_name: Option<String>,
    pub source_code: Option<u32>,
    pub source_glyph_id: Option<u16>,
}

impl FontJobSpec {
    pub fn from_source(source: &FontSource, family_name: String) -> Self {
        let face = source.data.as_deref().and_then(|data| Face::parse(data, 0).ok());
        let mut records = Vec::new();
        for identity in &source.used_identities {
            let candidate = GlyphJobRecord::from_identity(identity, face.as_ref());
            if let Some(existing) =
                records.iter_mut().find(|record: &&mut GlyphJobRecord| record.unicode == candidate.unicode)
            {
                merge_record(existing, candidate);
            } else {
                records.push(candidate);
            }
        }
        if records.is_empty() {
            records = source
                .used_unicode
                .iter()
                .map(|&unicode| GlyphJobRecord {
                    unicode,
                    source_glyph_name: None,
                    source_code: None,
                    source_glyph_id: None,
                })
                .collect();
        }
        if !records.iter().any(|record| record.unicode == ' ')
            && let Some(space) = face.as_ref().and_then(GlyphJobRecord::space)
        {
            records.push(space);
        }
        Self { family_name, records }
    }
}

impl GlyphJobRecord {
    fn from_identity(identity: &GlyphIdentity, face: Option<&Face<'_>>) -> Self {
        Self {
            unicode: identity.unicode,
            source_glyph_name: identity
                .glyph_id
                .and_then(|id| face.and_then(|face| glyph_name(face, id)))
                .map(str::to_owned),
            source_code: identity.source_code,
            source_glyph_id: identity.glyph_id,
        }
    }

    fn space(face: &Face<'_>) -> Option<Self> {
        let glyph = face.glyph_index(' ')?;
        Some(Self {
            unicode: ' ',
            source_glyph_name: glyph_name(face, glyph.0).map(str::to_owned),
            source_code: None,
            source_glyph_id: Some(glyph.0),
        })
    }
}

fn glyph_name<'a>(face: &Face<'a>, id: u16) -> Option<&'a str> {
    face.tables().post.and_then(|post| post.glyph_name(GlyphId(id)))
}

fn merge_record(existing: &mut GlyphJobRecord, candidate: GlyphJobRecord) {
    if existing.source_glyph_name.is_none() {
        existing.source_glyph_name = candidate.source_glyph_name;
    }
    if existing.source_code.is_none() {
        existing.source_code = candidate.source_code;
    }
    if existing.source_glyph_id.is_none() {
        existing.source_glyph_id = candidate.source_glyph_id;
    }
}
