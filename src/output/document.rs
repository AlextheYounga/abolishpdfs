use std::{fs, io, path::Path};

use crate::model::{
    Color, DocumentModel, FontCatalog, Glyph, Link, LinkTarget, OutlineItem, PageModel, ReconstructionDecision,
    TextObject, TextRenderMode,
};
use crate::text::projection::{self, css_number};

const DOCUMENT_CSS: &str = r#":root { color-scheme: light; }
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; background: #666; }
body { padding: 24px; }
.document { display: flex; flex-direction: column; align-items: center; gap: 24px; }
.page { position: relative; overflow: hidden; background: white; box-shadow: 0 2px 12px #2228; }
.page-content { position: absolute; inset: 0; overflow: hidden; transform-origin: 0 0; }
.text-glyph { position: absolute; white-space: pre; transform-origin: left bottom; }
.page-background { position: absolute; inset: 0; width: 100%; height: 100%; }
.page-link { position: absolute; z-index: 2; }
@media print {
  .page { break-after: page; break-inside: avoid; box-shadow: none; }
}
"#;

pub struct HtmlWriter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlDocument {
    pub index_html: String,
    pub document_css: String,
    pub assets: Vec<(String, Vec<u8>)>,
}

impl HtmlWriter {
    pub fn render(model: &DocumentModel) -> HtmlDocument {
        let index_html = render_index(model);
        let mut assets = model
            .pages
            .iter()
            .filter_map(|page| {
                page.background.as_ref().map(|background| (format!("page-{}.png", page.number), background.png.clone()))
            })
            .collect::<Vec<_>>();
        let mut font_css = String::new();
        for (id, font) in &model.fonts.fonts {
            let Some(data) = font.data.as_ref() else {
                continue;
            };
            let extension = font_extension(data);
            font_css.push_str(&format!(
                "@font-face{{font-family:'pdf-font-{}';src:url('assets/font-{}.{}');}}\n",
                id, id, extension
            ));
            assets.push((format!("font-{}.{}", id, extension), data.clone()));
        }
        HtmlDocument { index_html, document_css: format!("{font_css}{DOCUMENT_CSS}"), assets }
    }

    pub fn write_to(model: &DocumentModel, output: &Path) -> Result<(), OutputError> {
        let rendered = Self::render(model);
        fs::create_dir_all(output.join("assets")).map_err(OutputError::CreateDirectory)?;
        fs::write(output.join("index.html"), rendered.index_html).map_err(OutputError::WriteFile)?;
        fs::write(output.join("document.css"), rendered.document_css).map_err(OutputError::WriteFile)?;
        for (name, asset) in rendered.assets {
            fs::write(output.join("assets").join(name), asset).map_err(OutputError::WriteFile)?;
        }
        Ok(())
    }
}

fn render_index(model: &DocumentModel) -> String {
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>PDF</title><link rel=\"stylesheet\" href=\"document.css\"></head><body>",
    );
    if !model.outlines.is_empty() {
        html.push_str("<nav aria-label=\"Document outline\"><ol>");
        for item in &model.outlines {
            render_outline_item(&mut html, item);
        }
        html.push_str("</ol></nav>");
    }
    html.push_str("<main id=\"page-container\" class=\"document\">");
    for page in &model.pages {
        render_page(&mut html, page, &model.fonts);
    }
    html.push_str("</main></body></html>");
    html
}

fn render_page(html: &mut String, page: &PageModel, fonts: &FontCatalog) {
    let page_width = page.crop_box.right - page.crop_box.left;
    let page_height = page.crop_box.top - page.crop_box.bottom;
    html.push_str(&format!(
        "<section id=\"page-{}\" class=\"page\" data-page-no=\"{}\" aria-label=\"Page {}\" style=\"width:{}px;height:{}px\"><div class=\"page-content\">",
        page.number,
        page.number,
        page.number,
        css_number(page_width),
        css_number(page_height)
    ));
    if page.background.is_some() {
        html.push_str(&format!("<img class=\"page-background\" alt=\"\" src=\"assets/page-{}.png\">", page.number));
    }
    for text_object in &page.text_objects {
        render_text_object(html, page, text_object, fonts);
    }
    for link in &page.links {
        render_link(html, page, link);
    }
    html.push_str("</div></section>");
}

fn render_text_object(html: &mut String, page: &PageModel, text_object: &TextObject, fonts: &FontCatalog) {
    if !matches!(text_object.reconstruction, ReconstructionDecision::NativeText) {
        return;
    }
    for glyph in &text_object.glyphs {
        let Some(unicode) = glyph.unicode else {
            continue;
        };
        let fill = glyph.fill.unwrap_or(Color::BLACK);
        let font_family =
            glyph.font.and_then(|id| fonts.fonts.get(&id).and_then(|font| font.data.as_ref()).map(|_| id));
        let position = match glyph.transform.as_ref() {
            Some(matrix) if !projection::is_identity(matrix) => {
                let projection = projection::project(matrix).unwrap_or_else(|| projection::Projection {
                    scale: 1.0,
                    a: matrix.a,
                    b: matrix.b,
                    c: matrix.c,
                    d: matrix.d,
                });
                format!(
                    "left:{}px;bottom:{}px;{}",
                    css_number(glyph.origin.x - page.crop_box.left),
                    css_number(glyph.origin.y - page.crop_box.bottom),
                    projection.to_css()
                )
            }
            Some(_) => format!(
                "left:{}px;top:{}px;transform:matrix(1,0,0,1,0,0);",
                css_number(placement(page, glyph).left),
                css_number(placement(page, glyph).top)
            ),
            None => format!(
                "left:{}px;top:{}px;",
                css_number(placement(page, glyph).left),
                css_number(placement(page, glyph).top)
            ),
        };
        let family = font_family.map_or_else(|| "sans-serif".to_owned(), |id| format!("'pdf-font-{id}',sans-serif"));
        html.push_str(&format!(
            "<span class=\"text-glyph\" data-source=\"{}\" style=\"{position}font-family:{family};font-size:{}px;color:{};{}\">{}</span>",
            text_object.source,
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
        html.push_str(&format!("<a href=\"#page-{}\">{}</a>", page, escape_html(&item.title)));
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
        LinkTarget::LocalDestination(page) => format!(" href=\"#page-{}\"", page),
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
    match mode {
        TextRenderMode::Stroke | TextRenderMode::FillStroke => {
            glyph.stroke.map(|color| format!("-webkit-text-stroke:1px {};", css_color(color))).unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn font_extension(data: &[u8]) -> &'static str {
    match data.get(..4) {
        Some([0, 1, 0, 0]) => "ttf",
        Some([b'O', b'T', b'T', b'O']) => "otf",
        Some([b't', b't', b'c', b'f']) => "ttc",
        Some([b'w', b'O', b'F', b'F']) => "woff",
        Some([b'w', b'O', b'F', b'2']) => "woff2",
        _ => "bin",
    }
}

fn css_color(color: Color) -> String {
    format!("rgba({}, {}, {}, {:.4})", color.red, color.green, color.blue, color.alpha as f32 / 255.0)
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
#[path = "document_tests.rs"]
mod tests;
