use std::ops::Range;

use rustybuzz::{Feature, UnicodeBuffer};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    fonts::FontMetrics,
    model::{FontCatalog, FontSource, PreparedRun, RunOffset, RunPlacement},
    text::{SPACING_MIN_PX, projection::css_number},
};

use super::document::escape_html;

const POSITION_TOLERANCE: f32 = 0.2;
const SHAPING_TOLERANCE: f32 = 0.1;
const MAX_WORD_LETTER_SPACING: f32 = 0.75;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum WordLayout {
    Exact,
    Shaped,
    LetterSpaced(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct WordPlan {
    pub(super) range: Range<usize>,
    pub(super) layout: WordLayout,
}

impl WordPlan {
    pub(super) fn for_run(run: &PreparedRun, fonts: &FontCatalog) -> Vec<Self> {
        let characters = run.text.chars().collect::<Vec<_>>();
        word_ranges(&run.text)
            .into_iter()
            .map(|range| {
                let layout = choose_layout(run, fonts, &characters, &range);
                Self { range, layout }
            })
            .collect()
    }
}

pub(super) fn render_run_text(run: &PreparedRun, fonts: &FontCatalog) -> String {
    let characters = run.text.chars().collect::<Vec<_>>();
    let words = WordPlan::for_run(run, fonts);
    let exact_context = ExactWordContext { characters: &characters, offsets: &run.local_offsets };
    let mut rendered = String::new();
    let mut offset_index = 0;
    let mut character_index = 0;
    let mut word_index = 0;
    while character_index < characters.len() {
        render_offsets(&mut rendered, &run.local_offsets, &mut offset_index, character_index);
        if let Some(word) = words.get(word_index).filter(|word| word.range.start == character_index) {
            if word.layout == WordLayout::Exact {
                render_exact_word(&mut rendered, &word.range, exact_context, &mut offset_index);
            } else {
                render_word(&mut rendered, &characters[word.range.clone()], word.layout);
                while run.local_offsets.get(offset_index).is_some_and(|offset| offset.character_index < word.range.end)
                {
                    offset_index += 1;
                }
            }
            character_index = word.range.end;
            word_index += 1;
        } else {
            rendered.push_str(&escape_html(&characters[character_index].to_string()));
            character_index += 1;
        }
    }
    rendered
}

fn render_exact_word(
    rendered: &mut String,
    range: &Range<usize>,
    context: ExactWordContext<'_>,
    offset_index: &mut usize,
) {
    for character_index in range.clone() {
        render_offsets(rendered, context.offsets, offset_index, character_index);
        rendered.push_str(&escape_html(&context.characters[character_index].to_string()));
    }
}

fn render_offsets(rendered: &mut String, offsets: &[RunOffset], offset_index: &mut usize, character_index: usize) {
    while let Some(offset) = offsets.get(*offset_index).filter(|offset| offset.character_index == character_index) {
        rendered.push_str(&format!(
            "<span class=\"text-offset\" aria-hidden=\"true\" style=\"margin-left:{}px\"></span>",
            css_number(offset.amount)
        ));
        *offset_index += 1;
    }
}

#[derive(Clone, Copy)]
struct ExactWordContext<'a> {
    characters: &'a [char],
    offsets: &'a [RunOffset],
}

fn render_word(rendered: &mut String, characters: &[char], layout: WordLayout) {
    let text = characters.iter().collect::<String>();
    match layout {
        WordLayout::Exact => rendered.push_str(&escape_html(&text)),
        WordLayout::Shaped => {
            rendered.push_str(&format!("<span class=\"text-word text-word-shaped\">{}</span>", escape_html(&text)));
        }
        WordLayout::LetterSpaced(spacing) => rendered.push_str(&format!(
            "<span class=\"text-word\" style=\"letter-spacing:{}px\">{}</span>",
            css_number(spacing),
            escape_html(&text)
        )),
    }
}

fn word_ranges(text: &str) -> Vec<Range<usize>> {
    text.unicode_word_indices()
        .map(|(byte_index, word)| {
            let start = text[..byte_index].chars().count();
            start..start + word.chars().count()
        })
        .collect()
}

fn choose_layout(run: &PreparedRun, fonts: &FontCatalog, characters: &[char], range: &Range<usize>) -> WordLayout {
    let internal_offsets = run
        .local_offsets
        .iter()
        .filter(|offset| offset.character_index > range.start && offset.character_index < range.end)
        .collect::<Vec<_>>();
    if internal_offsets.is_empty() || range.len() < 2 {
        return WordLayout::Exact;
    }

    let Some(natural_advances) = natural_advances(run, fonts, characters, range) else {
        return WordLayout::Exact;
    };
    let target = target_positions(run, range, &natural_advances);
    if let Some(shaped) = shaped_positions(run, fonts, characters, range)
        && maximum_position_error(&target, &shaped) <= SHAPING_TOLERANCE
    {
        return WordLayout::Shaped;
    }

    fitted_letter_spacing(run, range, &target, &natural_advances).map_or(WordLayout::Exact, WordLayout::LetterSpaced)
}

fn natural_advances(
    run: &PreparedRun,
    fonts: &FontCatalog,
    characters: &[char],
    range: &Range<usize>,
) -> Option<Vec<f32>> {
    let font_id = run.style.font?;
    let font = fonts.fonts.get(&font_id)?;
    let scale = horizontal_scale(run) * run.style.font_size;
    let metrics = FontMetrics::new(font);
    characters[range.clone()]
        .iter()
        .map(|character| metrics.advance(*character).map(|advance| advance * scale))
        .collect()
}

fn target_positions(run: &PreparedRun, range: &Range<usize>, advances: &[f32]) -> Vec<f32> {
    let mut positions = vec![0.0];
    let mut position = 0.0;
    for local_index in 1..=range.len() {
        position += advances[local_index - 1] + run.letter_spacing;
        let global_index = range.start + local_index;
        position += run
            .local_offsets
            .iter()
            .filter(|offset| offset.character_index == global_index && global_index < range.end)
            .map(|offset| offset.amount)
            .sum::<f32>();
        positions.push(position);
    }
    positions
}

fn shaped_positions(
    run: &PreparedRun,
    fonts: &FontCatalog,
    characters: &[char],
    range: &Range<usize>,
) -> Option<Vec<f32>> {
    let font_id = run.style.font?;
    let font = fonts.fonts.get(&font_id)?;
    let data = font.data.as_deref()?;
    let face = rustybuzz::Face::from_slice(data, 0)?;
    let word = &characters[range.clone()];
    let expected_glyphs = expected_glyphs(font, word)?;
    let mut buffer = UnicodeBuffer::new();
    for (index, character) in word.iter().enumerate() {
        buffer.add(*character, u32::try_from(index).ok()?);
    }
    buffer.guess_segment_properties();
    let features = ["-liga".parse::<Feature>().ok()?, "-clig".parse::<Feature>().ok()?];
    let shaped = rustybuzz::shape(&face, &features, buffer);
    if shaped.len() != word.len()
        || shaped.glyph_infos().iter().map(|info| info.glyph_id).ne(expected_glyphs.iter().map(|id| u32::from(*id)))
    {
        return None;
    }

    let units_per_em = u16::try_from(face.units_per_em()).ok()?;
    let scale = run.style.font_size * horizontal_scale(run) / f32::from(units_per_em);
    let mut positions = Vec::with_capacity(word.len() + 1);
    let mut position = 0.0;
    for (index, glyph) in shaped.glyph_positions().iter().enumerate() {
        positions.push(position + glyph.x_offset as f32 * scale + run.letter_spacing * index as f32);
        position += glyph.x_advance as f32 * scale;
    }
    positions.push(position + run.letter_spacing * word.len() as f32);
    Some(positions)
}

fn expected_glyphs(font: &FontSource, word: &[char]) -> Option<Vec<u16>> {
    word.iter()
        .map(|character| {
            let mut identities = font.used_identities.iter().filter(|identity| identity.unicode == *character);
            let glyph_id = identities.next()?.glyph_id?;
            identities.all(|identity| identity.glyph_id == Some(glyph_id)).then_some(glyph_id)
        })
        .collect()
}

fn fitted_letter_spacing(
    run: &PreparedRun,
    range: &Range<usize>,
    target: &[f32],
    natural_advances: &[f32],
) -> Option<f32> {
    let mut natural = Vec::with_capacity(target.len());
    natural.push(0.0);
    for advance in natural_advances {
        natural.push(natural.last().copied().unwrap_or_default() + advance + run.letter_spacing);
    }
    let (weighted_error, weight) = target.iter().zip(&natural).enumerate().skip(1).fold(
        (0.0, 0.0),
        |(error, weight), (index, (target, natural))| {
            let index = index as f32;
            (error + index * (target - natural), weight + index * index)
        },
    );
    let adjustment = weighted_error / weight;
    if adjustment.abs() > MAX_WORD_LETTER_SPACING {
        return None;
    }
    let fitted =
        natural.iter().enumerate().map(|(index, position)| position + adjustment * index as f32).collect::<Vec<_>>();
    (maximum_position_error(target, &fitted) <= POSITION_TOLERANCE)
        .then_some(run.letter_spacing + adjustment)
        .filter(|spacing| spacing.abs() > SPACING_MIN_PX || range.len() > 1)
}

fn maximum_position_error(target: &[f32], candidate: &[f32]) -> f32 {
    target.iter().zip(candidate).map(|(target, candidate)| (target - candidate).abs()).fold(0.0, f32::max)
}

fn horizontal_scale(run: &PreparedRun) -> f32 {
    match run.placement {
        RunPlacement::Bounded { .. } => 1.0,
        RunPlacement::Transformed { matrix, .. } => matrix[0].hypot(matrix[1]),
    }
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
