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
    let crop_box = page
        .boundaries()
        .crop()
        .ok()
        .map(|boundary| rect(boundary.bounds))
        .unwrap_or(Rect {
            left: 0.0,
            bottom: 0.0,
            right: width,
            top: height,
        });
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
    if page_text.is_none() {
        model.diagnostics.push(DocumentDiagnostic {
            scope: DiagnosticScope::Page(index + 1),
            message: "could not load page text; text objects are retained as background".to_owned(),
        });
    }

    let mut text_objects = Vec::new();
    let mut graphics = Vec::new();
    let mut next_paint_order = 0;
    for object in page.objects().iter() {
        if let Some(graphics_object) = extract_object(
            &object,
            page_text.as_ref(),
            model,
            &font_ids,
            &mut text_objects,
            &mut next_paint_order,
            index + 1,
        ) {
            graphics.push(graphics_object);
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

fn extract_object(
    object: &PdfPageObject<'_>,
    page_text: Option<&PdfPageText<'_>>,
    model: &mut DocumentModel,
    font_ids: &HashMap<String, FontId>,
    text_objects: &mut Vec<TextObject>,
    next_paint_order: &mut usize,
    page_number: usize,
) -> Option<GraphicsObject> {
    let paint_order = *next_paint_order;
    *next_paint_order += 1;

    if let PdfPageObject::Text(text) = object {
        let object_glyphs = match page_text {
            Some(text_page) => match text_page.chars_for_object(text) {
                Ok(characters) => characters
                    .iter()
                    .map(|character| extract_glyph(&character, model, font_ids))
                    .collect::<Vec<_>>(),
                Err(error) => {
                    model.diagnostics.push(DocumentDiagnostic {
                        scope: DiagnosticScope::Object {
                            page: page_number,
                            paint_order,
                        },
                        message: format!("could not extract characters: {error}"),
                    });
                    Vec::new()
                }
            },
            None => Vec::new(),
        };
        let font = object_glyphs
            .first()
            .and_then(|glyph| glyph.font)
            .unwrap_or_else(|| first_font_id(model));
        let render_mode = text_render_mode(text.render_mode());
        let object_transform = text.matrix().ok().map(affine_transform);
        let mut object_glyphs = object_glyphs;
        for glyph in &mut object_glyphs {
            glyph.transform = object_transform;
        }
        let reconstruction = reconstruction_decision(&object_glyphs, font, render_mode, model);
        text_objects.push(TextObject {
            source: paint_order,
            paint_order,
            glyphs: object_glyphs,
            font,
            render_mode,
            reconstruction,
        });
        return None;
    }

    let children = match object {
        PdfPageObject::XObjectForm(form) => form
            .iter()
            .filter_map(|child| {
                extract_object(
                    &child,
                    page_text,
                    model,
                    font_ids,
                    text_objects,
                    next_paint_order,
                    page_number,
                )
            })
            .collect(),
        _ => Vec::new(),
    };

    Some(GraphicsObject {
        paint_order,
        kind: graphics_kind(object),
        bounds: object.bounds().ok().map(|bounds| rect(bounds.to_rect())),
        active: object.is_active().ok(),
        children,
    })
}

fn extract_glyph(
    character: &PdfPageTextChar<'_>,
    model: &mut DocumentModel,
    font_ids: &HashMap<String, FontId>,
) -> Glyph {
    let font_name = character.font_name();
    if let Some(id) = font_ids.get(&font_name)
        && let Some(unicode) = character.unicode_char()
        && let Some(font) = model.fonts.fonts.get_mut(id)
        && !font.used_unicode.contains(&unicode)
    {
        font.used_unicode.push(unicode);
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
    render_mode: TextRenderMode,
    model: &DocumentModel,
) -> ReconstructionDecision {
    if glyphs.is_empty() {
        return ReconstructionDecision::Background(FallbackReason::ExtractionError);
    }
    if glyphs.iter().any(|glyph| glyph.unicode.is_none()) {
        return ReconstructionDecision::Background(FallbackReason::MissingUnicode);
    }
    if matches!(
        render_mode,
        TextRenderMode::Invisible | TextRenderMode::Unknown
    ) {
        return ReconstructionDecision::Background(FallbackReason::UnsupportedRenderMode);
    }
    if glyphs
        .iter()
        .any(|glyph| glyph.tight_bounds.is_none() && glyph.loose_bounds.is_none())
    {
        return ReconstructionDecision::Background(FallbackReason::MissingGeometry);
    }
    if model
        .fonts
        .fonts
        .get(&font)
        .is_some_and(|font| font.embedded.unwrap_or(true) && !font.mapping_proven)
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

fn affine_transform(value: PdfMatrix) -> crate::model::AffineTransform {
    crate::model::AffineTransform {
        a: value.a(),
        b: value.b(),
        c: value.c(),
        d: value.d(),
        e: value.e(),
        f: value.f(),
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
