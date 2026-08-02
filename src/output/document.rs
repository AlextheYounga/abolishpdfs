use std::{fmt::Write, fs, path::Path};

use crate::model::{Color, DocumentModel, Glyph, PageModel, TextObject, TextRenderMode};

const DOCUMENT_CSS: &str = r#":root { color-scheme: light; }
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; background: #666; }
body { padding: 24px; }
.document { display: flex; flex-direction: column; align-items: center; gap: 24px; }
.page { position: relative; overflow: hidden; background: white; box-shadow: 0 2px 12px #2228; }
.text-object { position: absolute; white-space: pre; transform-origin: left bottom; }
"#;

pub struct HtmlWriter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlDocument {
    pub index_html: String,
    pub document_css: String,
    pub pages: Vec<(String, String)>,
}

impl HtmlWriter {
    pub fn render(model: &DocumentModel) -> HtmlDocument {
        let pages = model
            .pages
            .iter()
            .map(|page| (format!("{}.html", page.number), render_page(page)))
            .collect::<Vec<_>>();
        let index_html = render_index(&model.pages);
        HtmlDocument {
            index_html,
            document_css: DOCUMENT_CSS.to_owned(),
            pages,
        }
    }

    pub fn write_to(model: &DocumentModel, output: &Path) -> Result<(), OutputError> {
        let rendered = Self::render(model);
        fs::create_dir_all(output.join("pages")).map_err(OutputError::CreateDirectory)?;
        fs::create_dir_all(output.join("assets")).map_err(OutputError::CreateDirectory)?;
        fs::write(output.join("index.html"), rendered.index_html)
            .map_err(OutputError::WriteFile)?;
        fs::write(output.join("document.css"), rendered.document_css)
            .map_err(OutputError::WriteFile)?;
        for (name, page) in rendered.pages {
            fs::write(output.join("pages").join(name), page).map_err(OutputError::WriteFile)?;
        }
        Ok(())
    }
}

fn render_index(pages: &[PageModel]) -> String {
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>PDF</title><link rel=\"stylesheet\" href=\"document.css\"></head><body><main class=\"document\">",
    );
    for page in pages {
        write!(
            html,
            "<a href=\"pages/{}.html\">Page {}</a>",
            page.number, page.number
        )
        .unwrap();
    }
    html.push_str("</main></body></html>");
    html
}

fn render_page(page: &PageModel) -> String {
    let mut html = String::new();
    write!(
        html,
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Page {}</title><link rel=\"stylesheet\" href=\"../document.css\"></head><body><main class=\"document\"><section class=\"page\" aria-label=\"Page {}\" style=\"width:{}px;height:{}px\">",
        page.number,
        page.number,
        css_number(page.size.width),
        css_number(page.size.height)
    )
    .unwrap();
    for text_object in &page.text_objects {
        render_text_object(&mut html, page, text_object);
    }
    html.push_str("</section></main></body></html>");
    html
}

fn render_text_object(html: &mut String, page: &PageModel, text_object: &TextObject) {
    if !matches!(
        text_object.reconstruction,
        crate::model::ReconstructionDecision::NativeText
    ) {
        return;
    }
    let text = text_object
        .glyphs
        .iter()
        .filter_map(|glyph| glyph.unicode)
        .collect::<String>();
    let Some(first) = text_object.glyphs.first() else {
        return;
    };
    let placement = placement(page, first);
    let fill = first.fill.unwrap_or(Color::BLACK);
    write!(
        html,
        "<span class=\"text-object\" data-source=\"{}\" style=\"left:{}px;top:{}px;font-size:{}px;color:{};{}\">{}</span>",
        text_object.source,
        css_number(placement.left),
        css_number(placement.top),
        css_number(first.font_size),
        css_color(fill),
        render_mode_style(text_object.render_mode, first),
        escape_html(&text)
    )
    .unwrap();
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    left: f32,
    top: f32,
}

fn placement(page: &PageModel, glyph: &Glyph) -> Placement {
    let bounds = glyph.tight_bounds.or(glyph.loose_bounds);
    let left = bounds.map_or(glyph.origin.x, |bounds| bounds.left);
    let top = bounds.map_or(
        page.size.height - glyph.origin.y - glyph.font_size,
        |bounds| page.size.height - bounds.top,
    );
    Placement { left, top }
}

fn render_mode_style(mode: TextRenderMode, glyph: &Glyph) -> String {
    let transform = glyph
        .transform
        .map(|matrix| {
            format!(
                "transform:matrix({},{},{},{},{},{});",
                css_number(matrix.a),
                css_number(matrix.b),
                css_number(matrix.c),
                css_number(matrix.d),
                css_number(matrix.e),
                css_number(matrix.f)
            )
        })
        .unwrap_or_default();
    match mode {
        TextRenderMode::Stroke | TextRenderMode::FillStroke => format!(
            "font-family:sans-serif;{}{}",
            transform,
            glyph
                .stroke
                .map(|color| format!("-webkit-text-stroke:1px {};", css_color(color)))
                .unwrap_or_default()
        ),
        _ => format!("font-family:sans-serif;{transform}"),
    }
}

fn css_color(color: Color) -> String {
    format!(
        "rgba({}, {}, {}, {:.4})",
        color.red,
        color.green,
        color.blue,
        color.alpha as f32 / 255.0
    )
}

fn css_number(value: f32) -> String {
    format!("{value:.4}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
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
    CreateDirectory(std::io::Error),
    #[error("could not write output file: {0}")]
    WriteFile(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FontCatalog, ReconstructionDecision, Rect, Size};

    fn model_with_text(text: &str) -> DocumentModel {
        DocumentModel {
            pages: vec![PageModel {
                number: 1,
                size: Size {
                    width: 612.0,
                    height: 792.0,
                },
                crop_box: Rect {
                    left: 0.0,
                    bottom: 0.0,
                    right: 612.0,
                    top: 792.0,
                },
                text_objects: vec![TextObject {
                    source: 4,
                    paint_order: 4,
                    glyphs: vec![Glyph {
                        unicode: text.chars().next(),
                        font: None,
                        origin: crate::model::Point { x: 72.0, y: 720.0 },
                        tight_bounds: Some(Rect {
                            left: 72.0,
                            bottom: 710.0,
                            right: 84.0,
                            top: 724.0,
                        }),
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
            }],
            fonts: FontCatalog::new(),
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
    fn fallback_text_is_not_emitted_as_native_text() {
        let mut model = model_with_text("A");
        model.pages[0].text_objects[0].reconstruction =
            ReconstructionDecision::Background(crate::model::FallbackReason::UnprovenFontMapping);
        let output = HtmlWriter::render(&model);
        assert!(!output.pages[0].1.contains("data-source=\"4\""));
    }

    #[test]
    fn writer_creates_split_page_artifacts() {
        let directory = tempfile::tempdir().unwrap();
        HtmlWriter::write_to(&model_with_text("A"), directory.path()).unwrap();

        assert!(directory.path().join("index.html").is_file());
        assert!(directory.path().join("document.css").is_file());
        assert!(directory.path().join("pages/1.html").is_file());
        assert!(directory.path().join("assets").is_dir());
    }
}
