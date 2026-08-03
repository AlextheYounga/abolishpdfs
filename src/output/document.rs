use std::{fs, io, path::Path};

use crate::model::{
    Color, DocumentModel, Glyph, Link, LinkTarget, OutlineItem, PageModel, ReconstructionDecision, TextObject,
    TextRenderMode,
};

const DOCUMENT_CSS: &str = r#":root { color-scheme: light; }
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; background: #666; }
body { padding: 24px; }
.document { display: flex; flex-direction: column; align-items: center; gap: 24px; }
.page { position: relative; overflow: hidden; background: white; box-shadow: 0 2px 12px #2228; }
.text-glyph { position: absolute; white-space: pre; transform-origin: left bottom; }
.page-background { position: absolute; inset: 0; width: 100%; height: 100%; }
.page-link { position: absolute; z-index: 2; }
.page-frame { border: 0; display: block; }
"#;

pub struct HtmlWriter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlDocument {
    pub index_html: String,
    pub document_css: String,
    pub pages: Vec<(String, String)>,
    pub assets: Vec<(String, Vec<u8>)>,
}

impl HtmlWriter {
    pub fn render(model: &DocumentModel) -> HtmlDocument {
        let pages =
            model.pages.iter().map(|page| (format!("{}.html", page.number), render_page(page))).collect::<Vec<_>>();
        let index_html = render_index(model);
        let assets = model
            .pages
            .iter()
            .filter_map(|page| {
                page.background.as_ref().map(|background| (format!("page-{}.png", page.number), background.png.clone()))
            })
            .collect();
        HtmlDocument { index_html, document_css: DOCUMENT_CSS.to_owned(), pages, assets }
    }

    pub fn write_to(model: &DocumentModel, output: &Path) -> Result<(), OutputError> {
        let rendered = Self::render(model);
        fs::create_dir_all(output.join("pages")).map_err(OutputError::CreateDirectory)?;
        fs::create_dir_all(output.join("assets")).map_err(OutputError::CreateDirectory)?;
        fs::write(output.join("index.html"), rendered.index_html).map_err(OutputError::WriteFile)?;
        fs::write(output.join("document.css"), rendered.document_css).map_err(OutputError::WriteFile)?;
        for (name, page) in rendered.pages {
            fs::write(output.join("pages").join(name), page).map_err(OutputError::WriteFile)?;
        }
        for (name, asset) in rendered.assets {
            fs::write(output.join("assets").join(name), asset).map_err(OutputError::WriteFile)?;
        }
        Ok(())
    }
}

fn render_index(model: &DocumentModel) -> String {
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>PDF</title><link rel=\"stylesheet\" href=\"document.css\"></head><body><main class=\"document\">",
    );
    if !model.outlines.is_empty() {
        html.push_str("<nav aria-label=\"Document outline\"><ol>");
        for item in &model.outlines {
            render_outline_item(&mut html, item);
        }
        html.push_str("</ol></nav>");
    }
    for page in &model.pages {
        let page_width = page.crop_box.right - page.crop_box.left;
        let page_height = page.crop_box.top - page.crop_box.bottom;
        html.push_str(&format!(
            "<iframe class=\"page-frame\" title=\"Page {}\" src=\"pages/{}.html\" style=\"width:{}px;height:{}px\"></iframe>",
            page.number,
            page.number,
            css_number(page_width),
            css_number(page_height)
        ));
    }
    html.push_str("</main></body></html>");
    html
}

fn render_page(page: &PageModel) -> String {
    let page_width = page.crop_box.right - page.crop_box.left;
    let page_height = page.crop_box.top - page.crop_box.bottom;
    let mut html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Page {}</title><link rel=\"stylesheet\" href=\"../document.css\"></head><body><main class=\"document\"><section class=\"page\" aria-label=\"Page {}\" style=\"width:{}px;height:{}px\">",
        page.number,
        page.number,
        css_number(page_width),
        css_number(page_height)
    );
    if page.background.is_some() {
        html.push_str(&format!("<img class=\"page-background\" alt=\"\" src=\"../assets/page-{}.png\">", page.number));
    }
    for text_object in &page.text_objects {
        render_text_object(&mut html, page, text_object);
    }
    for link in &page.links {
        render_link(&mut html, page, link);
    }
    html.push_str("</section></main></body></html>");
    html
}

fn render_text_object(html: &mut String, page: &PageModel, text_object: &TextObject) {
    if !matches!(text_object.reconstruction, ReconstructionDecision::NativeText) {
        return;
    }
    for glyph in &text_object.glyphs {
        let Some(unicode) = glyph.unicode else {
            continue;
        };
        let placement = placement(page, glyph);
        let fill = glyph.fill.unwrap_or(Color::BLACK);
        html.push_str(&format!(
            "<span class=\"text-glyph\" data-source=\"{}\" style=\"left:{}px;top:{}px;font-size:{}px;color:{};{}\">{}</span>",
            text_object.source,
            css_number(placement.left),
            css_number(placement.top),
            css_number(glyph.font_size),
            css_color(fill),
            render_mode_style(text_object.render_mode, glyph),
            escape_html(&unicode.to_string())
        ));
    }
}

fn render_outline_item(html: &mut String, item: &OutlineItem) {
    html.push_str("<li>");
    if let Some(page) = item.target_page {
        html.push_str(&format!("<a href=\"pages/{}.html\">{}</a>", page, escape_html(&item.title)));
    } else {
        html.push_str(&escape_html(&item.title));
    }
    if !item.children.is_empty() {
        html.push_str("<ol>");
        for child in &item.children {
            render_outline_item(html, child);
        }
        html.push_str("</ol>");
    }
    html.push_str("</li>");
}

fn render_link(html: &mut String, page: &PageModel, link: &Link) {
    let left = link.bounds.left - page.crop_box.left;
    let top = page.crop_box.top - link.bounds.top;
    let width = link.bounds.right - link.bounds.left;
    let height = link.bounds.top - link.bounds.bottom;
    let target = match &link.target {
        LinkTarget::Uri(uri) if is_safe_uri(uri) => format!(" href=\"{}\"", escape_html(uri)),
        LinkTarget::LocalDestination(page) => format!(" href=\"../pages/{}.html\"", page),
        _ => String::new(),
    };
    let target_kind = link_target_kind(&link.target);
    html.push_str(&format!(
        "<a class=\"page-link\" aria-label=\"PDF link\" data-target=\"{}\"{} style=\"left:{}px;top:{}px;width:{}px;height:{}px\"></a>",
        target_kind,
        target,
        css_number(left),
        css_number(top),
        css_number(width),
        css_number(height)
    ));
}

fn link_target_kind(target: &LinkTarget) -> &'static str {
    match target {
        LinkTarget::Uri(_) => "uri",
        LinkTarget::LocalDestination(_) => "local",
        LinkTarget::RemoteDestination => "remote",
        LinkTarget::Unknown => "unknown",
    }
}

fn is_safe_uri(uri: &str) -> bool {
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "mailto" | "tel")
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    left: f32,
    top: f32,
}

fn placement(page: &PageModel, glyph: &Glyph) -> Placement {
    let bounds = glyph.tight_bounds.or(glyph.loose_bounds);
    let left = bounds.map_or(glyph.origin.x, |bounds| bounds.left) - page.crop_box.left;
    let top =
        bounds.map_or(page.crop_box.top - glyph.origin.y - glyph.font_size, |bounds| page.crop_box.top - bounds.top);
    Placement { left, top }
}

fn render_mode_style(mode: TextRenderMode, glyph: &Glyph) -> String {
    let transform = glyph
        .transform
        .map(|matrix| {
            format!(
                "transform:matrix({},{},{},{},0,0);",
                css_number(matrix.a),
                css_number(matrix.b),
                css_number(matrix.c),
                css_number(matrix.d)
            )
        })
        .unwrap_or_default();
    match mode {
        TextRenderMode::Stroke | TextRenderMode::FillStroke => format!(
            "font-family:sans-serif;{}{}",
            transform,
            glyph.stroke.map(|color| format!("-webkit-text-stroke:1px {};", css_color(color))).unwrap_or_default()
        ),
        _ => format!("font-family:sans-serif;{transform}"),
    }
}

fn css_color(color: Color) -> String {
    format!("rgba({}, {}, {}, {:.4})", color.red, color.green, color.blue, color.alpha as f32 / 255.0)
}

fn css_number(value: f32) -> String {
    format!("{value:.4}").trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("could not create output directory: {0}")]
    CreateDirectory(io::Error),
    #[error("could not write output file: {0}")]
    WriteFile(io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AffineTransform, FallbackReason, FontCatalog, Point, RasterBackground, ReconstructionDecision, Rect, Size,
    };

    fn model_with_text(text: &str) -> DocumentModel {
        DocumentModel {
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
                graphics: Vec::new(),
                links: Vec::new(),
                background: None,
            }],
            fonts: FontCatalog::new(),
            outlines: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn writer_converts_pdf_coordinates_to_css_coordinates() {
        let output = HtmlWriter::render(&model_with_text("<"));
        assert!(output.pages[0].1.contains("left:72px;top:68px"));
        assert!(output.pages[0].1.contains("&lt;"));
    }

    #[test]
    fn writer_uses_crop_box_as_page_origin() {
        let mut model = model_with_text("A");
        model.pages[0].crop_box = Rect { left: 36.0, bottom: 36.0, right: 576.0, top: 756.0 };
        let output = HtmlWriter::render(&model);
        assert!(output.pages[0].1.contains("width:540px;height:720px"));
        assert!(output.pages[0].1.contains("left:36px;top:32px"));
    }

    #[test]
    fn writer_does_not_apply_transform_translation_twice() {
        let mut model = model_with_text("A");
        model.pages[0].text_objects[0].glyphs[0].transform =
            Some(AffineTransform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 72.0, f: 720.0 });
        let output = HtmlWriter::render(&model);
        assert!(output.pages[0].1.contains("transform:matrix(1,0,0,1,0,0)"));
    }

    #[test]
    fn fallback_text_is_not_emitted_as_native_text() {
        let mut model = model_with_text("A");
        model.pages[0].text_objects[0].reconstruction =
            ReconstructionDecision::Background(FallbackReason::UnprovenFontMapping);
        let output = HtmlWriter::render(&model);
        assert!(!output.pages[0].1.contains("data-source=\"4\""));
    }

    #[test]
    fn writer_renders_safe_uri_links_at_pdf_coordinates() {
        let mut model = model_with_text("A");
        model.pages[0].links.push(Link {
            bounds: Rect { left: 36.0, bottom: 700.0, right: 144.0, top: 736.0 },
            target: LinkTarget::Uri("https://example.com/?a=1&b=2".to_owned()),
        });

        let output = HtmlWriter::render(&model);

        assert!(output.pages[0].1.contains(
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
        model.outlines.push(OutlineItem {
            title: "Intro & setup".to_owned(),
            target_page: Some(1),
            children: Vec::new(),
        });

        let output = HtmlWriter::render(&model);

        assert!(output.pages[0].1.contains("data-target=\"local\" href=\"../pages/2.html\""));
        assert!(output.index_html.contains("href=\"pages/1.html\">Intro &amp; setup</a>"));
    }

    #[test]
    fn writer_does_not_create_unsafe_uri_navigation() {
        let mut model = model_with_text("A");
        model.pages[0].links.push(Link {
            bounds: Rect { left: 0.0, bottom: 0.0, right: 10.0, top: 10.0 },
            target: LinkTarget::Uri("javascript:alert(1)".to_owned()),
        });

        let output = HtmlWriter::render(&model);

        assert!(output.pages[0].1.contains("data-target=\"uri\" style="));
        assert!(!output.pages[0].1.contains("href=\"javascript:"));
    }

    #[test]
    fn writer_creates_split_page_artifacts() {
        let directory = tempfile::tempdir().unwrap();
        let mut model = model_with_text("A");
        model.pages[0].background = Some(RasterBackground { width: 2, height: 2, png: vec![137, 80, 78, 71] });
        HtmlWriter::write_to(&model, directory.path()).unwrap();

        assert!(directory.path().join("index.html").is_file());
        assert!(directory.path().join("document.css").is_file());
        assert!(directory.path().join("pages/1.html").is_file());
        assert!(directory.path().join("assets/page-1.png").is_file());
        assert!(directory.path().join("assets").is_dir());
        let index = fs::read_to_string(directory.path().join("index.html")).unwrap();
        assert!(index.contains("class=\"page-frame\""));
        let page = fs::read_to_string(directory.path().join("pages/1.html")).unwrap();
        assert!(page.contains("class=\"page-background\""));
    }
}
