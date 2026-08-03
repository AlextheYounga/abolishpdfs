use std::path::Path;

use pdfium_render::prelude::*;
use thiserror::Error;

use super::PdfiumLibrary;

use crate::pdfium::report::{
    CapabilityCharacter, CapabilityPage, CapabilityReport, FontCapability, FontMappingCapability, ObjectCapability,
    TextObjectRenderingCapability,
};

pub struct CapabilityProbe;

impl CapabilityProbe {
    pub fn inspect(input: &Path, library: &PdfiumLibrary) -> Result<CapabilityReport, CapabilityProbeError> {
        let bindings = Pdfium::bind_to_library(library.path()).map_err(CapabilityProbeError::BindLibrary)?;
        let pdfium = Pdfium::new(bindings);
        let document = pdfium.load_pdf_from_file(input, None).map_err(CapabilityProbeError::OpenDocument)?;

        let mut pages = Vec::with_capacity(usize::try_from(document.pages().len()).unwrap_or_default());
        for index in document.pages().as_range() {
            let page =
                document.pages().get(index).map_err(|source| CapabilityProbeError::LoadPage { index, source })?;
            let text = page.text().map_err(|source| CapabilityProbeError::LoadText { index, source })?;
            let characters =
                text.chars().iter().map(|character| CapabilityCharacter::from_pdfium(&character)).collect();
            let text_object_count = page.objects().iter().filter(|object| object.as_text_object().is_some()).count();
            let fonts = page.fonts().into_iter().map(|font| FontCapability::from_pdfium(&font)).collect();
            let mut next_paint_order = 0;
            let objects = page
                .objects()
                .iter()
                .map(|object| ObjectCapability::from_pdfium(&object, &mut next_paint_order))
                .collect();
            let text_object_rendering = TextObjectRenderingCapability::inspect(&page);

            pages.push(CapabilityPage {
                number: usize::try_from(index).unwrap_or_default() + 1,
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
