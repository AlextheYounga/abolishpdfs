use std::{collections::HashMap, path::Path};

use pdfium_render::prelude::*;

use crate::fonts::mapping_is_proven;
use crate::model::{
    AffineTransform, Color, DiagnosticScope, DocumentDiagnostic, DocumentModel, FontCatalog, FontId, FontSource, Glyph,
    GraphicsKind, GraphicsObject, Link, LinkTarget, OutlineItem, PageModel, Point, ReconstructionDecision, Rect, Size,
    TextFailureReason, TextIntegrityFailure, TextObject, TextRenderMode,
};
use crate::text::projection;

use super::PdfiumLibrary;
use super::background::render_page_background;
#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;

pub struct DocumentExtractor;

impl DocumentExtractor {
    pub fn extract(input: &Path, library: &PdfiumLibrary) -> Result<DocumentModel, DocumentExtractionError> {
        let bindings = Pdfium::bind_to_library(library.path()).map_err(DocumentExtractionError::BindLibrary)?;
        let pdfium = Pdfium::new(bindings);
        let document = pdfium.load_pdf_from_file(input, None).map_err(DocumentExtractionError::OpenDocument)?;
        let mut model = DocumentModel {
            pages: Vec::with_capacity(usize::try_from(document.pages().len()).unwrap_or_default()),
            fonts: FontCatalog::new(),
            outlines: Vec::new(),
            diagnostics: Vec::new(),
            text_failures: Vec::new(),
        };

        for index in document.pages().as_range() {
            let page =
                document.pages().get(index).map_err(|source| DocumentExtractionError::LoadPage { index, source })?;
            let page_index = usize::try_from(index).unwrap_or_default();
            let page_model = extract_page(&page, page_index, &mut model);
            model.pages.push(page_model);
        }
        prove_font_mappings(&mut model);
        model.outlines = extract_outlines(&document);

        Ok(model)
    }
}

fn prove_font_mappings(model: &mut DocumentModel) {
    for font in model.fonts.fonts.values_mut() {
        font.mapping_proven = font.data.as_deref().is_some_and(|data| mapping_is_proven(data, &font.used_unicode));
    }
}

fn extract_page(page: &PdfPage<'_>, index: usize, model: &mut DocumentModel) -> PageModel {
    let width = page.width().value;
    let height = page.height().value;
    let crop_box = page
        .boundaries()
        .crop()
        .ok()
        .map_or(Rect { left: 0.0, bottom: 0.0, right: width, top: height }, |boundary| rect(boundary.bounds));
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
    {
        let mut context = ExtractionContext {
            model,
            font_ids: &font_ids,
            text_objects: &mut text_objects,
            next_paint_order: &mut next_paint_order,
            page_number: index + 1,
        };
        for object in page.objects().iter() {
            if let Some(graphics_object) = extract_object(&object, page_text.as_ref(), &mut context) {
                graphics.push(graphics_object);
            }
        }
    }
    prove_font_mappings(model);
    let decisions = text_objects
        .iter()
        .map(|text_object| {
            reconstruction_decision(&text_object.glyphs, text_object.font, text_object.render_mode, model)
        })
        .collect::<Vec<_>>();
    for (text_object, decision) in text_objects.iter_mut().zip(decisions) {
        text_object.reconstruction = decision;
    }
    let failed_text_paint_orders: Vec<usize> = text_objects
        .iter()
        .filter(|text_object| matches!(text_object.reconstruction, ReconstructionDecision::TextFailure(_)))
        .map(|text_object| text_object.paint_order)
        .collect();

    model.text_failures.extend(text_objects.iter().filter_map(|text_object| {
        let ReconstructionDecision::TextFailure(reason) = text_object.reconstruction.clone() else {
            return None;
        };
        Some(TextIntegrityFailure {
            page: index + 1,
            paint_order: text_object.paint_order,
            reason,
            semantic_text_available: text_object.glyphs.iter().all(|glyph| glyph.unicode.is_some()),
        })
    }));

    let background = if needs_raster_background(&failed_text_paint_orders, &graphics) {
        match render_page_background(page, &failed_text_paint_orders) {
            Ok(background) => Some(background),
            Err(error) => {
                model.diagnostics.push(DocumentDiagnostic {
                    scope: DiagnosticScope::Page(index + 1),
                    message: format!("could not render fallback background: {error}"),
                });
                None
            }
        }
    } else {
        None
    };

    let links = page
        .links()
        .iter()
        .filter_map(|link| link.rect().ok().map(|bounds| Link { bounds: rect(bounds), target: link_target(&link) }))
        .collect();

    PageModel { number: index + 1, size: Size { width, height }, crop_box, text_objects, graphics, links, background }
}

fn needs_raster_background(failed_text_paint_orders: &[usize], graphics: &[GraphicsObject]) -> bool {
    !failed_text_paint_orders.is_empty() || !graphics.is_empty()
}

struct ExtractionContext<'a, 'b> {
    model: &'a mut DocumentModel,
    font_ids: &'b HashMap<String, FontId>,
    text_objects: &'a mut Vec<TextObject>,
    next_paint_order: &'a mut usize,
    page_number: usize,
}

fn extract_object(
    object: &PdfPageObject<'_>,
    page_text: Option<&PdfPageText<'_>>,
    context: &mut ExtractionContext<'_, '_>,
) -> Option<GraphicsObject> {
    let paint_order = *context.next_paint_order;
    *context.next_paint_order += 1;

    if let PdfPageObject::Text(text) = object {
        let object_glyphs = match page_text {
            Some(text_page) => match text_page.chars_for_object(text) {
                Ok(characters) => {
                    characters.iter().map(|character| extract_glyph(&character, context)).collect::<Vec<_>>()
                }
                Err(error) => {
                    context.model.diagnostics.push(DocumentDiagnostic {
                        scope: DiagnosticScope::Object { page: context.page_number, paint_order },
                        message: format!("could not extract characters: {error}"),
                    });
                    Vec::new()
                }
            },
            None => Vec::new(),
        };
        let font = object_glyphs.first().and_then(|glyph| glyph.font).unwrap_or_else(|| first_font_id(context.model));
        let render_mode = text_render_mode(text.render_mode());
        let object_transform = text.matrix().ok().map(affine_transform);
        let font_size_scale = object_transform
            .as_ref()
            .and_then(projection::project)
            .map(|projection| projection.scale)
            .filter(|scale| scale.is_finite() && *scale > projection::EPSILON)
            .unwrap_or(1.0);
        let mut object_glyphs = object_glyphs;
        for glyph in &mut object_glyphs {
            glyph.transform = object_transform;
            glyph.font_size *= font_size_scale;
        }
        let reconstruction = reconstruction_decision(&object_glyphs, font, render_mode, context.model);
        context.text_objects.push(TextObject {
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
        PdfPageObject::XObjectForm(form) => {
            form.iter().filter_map(|child| extract_object(&child, page_text, context)).collect()
        }
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

fn extract_glyph(character: &PdfPageTextChar<'_>, context: &mut ExtractionContext<'_, '_>) -> Glyph {
    let font_name = character.font_name();
    if let Some(id) = context.font_ids.get(&font_name)
        && let Some(unicode) = character.unicode_char()
        && let Some(font) = context.model.fonts.fonts.get_mut(id)
        && !font.used_unicode.contains(&unicode)
    {
        font.used_unicode.push(unicode);
    }
    let origin = character.origin().ok();
    Glyph {
        unicode: character.unicode_char(),
        font: context.font_ids.get(&font_name).copied(),
        origin: origin.map_or(Point { x: 0.0, y: 0.0 }, |(x, y)| Point { x: x.value, y: y.value }),
        tight_bounds: character.tight_bounds().ok().map(rect),
        loose_bounds: character.loose_bounds().ok().map(rect),
        transform: None,
        font_size: character.unscaled_font_size().value,
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
        return ReconstructionDecision::TextFailure(TextFailureReason::ExtractionError);
    }
    if glyphs.iter().any(|glyph| glyph.unicode.is_none()) {
        return ReconstructionDecision::TextFailure(TextFailureReason::MissingUnicode);
    }
    if matches!(render_mode, TextRenderMode::Invisible | TextRenderMode::Unknown) {
        return ReconstructionDecision::TextFailure(TextFailureReason::UnsupportedRenderMode);
    }
    if glyphs.iter().any(|glyph| glyph.tight_bounds.is_none() && glyph.loose_bounds.is_none()) {
        return ReconstructionDecision::TextFailure(TextFailureReason::MissingGeometry);
    }
    if model.fonts.fonts.get(&font).is_some_and(|font| font.embedded.unwrap_or(true) && !font.mapping_proven) {
        return ReconstructionDecision::TextFailure(TextFailureReason::UnprovenFontMapping);
    }
    ReconstructionDecision::NativeText
}

fn first_font_id(model: &DocumentModel) -> FontId {
    model.fonts.fonts.keys().next().copied().unwrap_or(0)
}

fn link_target(link: &PdfLink<'_>) -> LinkTarget {
    let Some(action) = link.action() else {
        return link
            .destination()
            .and_then(|destination| destination.page_index().ok())
            .and_then(|index| usize::try_from(index).ok())
            .map_or(LinkTarget::Unknown, |index| LinkTarget::LocalDestination(index + 1));
    };
    if let Some(uri) = action.as_uri_action().and_then(|uri| uri.uri().ok()) {
        return LinkTarget::Uri(uri);
    }
    if let Some(local) = action.as_local_destination_action() {
        local
            .destination()
            .ok()
            .and_then(|destination| destination.page_index().ok())
            .and_then(|index| usize::try_from(index).ok())
            .map_or(LinkTarget::Unknown, |index| LinkTarget::LocalDestination(index + 1))
    } else if action.as_remote_destination_action().is_some() {
        LinkTarget::RemoteDestination
    } else {
        LinkTarget::Unknown
    }
}

fn extract_outlines(document: &PdfDocument<'_>) -> Vec<OutlineItem> {
    let mut outlines = Vec::new();
    let mut bookmark = document.bookmarks().root();
    while let Some(current) = bookmark {
        outlines.push(outline_item(&current));
        bookmark = current.next_sibling();
    }
    outlines
}

fn outline_item(bookmark: &PdfBookmark<'_>) -> OutlineItem {
    let mut children = Vec::new();
    let mut child = bookmark.first_child();
    while let Some(current) = child {
        children.push(outline_item(&current));
        child = current.next_sibling();
    }
    let target_page = bookmark
        .destination()
        .and_then(|destination| destination.page_index().ok())
        .and_then(|index| usize::try_from(index).ok())
        .map(|index| index + 1);
    OutlineItem { title: bookmark.title().unwrap_or_else(|| "Untitled".to_owned()), target_page, children }
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
        PdfPageTextRenderMode::Invisible | PdfPageTextRenderMode::InvisibleClipping => TextRenderMode::Invisible,
        _ => TextRenderMode::Unknown,
    }
}

fn rect(value: PdfRect) -> Rect {
    Rect { left: value.left().value, bottom: value.bottom().value, right: value.right().value, top: value.top().value }
}

fn affine_transform(value: PdfMatrix) -> AffineTransform {
    AffineTransform { a: value.a(), b: value.b(), c: value.c(), d: value.d(), e: value.e(), f: value.f() }
}

fn color(value: PdfColor) -> Color {
    Color { red: value.red(), green: value.green(), blue: value.blue(), alpha: value.alpha() }
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
