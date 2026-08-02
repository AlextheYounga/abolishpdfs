use std::{collections::HashMap, path::Path};

use pdfium_render::prelude::*;

use crate::model::{
    Color, DiagnosticScope, DocumentDiagnostic, DocumentModel, FallbackReason, FontCatalog, FontId,
    FontSource, Glyph, GraphicsKind, GraphicsObject, Link, LinkTarget, PageModel, Point,
    ReconstructionDecision, Rect, Size, TextObject, TextRenderMode,
};

use super::PdfiumLibrary;

pub struct DocumentExtractor;

impl DocumentExtractor {
    pub fn extract(
        input: &Path,
        library: &PdfiumLibrary,
    ) -> Result<DocumentModel, DocumentExtractionError> {
        let bindings = Pdfium::bind_to_library(library.path())
            .map_err(DocumentExtractionError::BindLibrary)?;
        let pdfium = Pdfium::new(bindings);
        let document = pdfium
            .load_pdf_from_file(input, None)
            .map_err(DocumentExtractionError::OpenDocument)?;
        let mut model = DocumentModel {
            pages: Vec::with_capacity(document.pages().len() as usize),
            fonts: FontCatalog::new(),
            diagnostics: Vec::new(),
        };

        for index in document.pages().as_range() {
            let page = document
                .pages()
                .get(index)
                .map_err(|source| DocumentExtractionError::LoadPage { index, source })?;
            let page_model = extract_page(&page, index as usize, &mut model);
            model.pages.push(page_model);
        }

        Ok(model)
    }
}

fn extract_page(page: &PdfPage<'_>, index: usize, model: &mut DocumentModel) -> PageModel {
    let width = page.width().value;
    let height = page.height().value;
    let crop_box = Rect {
        left: 0.0,
        bottom: 0.0,
        right: width,
        top: height,
    };
    let mut font_ids = HashMap::new();

    for font in page.fonts() {
        let id = model.fonts.insert_or_get(FontSource {
            name: font.name(),
            embedded: font.is_embedded().ok(),
            data: font.data().ok(),
            used_unicode: Vec::new(),
            mapping_proven: false,
        });
        font_ids.insert(font.name(), id);
    }

    let page_text = page.text().ok();
    let glyphs = page_text
        .as_ref()
        .map(|text| {
            text.chars()
                .iter()
                .map(|character| extract_glyph(&character, model, &font_ids))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if page_text.is_none() {
        model.diagnostics.push(DocumentDiagnostic {
            scope: DiagnosticScope::Page(index + 1),
            message: "could not load page text; text objects are retained as background".to_owned(),
        });
    }

    let mut text_offset = 0;
    let mut text_objects = Vec::new();
    let mut graphics = Vec::new();
    for (paint_order, object) in page.objects().iter().enumerate() {
        match &object {
            PdfPageObject::Text(text) => {
                let source_text = text.text();
                let count = source_text.chars().count();
                let object_glyphs = glyphs
                    .iter()
                    .skip(text_offset)
                    .take(count)
                    .cloned()
                    .collect::<Vec<_>>();
                text_offset += count;
                let font = first_font_id(model);
                let font = object_glyphs
                    .first()
                    .and_then(|glyph| glyph.font)
                    .unwrap_or(font);
                let reconstruction = reconstruction_decision(&object_glyphs, font, model);
                text_objects.push(TextObject {
                    source: paint_order,
                    paint_order,
                    glyphs: object_glyphs,
                    font,
                    render_mode: text_render_mode(text.render_mode()),
                    reconstruction,
                });
            }
            _ => graphics.push(GraphicsObject {
                paint_order,
                kind: graphics_kind(&object),
                bounds: object.bounds().ok().map(|bounds| rect(bounds.to_rect())),
                active: object.is_active().ok(),
            }),
        }
    }

    let links = page
        .links()
        .iter()
        .filter_map(|link| {
            link.rect().ok().map(|bounds| Link {
                bounds: rect(bounds),
                target: link_target(&link),
            })
        })
        .collect();

    PageModel {
        number: index + 1,
        size: Size { width, height },
        crop_box,
        text_objects,
        graphics,
        links,
    }
}

fn extract_glyph(
    character: &PdfPageTextChar<'_>,
    model: &mut DocumentModel,
    font_ids: &HashMap<String, FontId>,
) -> Glyph {
    let font_name = character.font_name();
    if let Some(id) = font_ids.get(&font_name) {
        if let Some(unicode) = character.unicode_char() {
            if let Some(font) = model.fonts.fonts.get_mut(id) {
                if !font.used_unicode.contains(&unicode) {
                    font.used_unicode.push(unicode);
                }
            }
        }
    }
    let origin = character.origin().ok();
    Glyph {
        unicode: character.unicode_char(),
        font: font_ids.get(&font_name).copied(),
        origin: origin
            .map(|(x, y)| Point {
                x: x.value,
                y: y.value,
            })
            .unwrap_or(Point { x: 0.0, y: 0.0 }),
        tight_bounds: character.tight_bounds().ok().map(rect),
        loose_bounds: character.loose_bounds().ok().map(rect),
        transform: None,
        font_size: character.scaled_font_size().value,
        fill: character.fill_color().ok().map(color),
        stroke: character.stroke_color().ok().map(color),
        generated_by_pdfium: character.is_generated().ok(),
    }
}

fn reconstruction_decision(
    glyphs: &[Glyph],
    font: FontId,
    model: &DocumentModel,
) -> ReconstructionDecision {
    if glyphs.iter().any(|glyph| glyph.unicode.is_none()) {
        return ReconstructionDecision::Background(FallbackReason::MissingUnicode);
    }
    if glyphs
        .iter()
        .any(|glyph| glyph.tight_bounds.is_none() && glyph.loose_bounds.is_none())
    {
        return ReconstructionDecision::Background(FallbackReason::MissingGeometry);
    }
    if !model
        .fonts
        .fonts
        .get(&font)
        .is_some_and(|font| font.mapping_proven)
    {
        return ReconstructionDecision::Background(FallbackReason::UnprovenFontMapping);
    }
    ReconstructionDecision::NativeText
}

fn first_font_id(model: &DocumentModel) -> FontId {
    model.fonts.fonts.keys().next().copied().unwrap_or(0)
}

fn link_target(link: &PdfLink<'_>) -> LinkTarget {
    let Some(action) = link.action() else {
        return if link.destination().is_some() {
            LinkTarget::LocalDestination
        } else {
            LinkTarget::Unknown
        };
    };
    if let Some(uri) = action.as_uri_action().and_then(|uri| uri.uri().ok()) {
        return LinkTarget::Uri(uri);
    }
    if action.as_local_destination_action().is_some() {
        LinkTarget::LocalDestination
    } else if action.as_remote_destination_action().is_some() {
        LinkTarget::RemoteDestination
    } else {
        LinkTarget::Unknown
    }
}

fn graphics_kind(object: &PdfPageObject<'_>) -> GraphicsKind {
    match object {
        PdfPageObject::Path(_) => GraphicsKind::Path,
        PdfPageObject::Image(_) => GraphicsKind::Image,
        PdfPageObject::Shading(_) => GraphicsKind::Shading,
        PdfPageObject::XObjectForm(_) => GraphicsKind::Form,
        PdfPageObject::Unsupported(_) => GraphicsKind::Unsupported,
        PdfPageObject::Text(_) => unreachable!(),
    }
}

fn text_render_mode(mode: PdfPageTextRenderMode) -> TextRenderMode {
    match mode {
        PdfPageTextRenderMode::FilledUnstroked => TextRenderMode::Fill,
        PdfPageTextRenderMode::StrokedUnfilled => TextRenderMode::Stroke,
        PdfPageTextRenderMode::FilledThenStroked => TextRenderMode::FillStroke,
        PdfPageTextRenderMode::Invisible | PdfPageTextRenderMode::InvisibleClipping => {
            TextRenderMode::Invisible
        }
        _ => TextRenderMode::Unknown,
    }
}

fn rect(value: PdfRect) -> Rect {
    Rect {
        left: value.left().value,
        bottom: value.bottom().value,
        right: value.right().value,
        top: value.top().value,
    }
}

fn color(value: PdfColor) -> Color {
    Color {
        red: value.red(),
        green: value.green(),
        blue: value.blue(),
        alpha: value.alpha(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DocumentExtractionError {
    #[error("could not bind PDFium: {0}")]
    BindLibrary(PdfiumError),
    #[error("could not open PDF document: {0}")]
    OpenDocument(PdfiumError),
    #[error("could not load page {index}: {source}")]
    LoadPage { index: i32, source: PdfiumError },
}
