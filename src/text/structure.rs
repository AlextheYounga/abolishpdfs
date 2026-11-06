use std::ops::Range;

use crate::model::{PreparedRun, RunPlacement, RunStyle};

const LINE_TOLERANCE: f32 = 2.0;
const MATRIX_TOLERANCE: f32 = 1e-3;
const LEFT_TOLERANCE: f32 = 1.0;
const CONTINUATION_GAP_FACTOR: f32 = 1.8;
const LINE_GAP_FACTOR: f32 = 4.0;
const STYLE_SIZE_TOLERANCE: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockKind {
    Heading1,
    Heading2,
    Paragraph,
    ListItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextBlock {
    pub(crate) kind: BlockKind,
    pub(crate) lines: Vec<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextStructure {
    pub(crate) blocks: Vec<TextBlock>,
}

impl TextStructure {
    pub(crate) fn from_runs(runs: &[PreparedRun]) -> Self {
        let lines = visual_lines(runs);
        let median_font_size = median_font_size(runs);
        let largest_font_size = runs.iter().map(|run| run.style.font_size).fold(0.0, f32::max);
        let mut blocks = Vec::new();
        let mut line_index = 0;

        while line_index < lines.len() {
            let kind = classify_line(&lines[line_index], runs, median_font_size, largest_font_size);
            if matches!(kind, BlockKind::Heading1 | BlockKind::Heading2) {
                blocks.push(TextBlock { kind, lines: vec![lines[line_index].clone()] });
                line_index += 1;
                continue;
            }

            let mut block_lines = vec![lines[line_index].clone()];
            line_index += 1;
            while line_index < lines.len()
                && can_continue_block(kind, block_lines.last().unwrap_or(&lines[line_index]), &lines[line_index], runs)
                && !is_heading(&lines[line_index], runs, median_font_size, largest_font_size)
            {
                block_lines.push(lines[line_index].clone());
                line_index += 1;
            }
            blocks.push(TextBlock { kind, lines: block_lines });
        }

        Self { blocks }
    }
}

fn visual_lines(runs: &[PreparedRun]) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    while start < runs.len() {
        let mut end = start + 1;
        while end < runs.len() && shares_visual_line(&runs[end - 1], &runs[end]) {
            end += 1;
        }
        lines.push(start..end);
        start = end;
    }
    lines
}

fn shares_visual_line(first: &PreparedRun, next: &PreparedRun) -> bool {
    let baseline_matches = match (&first.placement, &next.placement) {
        (RunPlacement::Bounded { top: first, .. }, RunPlacement::Bounded { top: next, .. }) => {
            (first - next).abs() <= LINE_TOLERANCE
        }
        (
            RunPlacement::Transformed { bottom: first_bottom, matrix: first_matrix, .. },
            RunPlacement::Transformed { bottom: next_bottom, matrix: next_matrix, .. },
        ) => {
            (first_bottom - next_bottom).abs() <= LINE_TOLERANCE
                && first_matrix.iter().zip(next_matrix).all(|(first, next)| (first - next).abs() <= MATRIX_TOLERANCE)
        }
        _ => false,
    };
    let gap = line_left(next) - estimated_right(first);
    baseline_matches
        && gap >= -first.style.font_size * 2.0
        && gap <= first.style.font_size.max(next.style.font_size) * LINE_GAP_FACTOR
}

fn classify_line(
    line: &Range<usize>,
    runs: &[PreparedRun],
    median_font_size: f32,
    largest_font_size: f32,
) -> BlockKind {
    let text = line_text(line, runs);
    let font_size = line_font_size(line, runs);
    if has_letters(&text)
        && largest_font_size > median_font_size * 1.5
        && font_size >= largest_font_size - LINE_TOLERANCE
    {
        BlockKind::Heading1
    } else if is_uppercase_label(&text) && font_size > median_font_size * 1.05 {
        BlockKind::Heading2
    } else if starts_list_item(&text) {
        BlockKind::ListItem
    } else {
        BlockKind::Paragraph
    }
}

fn is_heading(line: &Range<usize>, runs: &[PreparedRun], median: f32, largest: f32) -> bool {
    matches!(classify_line(line, runs, median, largest), BlockKind::Heading1 | BlockKind::Heading2)
}

fn can_continue_block(kind: BlockKind, previous: &Range<usize>, next: &Range<usize>, runs: &[PreparedRun]) -> bool {
    if starts_list_item(&line_text(next, runs)) || previous.len() != 1 || next.len() != 1 {
        return false;
    }
    let previous_run = &runs[previous.start];
    let next_run = &runs[next.start];
    let gap = (line_position(previous_run) - line_position(next_run)).abs();
    matches!(kind, BlockKind::Paragraph | BlockKind::ListItem)
        && (line_left(previous_run) - line_left(next_run)).abs() <= LEFT_TOLERANCE
        && placements_continue(&previous_run.placement, &next_run.placement)
        && styles_continue(&previous_run.style, &next_run.style)
        && gap > LINE_TOLERANCE
        && gap <= previous_run.style.font_size * CONTINUATION_GAP_FACTOR
}

fn placements_continue(first: &RunPlacement, next: &RunPlacement) -> bool {
    match (first, next) {
        (RunPlacement::Bounded { .. }, RunPlacement::Bounded { .. }) => true,
        (RunPlacement::Transformed { matrix: first, .. }, RunPlacement::Transformed { matrix: next, .. }) => {
            first.iter().zip(next).all(|(first, next)| (first - next).abs() <= MATRIX_TOLERANCE)
        }
        _ => false,
    }
}

fn styles_continue(first: &RunStyle, next: &RunStyle) -> bool {
    first.font == next.font
        && (first.font_size - next.font_size).abs() <= first.font_size.max(1.0) * STYLE_SIZE_TOLERANCE
        && first.fill == next.fill
        && first.stroke == next.stroke
        && first.render_mode == next.render_mode
}

fn line_text(line: &Range<usize>, runs: &[PreparedRun]) -> String {
    runs[line.clone()].iter().map(|run| run.text.as_str()).collect::<Vec<_>>().join(" ")
}

fn line_font_size(line: &Range<usize>, runs: &[PreparedRun]) -> f32 {
    runs[line.clone()].iter().map(|run| run.style.font_size).fold(0.0, f32::max)
}

fn line_position(run: &PreparedRun) -> f32 {
    match run.placement {
        RunPlacement::Bounded { top, .. } => top,
        RunPlacement::Transformed { bottom, .. } => bottom,
    }
}

fn line_left(run: &PreparedRun) -> f32 {
    match run.placement {
        RunPlacement::Bounded { left, .. } | RunPlacement::Transformed { left, .. } => left,
    }
}

fn estimated_right(run: &PreparedRun) -> f32 {
    line_left(run) + run.observed_advances.iter().sum::<f32>() + run.style.font_size
}

fn median_font_size(runs: &[PreparedRun]) -> f32 {
    let mut sizes = runs.iter().map(|run| run.style.font_size).collect::<Vec<_>>();
    sizes.sort_by(f32::total_cmp);
    sizes.get(sizes.len() / 2).copied().unwrap_or_default()
}

fn has_letters(text: &str) -> bool {
    text.chars().any(char::is_alphabetic)
}

fn is_uppercase_label(text: &str) -> bool {
    let letters = text.chars().filter(|character| character.is_alphabetic()).collect::<Vec<_>>();
    letters.len() >= 2 && letters.iter().all(|character| !character.is_lowercase())
}

fn starts_list_item(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with("- ") || text.starts_with("• ") || text.starts_with("▪ ") || text.starts_with("◦ ")
}

#[cfg(test)]
#[path = "structure_tests.rs"]
mod tests;
