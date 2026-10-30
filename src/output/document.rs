use std::{fmt, fs, io, path::Path};

use crate::model::{
    Color, DocumentModel, FontCatalog, FontProcessingState, Link, LinkTarget, OutlineItem, PageModel, PreparedRun,
    RunOffset, RunPlacement, TextIntegrityFailure, TextRenderMode,
};
use crate::text::{SPACING_MIN_PX, projection::css_number};

const DOCUMENT_CSS: &str = r#":root { color-scheme: light; }
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; background: #666; }
body { padding: 24px; }
.document { display: flex; flex-direction: column; align-items: center; gap: 24px; }
.page { position: relative; overflow: hidden; background: white; box-shadow: 0 2px 12px #2228; }
.page-content { position: absolute; inset: 0; overflow: hidden; transform-origin: 0 0; }
.text-run { position: absolute; white-space: pre; transform-origin: left bottom; }
.text-offset { display: inline-block; width: 0; height: 0; }
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
        for font in model.fonts.fonts.values() {
            let FontProcessingState::Ready(processed) = &font.processing else {
                continue;
            };
            font_css.push_str(&format!(
                "@font-face{{font-family:'{}';src:url('assets/{}');}}\n",
                processed.family_name, processed.asset_name
            ));
            assets.push((processed.asset_name.clone(), processed.data.clone()));
        }
        HtmlDocument { index_html, document_css: format!("{font_css}{DOCUMENT_CSS}"), assets }
    }

    pub fn write_to(model: &DocumentModel, output: &Path) -> Result<(), OutputError> {
        let failures = model.discovered_text_failures();
        if !failures.is_empty() {
            return Err(OutputError::TextIntegrity(TextIntegrityError { failures }));
        }
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
    let mut html = String::new();
    render_line(&mut html, 0, "<!doctype html>");
    render_line(&mut html, 0, "<html lang=\"en\">");
    render_line(&mut html, 1, "<head>");
    render_line(&mut html, 2, "<meta charset=\"utf-8\">");
    render_line(&mut html, 2, "<title>PDF</title>");
    render_line(&mut html, 2, "<link rel=\"stylesheet\" href=\"document.css\">");
    render_line(&mut html, 1, "</head>");
    render_line(&mut html, 1, "<body>");
    if !model.outlines.is_empty() {
        render_line(&mut html, 2, "<nav aria-label=\"Document outline\">");
        render_line(&mut html, 3, "<ol>");
        for item in &model.outlines {
            render_outline_item(&mut html, item, 4);
        }
        render_line(&mut html, 3, "</ol>");
        render_line(&mut html, 2, "</nav>");
    }
    render_line(&mut html, 2, "<main id=\"page-container\" class=\"document\">");
    for page in &model.pages {
        render_page(&mut html, page, &model.fonts, 3);
    }
    render_line(&mut html, 2, "</main>");
    render_line(&mut html, 1, "</body>");
    render_line(&mut html, 0, "</html>");
    html
}

fn render_line(html: &mut String, depth: usize, content: &str) {
    html.push_str(&"  ".repeat(depth));
    html.push_str(content);
    html.push('\n');
}

fn render_page(html: &mut String, page: &PageModel, fonts: &FontCatalog, depth: usize) {
    let page_width = page.crop_box.right - page.crop_box.left;
    let page_height = page.crop_box.top - page.crop_box.bottom;
    render_line(
        html,
        depth,
        &format!(
            "<section id=\"page-{}\" class=\"page\" data-page-no=\"{}\" aria-label=\"Page {}\" style=\"width:{}px;height:{}px\">",
            page.number,
            page.number,
            page.number,
            css_number(page_width),
            css_number(page_height)
        ),
    );
    render_line(html, depth + 1, "<div class=\"page-content\">");
    if page.background.is_some() {
        render_line(
            html,
            depth + 2,
            &format!("<img class=\"page-background\" alt=\"\" src=\"assets/page-{}.png\">", page.number),
        );
    }
    for run in &page.prepared_runs {
        render_run(html, run, fonts, depth + 2);
    }
    for link in &page.links {
        render_link(html, page, link, depth + 2);
    }
    render_line(html, depth + 1, "</div>");
    render_line(html, depth, "</section>");
}

fn render_run(html: &mut String, run: &PreparedRun, fonts: &FontCatalog, depth: usize) {
    let fill = run.style.fill.unwrap_or(Color::BLACK);
    let family = match run.style.font.and_then(|id| fonts.fonts.get(&id)) {
        None => "sans-serif".to_owned(),
        Some(font) => match &font.processing {
            FontProcessingState::NotRequired => "sans-serif".to_owned(),
            FontProcessingState::Ready(processed) => format!("'{}',sans-serif", processed.family_name),
            FontProcessingState::Pending | FontProcessingState::Failed(_) => return,
        },
    };
    let position = match run.placement {
        RunPlacement::Bounded { left, top } => format!("left:{}px;top:{}px;", css_number(left), css_number(top)),
        RunPlacement::Transformed { left, bottom, matrix } => format!(
            "left:{}px;bottom:{}px;transform:matrix({},{},{},{},0,0);",
            css_number(left),
            css_number(bottom),
            css_number(matrix[0]),
            css_number(-matrix[1]),
            css_number(-matrix[2]),
            css_number(matrix[3])
        ),
    };
    let spacing = if run.letter_spacing.abs() > SPACING_MIN_PX {
        format!("letter-spacing:{}px;", css_number(run.letter_spacing))
    } else {
        String::new()
    };
    render_line(
        html,
        depth,
        &format!(
            "<span class=\"text-run\" data-source=\"{}\" style=\"{position}font-family:{family};font-size:{}px;color:{};font-kerning:none;font-variant-ligatures:none;{spacing}{}\">{}</span>",
            run.source,
            css_number(run.style.font_size),
            css_color(fill),
            render_mode_style(run.style.render_mode, run.style.stroke),
            render_run_text(&run.text, &run.local_offsets)
        ),
    );
}

fn render_run_text(text: &str, offsets: &[RunOffset]) -> String {
    let mut rendered = String::new();
    let mut offset_index = 0;
    for (character_index, character) in text.chars().enumerate() {
        while let Some(offset) = offsets.get(offset_index).filter(|offset| offset.character_index == character_index) {
            rendered.push_str(&format!(
                "<span class=\"text-offset\" aria-hidden=\"true\" style=\"margin-left:{}px\"></span>",
                css_number(offset.amount)
            ));
            offset_index += 1;
        }
        rendered.push_str(&escape_html(&character.to_string()));
    }
    rendered
}

fn render_outline_item(html: &mut String, item: &OutlineItem, depth: usize) {
    render_line(html, depth, "<li>");
    if let Some(page) = item.target_page {
        render_line(html, depth + 1, &format!("<a href=\"#page-{}\">{}</a>", page, escape_html(&item.title)));
    } else {
        render_line(html, depth + 1, &escape_html(&item.title));
    }
    if !item.children.is_empty() {
        render_line(html, depth + 1, "<ol>");
        for child in &item.children {
            render_outline_item(html, child, depth + 2);
        }
        render_line(html, depth + 1, "</ol>");
    }
    render_line(html, depth, "</li>");
}

fn render_link(html: &mut String, page: &PageModel, link: &Link, depth: usize) {
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
    render_line(
        html,
        depth,
        &format!(
            "<a class=\"page-link\" aria-label=\"PDF link\" data-target=\"{}\"{} style=\"left:{}px;top:{}px;width:{}px;height:{}px\"></a>",
            target_kind,
            target,
            css_number(left),
            css_number(top),
            css_number(width),
            css_number(height)
        ),
    );
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

fn render_mode_style(mode: TextRenderMode, stroke: Option<Color>) -> String {
    match mode {
        TextRenderMode::Stroke | TextRenderMode::FillStroke => {
            stroke.map(|color| format!("-webkit-text-stroke:1px {};", css_color(color))).unwrap_or_default()
        }
        _ => String::new(),
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
    #[error("{0}")]
    TextIntegrity(TextIntegrityError),
    #[error("could not create output directory: {0}")]
    CreateDirectory(io::Error),
    #[error("could not write output file: {0}")]
    WriteFile(io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextIntegrityError {
    pub failures: Vec<TextIntegrityFailure>,
}

impl fmt::Display for TextIntegrityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "native text conversion failed")?;
        for failure in &self.failures {
            write!(
                formatter,
                "; page {} object {}: {:?} (semantic text available: {})",
                failure.page, failure.paint_order, failure.reason, failure.semantic_text_available
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
