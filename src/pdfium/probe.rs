use std::path::Path;

use pdfium_render::prelude::*;
use serde::Serialize;
use thiserror::Error;

use super::PdfiumLibrary;

pub struct CapabilityProbe;

impl CapabilityProbe {
    pub fn inspect(
        input: &Path,
        library: &PdfiumLibrary,
    ) -> Result<CapabilityReport, CapabilityProbeError> {
        let bindings =
            Pdfium::bind_to_library(library.path()).map_err(CapabilityProbeError::BindLibrary)?;
        let pdfium = Pdfium::new(bindings);
        let document = pdfium
            .load_pdf_from_file(input, None)
            .map_err(CapabilityProbeError::OpenDocument)?;

        let mut pages = Vec::with_capacity(document.pages().len() as usize);
        for index in document.pages().as_range() {
            let page = document
                .pages()
                .get(index)
                .map_err(|source| CapabilityProbeError::LoadPage { index, source })?;
            let text = page
                .text()
                .map_err(|source| CapabilityProbeError::LoadText { index, source })?;
            let characters = text
                .chars()
                .iter()
                .map(CapabilityCharacter::from_pdfium)
                .collect();
            let text_object_count = page
                .objects()
                .iter()
                .filter(|object| object.as_text_object().is_some())
                .count();
            let fonts = page
                .fonts()
                .into_iter()
                .map(FontCapability::from_pdfium)
                .collect();
            let mut next_paint_order = 0;
            let objects = page
                .objects()
                .iter()
                .map(|object| ObjectCapability::from_pdfium(object, &mut next_paint_order))
                .collect();
            let text_object_rendering = TextObjectRenderingCapability::inspect(&page);

            pages.push(CapabilityPage {
                number: index as usize + 1,
                width_points: page.width().value,
                height_points: page.height().value,
                page_object_count: page.objects().len(),
                text_object_count,
                characters,
                fonts,
                objects,
                text_object_rendering,
            });
        }

        Ok(CapabilityReport {
            pdfium_library: library.path().display().to_string(),
            pdfium_bindings: "pdfium_7881",
            page_count: pages.len(),
            font_mapping: FontMappingCapability::for_pdfium_render_0_9_3(),
            pages,
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CapabilityReport {
    pub pdfium_library: String,
    pub pdfium_bindings: &'static str,
    pub page_count: usize,
    pub font_mapping: FontMappingCapability,
    pub pages: Vec<CapabilityPage>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CapabilityPage {
    pub number: usize,
    pub width_points: f32,
    pub height_points: f32,
    pub page_object_count: usize,
    pub text_object_count: usize,
    pub characters: Vec<CapabilityCharacter>,
    pub fonts: Vec<FontCapability>,
    pub objects: Vec<ObjectCapability>,
    pub text_object_rendering: TextObjectRenderingCapability,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CapabilityCharacter {
    pub index: usize,
    pub unicode: Option<char>,
    pub generated: Option<bool>,
    pub generated_error: Option<String>,
    pub font_name: String,
    pub text_object_available: bool,
    pub origin_available: bool,
    pub tight_bounds_available: bool,
    pub loose_bounds_available: bool,
}

impl CapabilityCharacter {
    fn from_pdfium(character: PdfPageTextChar<'_>) -> Self {
        let (generated, generated_error) = match character.is_generated() {
            Ok(generated) => (Some(generated), None),
            Err(error) => (None, Some(error.to_string())),
        };

        Self {
            index: character.index(),
            unicode: character.unicode_char(),
            generated,
            generated_error,
            font_name: character.font_name(),
            text_object_available: character.text_object().is_ok(),
            origin_available: character.origin().is_ok(),
            tight_bounds_available: character.tight_bounds().is_ok(),
            loose_bounds_available: character.loose_bounds().is_ok(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FontCapability {
    pub name: String,
    pub embedded: Option<bool>,
    pub embedded_error: Option<String>,
    pub data_length: Option<usize>,
    pub data_error: Option<String>,
}

impl FontCapability {
    fn from_pdfium(font: PdfFont<'_>) -> Self {
        let (embedded, embedded_error) = match font.is_embedded() {
            Ok(embedded) => (Some(embedded), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let (data_length, data_error) = match font.data() {
            Ok(data) => (Some(data.len()), None),
            Err(error) => (None, Some(error.to_string())),
        };

        Self {
            name: font.name(),
            embedded,
            embedded_error,
            data_length,
            data_error,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ObjectCapability {
    pub paint_order: usize,
    pub kind: &'static str,
    pub active: Option<bool>,
    pub active_error: Option<String>,
    pub children: Vec<ObjectCapability>,
}

impl ObjectCapability {
    fn from_pdfium(object: PdfPageObject<'_>, next_paint_order: &mut usize) -> Self {
        let paint_order = *next_paint_order;
        *next_paint_order += 1;
        let (active, active_error) = match object.is_active() {
            Ok(active) => (Some(active), None),
            Err(error) => (None, Some(error.to_string())),
        };

        let (kind, children) = match &object {
            PdfPageObject::Text(_) => ("text", Vec::new()),
            PdfPageObject::Path(_) => ("path", Vec::new()),
            PdfPageObject::Image(_) => ("image", Vec::new()),
            PdfPageObject::Shading(_) => ("shading", Vec::new()),
            PdfPageObject::Unsupported(_) => ("unsupported", Vec::new()),
            PdfPageObject::XObjectForm(form) => (
                "form",
                form.iter()
                    .map(|object| Self::from_pdfium(object, next_paint_order))
                    .collect(),
            ),
        };

        Self {
            paint_order,
            kind,
            active,
            active_error,
            children,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TextObjectRenderingCapability {
    pub text_object_count: usize,
    pub baseline_checksum: Option<u64>,
    pub suppressed_checksum: Option<u64>,
    pub restored_checksum: Option<u64>,
    pub suppression_changed_bitmap: Option<bool>,
    pub restoration_matches_baseline: Option<bool>,
    pub deactivation_error: Option<String>,
    pub reactivation_error: Option<String>,
    pub render_error: Option<String>,
}

impl TextObjectRenderingCapability {
    fn inspect(page: &PdfPage<'_>) -> Self {
        let mut result = Self::empty();
        let baseline_checksum = match bitmap_checksum(page) {
            Ok(checksum) => checksum,
            Err(error) => {
                result.render_error = Some(format!("baseline render: {error}"));
                return result;
            }
        };
        result.baseline_checksum = Some(baseline_checksum);

        let mut text_objects: Vec<_> = page
            .objects()
            .iter()
            .filter(|object| object.as_text_object().is_some())
            .collect();
        result.text_object_count = text_objects.len();
        let mut originally_active = Vec::with_capacity(text_objects.len());

        for object in &mut text_objects {
            match object.is_active() {
                Ok(active) => {
                    originally_active.push(active);
                    if active {
                        if let Err(error) = object.set_inactive() {
                            result.deactivation_error = Some(error.to_string());
                        }
                    }
                }
                Err(error) => {
                    originally_active.push(false);
                    result.deactivation_error = Some(error.to_string());
                }
            }
        }

        match bitmap_checksum(page) {
            Ok(checksum) => {
                result.suppressed_checksum = Some(checksum);
                result.suppression_changed_bitmap = Some(checksum != baseline_checksum);
            }
            Err(error) => result.render_error = Some(format!("suppressed render: {error}")),
        }

        for (object, was_active) in text_objects.iter_mut().zip(originally_active) {
            if was_active {
                if let Err(error) = object.set_active() {
                    result.reactivation_error = Some(error.to_string());
                }
            }
        }

        match bitmap_checksum(page) {
            Ok(checksum) => {
                result.restored_checksum = Some(checksum);
                result.restoration_matches_baseline = Some(checksum == baseline_checksum);
            }
            Err(error) => result.render_error = Some(format!("restored render: {error}")),
        }

        result
    }

    fn empty() -> Self {
        Self {
            text_object_count: 0,
            baseline_checksum: None,
            suppressed_checksum: None,
            restored_checksum: None,
            suppression_changed_bitmap: None,
            restoration_matches_baseline: None,
            deactivation_error: None,
            reactivation_error: None,
            render_error: None,
        }
    }
}

fn bitmap_checksum(page: &PdfPage<'_>) -> Result<u64, PdfiumError> {
    let bitmap = page.render_with_config(&PdfRenderConfig::new().set_target_width(300))?;
    Ok(bitmap
        .as_raw_bytes()
        .into_iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        }))
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FontMappingCapability {
    pub character_to_text_object: bool,
    pub character_to_glyph_id: bool,
    pub character_to_pdf_code: bool,
    pub note: &'static str,
}

impl FontMappingCapability {
    fn for_pdfium_render_0_9_3() -> Self {
        Self {
            character_to_text_object: true,
            character_to_glyph_id: false,
            character_to_pdf_code: false,
            note: "pdfium-render 0.9.3 exposes character-to-text-object association, but not a public character-to-glyph-ID or character-to-PDF-code mapping.",
        }
    }
}

#[derive(Debug, Error)]
pub enum CapabilityProbeError {
    #[error("could not bind PDFium: {0}")]
    BindLibrary(PdfiumError),
    #[error("could not open PDF document: {0}")]
    OpenDocument(PdfiumError),
    #[error("could not load page {index}: {source}")]
    LoadPage { index: i32, source: PdfiumError },
    #[error("could not load text from page {index}: {source}")]
    LoadText { index: i32, source: PdfiumError },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_owned_diagnostic_values() {
        let report = CapabilityReport {
            pdfium_library: "/release/libpdfium.so".to_owned(),
            pdfium_bindings: "pdfium_7881",
            page_count: 1,
            font_mapping: FontMappingCapability::for_pdfium_render_0_9_3(),
            pages: vec![CapabilityPage {
                number: 1,
                width_points: 612.0,
                height_points: 792.0,
                page_object_count: 2,
                text_object_count: 1,
                characters: vec![CapabilityCharacter {
                    index: 0,
                    unicode: Some('A'),
                    generated: Some(false),
                    generated_error: None,
                    font_name: "Helvetica".to_owned(),
                    text_object_available: true,
                    origin_available: true,
                    tight_bounds_available: true,
                    loose_bounds_available: true,
                }],
                fonts: vec![FontCapability {
                    name: "Helvetica".to_owned(),
                    embedded: Some(false),
                    embedded_error: None,
                    data_length: Some(12_345),
                    data_error: None,
                }],
                objects: vec![ObjectCapability {
                    paint_order: 0,
                    kind: "text",
                    active: Some(true),
                    active_error: None,
                    children: Vec::new(),
                }],
                text_object_rendering: TextObjectRenderingCapability {
                    text_object_count: 1,
                    baseline_checksum: Some(1),
                    suppressed_checksum: Some(2),
                    restored_checksum: Some(1),
                    suppression_changed_bitmap: Some(true),
                    restoration_matches_baseline: Some(true),
                    deactivation_error: None,
                    reactivation_error: None,
                    render_error: None,
                },
            }],
        };

        let json = serde_json::to_value(report).unwrap();

        assert_eq!(json["pages"][0]["characters"][0]["unicode"], "A");
        assert_eq!(json["pages"][0]["characters"][0]["generated"], false);
        assert_eq!(json["pages"][0]["fonts"][0]["data_length"], 12_345);
        assert_eq!(json["pages"][0]["objects"][0]["kind"], "text");
        assert_eq!(
            json["pages"][0]["text_object_rendering"]["suppression_changed_bitmap"],
            true
        );
        assert_eq!(json["font_mapping"]["character_to_glyph_id"], false);
    }
}
