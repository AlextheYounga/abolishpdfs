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

            pages.push(CapabilityPage {
                number: index as usize + 1,
                width_points: page.width().value,
                height_points: page.height().value,
                page_object_count: page.objects().len(),
                characters,
            });
        }

        Ok(CapabilityReport {
            pdfium_library: library.path().display().to_string(),
            page_count: pages.len(),
            pages,
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CapabilityReport {
    pub pdfium_library: String,
    pub page_count: usize,
    pub pages: Vec<CapabilityPage>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CapabilityPage {
    pub number: usize,
    pub width_points: f32,
    pub height_points: f32,
    pub page_object_count: usize,
    pub characters: Vec<CapabilityCharacter>,
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
            page_count: 1,
            pages: vec![CapabilityPage {
                number: 1,
                width_points: 612.0,
                height_points: 792.0,
                page_object_count: 2,
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
            }],
        };

        let json = serde_json::to_value(report).unwrap();

        assert_eq!(json["pages"][0]["characters"][0]["unicode"], "A");
        assert_eq!(json["pages"][0]["characters"][0]["generated"], false);
    }
}
