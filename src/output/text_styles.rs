use crate::{
    model::{Color, DocumentModel, FontCatalog, FontProcessingState, PreparedRun, TextRenderMode},
    text::{SPACING_MIN_PX, projection::css_number},
};

#[derive(Default)]
pub(super) struct TextStyleCatalog {
    declarations: Vec<String>,
}

impl TextStyleCatalog {
    pub(super) fn from_model(model: &DocumentModel) -> Self {
        let mut catalog = Self::default();
        for run in model.pages.iter().flat_map(|page| &page.prepared_runs) {
            if let Some(declarations) = run_style_declarations(run, &model.fonts)
                && !catalog.declarations.contains(&declarations)
            {
                catalog.declarations.push(declarations);
            }
        }
        catalog
    }

    pub(super) fn index_for(&self, run: &PreparedRun, fonts: &FontCatalog) -> Option<usize> {
        let declarations = run_style_declarations(run, fonts)?;
        self.declarations.iter().position(|candidate| candidate == &declarations)
    }

    pub(super) fn render_css(&self) -> String {
        self.declarations
            .iter()
            .enumerate()
            .map(|(index, declarations)| format!(".text-style-{index} {{ {declarations} }}\n"))
            .collect()
    }
}

fn run_style_declarations(run: &PreparedRun, fonts: &FontCatalog) -> Option<String> {
    let fill = run.style.fill.unwrap_or(Color::BLACK);
    let family = match run.style.font.and_then(|id| fonts.fonts.get(&id)) {
        None => "sans-serif".to_owned(),
        Some(font) => match &font.processing {
            FontProcessingState::NotRequired => "sans-serif".to_owned(),
            FontProcessingState::Ready(processed) => format!("'{}',sans-serif", processed.family_name),
            FontProcessingState::Pending | FontProcessingState::Failed(_) => return None,
        },
    };
    let spacing = if run.letter_spacing.abs() > SPACING_MIN_PX {
        format!("letter-spacing: {}px; ", css_number(run.letter_spacing))
    } else {
        String::new()
    };
    Some(format!(
        "font-family: {family}; font-size: {}px; color: {}; {spacing}{}",
        css_number(run.style.font_size),
        css_color(fill),
        render_mode_style(run.style.render_mode, run.style.stroke)
    ))
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
