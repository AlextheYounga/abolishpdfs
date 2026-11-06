use super::*;
use crate::model::{Color, RunPlacement, TextRenderMode};

fn run(text: &str, left: f32, bottom: f32, font_size: f32) -> PreparedRun {
    PreparedRun {
        source: 1,
        style: RunStyle {
            font: None,
            font_size,
            fill: Some(Color::BLACK),
            stroke: None,
            render_mode: TextRenderMode::Fill,
        },
        placement: RunPlacement::Transformed { left, bottom, matrix: [1.0, 0.0, 0.0, 1.0] },
        observed_advances: vec![font_size * 0.5; text.chars().count().saturating_sub(1)],
        local_offsets: Vec::new(),
        letter_spacing: 0.0,
        text: text.to_owned(),
    }
}

#[test]
fn structure_groups_same_baseline_fragments_into_one_line() {
    let runs = [run("Company", 10.0, 100.0, 12.0), run("Engineer", 80.0, 100.0, 12.0)];

    let structure = TextStructure::from_runs(&runs);

    assert_eq!(structure.blocks.len(), 1);
    assert_eq!(structure.blocks[0].kind, BlockKind::Paragraph);
    assert_eq!(structure.blocks[0].lines.len(), 1);
    assert_eq!(structure.blocks[0].lines[0], 0..2);
}

#[test]
fn structure_does_not_join_distant_columns_on_the_same_baseline() {
    let runs = [run("Left column", 10.0, 100.0, 12.0), run("Right column", 300.0, 100.0, 12.0)];

    let structure = TextStructure::from_runs(&runs);

    assert_eq!(structure.blocks.len(), 2);
}

#[test]
fn structure_groups_compatible_continuation_lines_into_a_paragraph() {
    let runs = [run("First line", 10.0, 100.0, 10.0), run("second line", 10.0, 84.0, 10.0)];

    let structure = TextStructure::from_runs(&runs);

    assert_eq!(structure.blocks, vec![TextBlock { kind: BlockKind::Paragraph, lines: vec![0..1, 1..2] }]);
}

#[test]
fn structure_does_not_group_lines_with_different_transforms() {
    let first = run("First line", 10.0, 100.0, 10.0);
    let mut second = run("Rotated line", 10.0, 84.0, 10.0);
    second.placement = RunPlacement::Transformed { left: 10.0, bottom: 84.0, matrix: [0.0, 1.0, -1.0, 0.0] };

    let structure = TextStructure::from_runs(&[first, second]);

    assert_eq!(structure.blocks.len(), 2);
}

#[test]
fn structure_recognizes_explicit_list_items_and_their_continuations() {
    let runs = [
        run("- First item", 10.0, 100.0, 10.0),
        run("continues here", 10.0, 85.0, 10.0),
        run("- Second item", 10.0, 67.0, 10.0),
    ];

    let structure = TextStructure::from_runs(&runs);

    assert_eq!(structure.blocks[0], TextBlock { kind: BlockKind::ListItem, lines: vec![0..1, 1..2] });
    assert_eq!(structure.blocks[1].kind, BlockKind::ListItem);
    assert_eq!(structure.blocks[1].lines.len(), 1);
    assert_eq!(structure.blocks[1].lines[0], 2..3);
}

#[test]
fn structure_recognizes_large_titles_and_uppercase_section_labels() {
    let runs = [
        run("DOCUMENT TITLE", 10.0, 120.0, 24.0),
        run("EXPERIENCE", 10.0, 90.0, 14.0),
        run("Body", 10.0, 70.0, 10.0),
        run("More body", 10.0, 54.0, 10.0),
        run("Last body", 10.0, 38.0, 10.0),
    ];

    let structure = TextStructure::from_runs(&runs);

    assert_eq!(structure.blocks[0].kind, BlockKind::Heading1);
    assert_eq!(structure.blocks[1].kind, BlockKind::Heading2);
    assert_eq!(structure.blocks[2].kind, BlockKind::Paragraph);
}
