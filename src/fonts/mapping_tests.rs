use super::*;

#[test]
fn format_six_cmap_proves_only_present_nonzero_glyphs() {
    let data = sfnt_with_cmap(&[(3, 1, format_six())]);
    assert!(mapping_is_proven(&data, &['A', 'B']));
    assert!(!mapping_is_proven(&data, &['C']));
}

#[test]
fn invalid_or_truncated_font_does_not_prove_mapping() {
    assert!(!mapping_is_proven(&[0, 1, 2], &['A']));
    let data = sfnt_with_cmap(&[(3, 1, format_six())]);
    for end in 0..data.len() {
        assert!(!mapping_is_proven(&data[..end], &['A', 'B']));
    }
}

#[test]
fn empty_usage_is_trivially_proven_for_a_valid_cmap() {
    assert!(mapping_is_proven(&sfnt_with_cmap(&[(3, 1, format_six())]), &[]));
}

#[test]
fn format_four_indexes_each_character_in_the_glyph_array() {
    // A maps to .notdef, B to GID 37 in the same segment. Looking up only
    // the first array entry incorrectly rejects B as well as A.
    let format_four = words(&[4, 36, 0, 4, 4, 1, 0, 66, 65535, 0, 65, 65535, 0, 1, 4, 0, 0, 37]);
    let data = sfnt_with_cmap(&[(3, 1, format_four)]);
    assert!(!mapping_is_proven(&data, &['A']));
    assert!(mapping_is_proven(&data, &['B']));
}

#[test]
fn unicode_subtables_can_supply_complementary_bmp_and_non_bmp_coverage() {
    let mut format_twelve = words(&[12, 0]);
    for value in [28u32, 0, 1, 0x1F600, 0x1F600, 42] {
        format_twelve.extend(value.to_be_bytes());
    }
    let data = sfnt_with_cmap(&[(3, 1, format_six()), (3, 10, format_twelve)]);
    assert!(mapping_is_proven(&data, &['A', '\u{1F600}']));
    assert!(!mapping_is_proven(&data, &['\u{1F601}']));
}

#[test]
fn non_unicode_cmap_cannot_prove_unicode_mapping() {
    assert!(!mapping_is_proven(&sfnt_with_cmap(&[(3, 0, format_six())]), &['A']));
}

#[test]
fn invalid_subtable_does_not_hide_a_later_valid_unicode_subtable() {
    let data = sfnt_with_cmap(&[(3, 1, vec![0, 255]), (3, 1, format_six())]);
    assert!(mapping_is_proven(&data, &['A']));
}

#[test]
fn identity_proof_requires_nonempty_in_range_source_glyphs() {
    let data = include_bytes!("../../tests/fixtures/fonts/LiberationSans-Regular.ttf");
    let identity = |glyph_id| GlyphIdentity { unicode: '\u{1F600}', source_code: None, glyph_id };
    assert!(identity_mapping_is_proven(data, &[identity(Some(36))]));
    assert!(!identity_mapping_is_proven(data, &[]));
    for glyph in [None, Some(0), Some(u16::MAX)] {
        assert!(!identity_mapping_is_proven(data, &[identity(glyph)]));
    }
}

fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|value| value.to_be_bytes()).collect()
}

fn format_six() -> Vec<u8> {
    words(&[6, 14, 0, 65, 2, 1, 2])
}

fn sfnt_with_cmap(subtables: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(subtables.len()).unwrap();
    let mut cmap = words(&[0, count]);
    let mut offset = 4 + subtables.len() * 8;
    for (platform, encoding, subtable) in subtables {
        cmap.extend(words(&[*platform, *encoding]));
        cmap.extend(u32::try_from(offset).unwrap().to_be_bytes());
        offset += subtable.len();
    }
    for (_, _, subtable) in subtables {
        cmap.extend(subtable);
    }
    let mut font = vec![0; 28];
    font[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    font[4..6].copy_from_slice(&1u16.to_be_bytes());
    font[12..16].copy_from_slice(b"cmap");
    font[20..24].copy_from_slice(&28u32.to_be_bytes());
    font[24..28].copy_from_slice(&u32::try_from(cmap.len()).unwrap().to_be_bytes());
    font.extend(cmap);
    font
}
