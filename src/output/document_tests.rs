use std::fs;

use super::*;
use crate::model::{
    AffineTransform, FallbackReason, FontCatalog, FontSource, Glyph, Point, RasterBackground, ReconstructionDecision,
    Rect, Size, TextObject,
};
use crate::text::prepare;

fn model_with_text(text: &str) -> DocumentModel {
    let mut model = DocumentModel {
        pages: vec![PageModel {
            number: 1,
            size: Size { width: 612.0, height: 792.0 },
            crop_box: Rect { left: 0.0, bottom: 0.0, right: 612.0, top: 792.0 },
            text_objects: vec![TextObject {
                source: 4,
                paint_order: 4,
                glyphs: vec![Glyph {
                    unicode: text.chars().next(),
                    font: None,
                    origin: Point { x: 72.0, y: 720.0 },
                    tight_bounds: Some(Rect { left: 72.0, bottom: 710.0, right: 84.0, top: 724.0 }),
                    loose_bounds: None,
                    transform: None,
                    font_size: 12.0,
                    fill: Some(Color::BLACK),
                    stroke: None,
                    generated_by_pdfium: Some(false),
                }],
                font: 0,
                render_mode: TextRenderMode::Fill,
                reconstruction: ReconstructionDecision::NativeText,
            }],
            prepared_runs: Vec::new(),
            graphics: Vec::new(),
            links: Vec::new(),
            background: None,
        }],
        fonts: FontCatalog::new(),
        outlines: Vec::new(),
        diagnostics: Vec::new(),
    };
    prepare(&mut model);
    model
}

#[test]
fn writer_converts_pdf_coordinates_to_css_coordinates() {
    let output = HtmlWriter::render(&model_with_text("<"));
    assert!(output.index_html.contains("left:72px;top:68px"));
    assert!(output.index_html.contains("&lt;"));
}

#[test]
fn writer_uses_crop_box_as_page_origin() {
    let mut model = model_with_text("A");
    model.pages[0].crop_box = Rect { left: 36.0, bottom: 36.0, right: 576.0, top: 756.0 };
    prepare(&mut model);
    let output = HtmlWriter::render(&model);
    assert!(output.index_html.contains("width:540px;height:720px"));
    assert!(output.index_html.contains("left:36px;top:32px"));
}

#[test]
fn writer_does_not_apply_transform_translation_twice() {
    let mut model = model_with_text("A");
    model.pages[0].text_objects[0].glyphs[0].transform =
        Some(AffineTransform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 72.0, f: 720.0 });
    prepare(&mut model);
    let output = HtmlWriter::render(&model);
    assert!(!output.index_html.contains("transform:matrix"));
}

#[test]
fn writer_rotates_and_sizes_transformed_text_on_baseline() {
    let mut model = model_with_text("R");
    model.pages[0].text_objects[0].glyphs[0].font_size = 24.0;
    model.pages[0].text_objects[0].glyphs[0].origin = Point { x: 220.0, y: 420.0 };
    model.pages[0].text_objects[0].glyphs[0].transform =
        Some(AffineTransform { a: 0.0, b: 1.0, c: -1.0, d: 0.0, e: 220.0, f: 420.0 });
    prepare(&mut model);
    let output = HtmlWriter::render(&model);
    let page = &output.index_html;
    assert!(
        page.contains("left:220px;bottom:420px;transform:matrix(0,-1,1,0,0,0);font-family:sans-serif;font-size:24px")
    );
}

#[test]
fn writer_emits_axis_aligned_glyph_without_transform_matrix() {
    let output = HtmlWriter::render(&model_with_text("A"));
    let page = &output.index_html;
    assert!(page.contains("left:72px;top:68px;font-family:sans-serif;font-size:12px"));
    assert!(!page.contains("transform:matrix"));
}

#[test]
fn writer_serializes_prepared_run_text_and_spacing() {
    let mut model = model_with_text("A");
    let mut second = model.pages[0].text_objects[0].glyphs[0].clone();
    second.unicode = Some('B');
    second.origin.x += 12.0;
    second.tight_bounds = Some(Rect { left: 84.0, bottom: 710.0, right: 96.0, top: 724.0 });
    model.pages[0].text_objects[0].glyphs.push(second);
    prepare(&mut model);
    assert_eq!(model.pages[0].prepared_runs.len(), 1);
    model.pages[0].prepared_runs[0].letter_spacing = 1.25;

    let output = HtmlWriter::render(&model);
    assert_eq!(output.index_html.matches("class=\"text-run\"").count(), 1);
    assert!(output.index_html.contains(">AB</span>"));
    assert!(output.index_html.contains("letter-spacing:1.25px;"));
}

#[test]
fn fallback_text_is_not_emitted_as_native_text() {
    let mut model = model_with_text("A");
    model.pages[0].text_objects[0].reconstruction =
        ReconstructionDecision::Background(FallbackReason::UnprovenFontMapping);
    prepare(&mut model);
    let output = HtmlWriter::render(&model);
    assert!(!output.index_html.contains("data-source=\"4\""));
}

#[test]
fn writer_renders_safe_uri_links_at_pdf_coordinates() {
    let mut model = model_with_text("A");
    model.pages[0].links.push(Link {
        bounds: Rect { left: 36.0, bottom: 700.0, right: 144.0, top: 736.0 },
        target: LinkTarget::Uri("https://example.com/?a=1&b=2".to_owned()),
    });
    let output = HtmlWriter::render(&model);
    assert!(output.index_html.contains(
        "class=\"page-link\" aria-label=\"PDF link\" data-target=\"uri\" href=\"https://example.com/?a=1&amp;b=2\" style=\"left:36px;top:56px;width:108px;height:36px\""
    ));
}

#[test]
fn writer_renders_local_links_and_outline_navigation() {
    let mut model = model_with_text("A");
    model.pages[0].links.push(Link {
        bounds: Rect { left: 0.0, bottom: 0.0, right: 10.0, top: 10.0 },
        target: LinkTarget::LocalDestination(2),
    });
    model.outlines.push(OutlineItem { title: "Intro & setup".to_owned(), target_page: Some(1), children: Vec::new() });
    let output = HtmlWriter::render(&model);
    assert!(output.index_html.contains("data-target=\"local\" href=\"#page-2\""));
    assert!(output.index_html.contains("href=\"#page-1\">Intro &amp; setup</a>"));
}

#[test]
fn writer_embeds_fonts_and_assigns_native_glyphs() {
    let mut model = model_with_text("A");
    model.pages[0].text_objects[0].glyphs[0].font = Some(0);
    model.fonts.insert(FontSource {
        name: "Embedded".to_owned(),
        embedded: Some(true),
        data: Some(vec![0, 1, 0, 0, 1]),
        used_unicode: vec!['A'],
        mapping_proven: true,
    });
    prepare(&mut model);
    let output = HtmlWriter::render(&model);
    assert!(output.document_css.contains("@font-face{font-family:'pdf-font-0';src:url('assets/font-0.ttf');}"));
    assert!(output.assets.contains(&("font-0.ttf".to_owned(), vec![0, 1, 0, 0, 1])));
    assert!(output.index_html.contains("font-family:'pdf-font-0',sans-serif"));
}

#[test]
fn writer_does_not_create_unsafe_uri_navigation() {
    let mut model = model_with_text("A");
    model.pages[0].links.push(Link {
        bounds: Rect { left: 0.0, bottom: 0.0, right: 10.0, top: 10.0 },
        target: LinkTarget::Uri("javascript:alert(1)".to_owned()),
    });
    let output = HtmlWriter::render(&model);
    assert!(output.index_html.contains("data-target=\"uri\" style="));
    assert!(!output.index_html.contains("href=\"javascript:"));
}

#[test]
fn writer_creates_single_document_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let mut model = model_with_text("A");
    model.pages[0].background = Some(RasterBackground { width: 2, height: 2, png: vec![137, 80, 78, 71] });
    HtmlWriter::write_to(&model, directory.path()).unwrap();
    assert!(directory.path().join("index.html").is_file());
    assert!(directory.path().join("document.css").is_file());
    assert!(!directory.path().join("pages").exists());
    assert!(directory.path().join("assets/page-1.png").is_file());
    assert!(directory.path().join("assets").is_dir());
    let index = fs::read_to_string(directory.path().join("index.html")).unwrap();
    assert!(!index.contains("<iframe"));
    assert!(index.contains("id=\"page-1\""));
    assert!(index.contains("class=\"page-background\" alt=\"\" src=\"assets/page-1.png\""));
}

#[test]
fn writer_emits_all_pages_in_document_order() {
    let mut model = model_with_text("A");
    model.pages.push(PageModel { number: 2, ..model.pages[0].clone() });
    let output = HtmlWriter::render(&model);

    let first_page = output.index_html.find("id=\"page-1\"").unwrap();
    let second_page = output.index_html.find("id=\"page-2\"").unwrap();
    assert!(first_page < second_page);
    assert_eq!(output.index_html.matches("class=\"page\"").count(), 2);
}
