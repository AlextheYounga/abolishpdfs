/// Proves that the Unicode values reported by PDFium are present in an
/// embedded font's Unicode cmap.
pub fn mapping_is_proven(data: &[u8], used: &[char]) -> bool {
    let Some(cmap) = cmap_table(data) else {
        return false;
    };
    used.iter().all(|character| glyph_index(cmap, u32::from(*character)).is_some_and(|glyph| glyph != 0))
}

fn cmap_table(data: &[u8]) -> Option<&[u8]> {
    let version = read_u32(data, 0)?;
    if version != 0x0001_0000 && version != 0x4F54_544F {
        return None;
    }
    let table_count = usize::from(read_u16(data, 4)?);
    let directory_end = 12usize.checked_add(table_count.checked_mul(16)?)?;
    if directory_end > data.len() {
        return None;
    }
    for record in data[12..directory_end].chunks_exact(16) {
        if &record[..4] == b"cmap" {
            let offset = usize::try_from(read_u32(record, 8)?).ok()?;
            let length = usize::try_from(read_u32(record, 12)?).ok()?;
            let end = offset.checked_add(length)?;
            return data.get(offset..end);
        }
    }
    None
}

fn glyph_index(cmap: &[u8], codepoint: u32) -> Option<u16> {
    let subtable_count = usize::from(read_u16(cmap, 2)?);
    let records_end = 4usize.checked_add(subtable_count.checked_mul(8)?)?;
    if records_end > cmap.len() {
        return None;
    }

    let mut best: Option<(u8, &[u8])> = None;
    for record in cmap[4..records_end].chunks_exact(8) {
        let platform = read_u16(record, 0)?;
        let encoding = read_u16(record, 2)?;
        let offset = usize::try_from(read_u32(record, 4)?).ok()?;
        let subtable = cmap.get(offset..)?;
        let format = u8::try_from(read_u16(subtable, 0)?).ok()?;
        let priority = cmap_priority(platform, encoding, format, codepoint);
        if priority > best.map_or(0, |current| current.0) {
            best = Some((priority, subtable));
        }
    }
    best.and_then(|(_, subtable)| glyph_index_in_subtable(subtable, codepoint))
}

fn cmap_priority(platform: u16, encoding: u16, format: u8, codepoint: u32) -> u8 {
    let unicode_platform = platform == 0;
    let windows_unicode = platform == 3 && matches!(encoding, 1 | 10);
    let supports_codepoint = codepoint <= 0xFFFF || matches!(format, 12 | 13);
    if !supports_codepoint || (!unicode_platform && !windows_unicode) {
        return 0;
    }
    match format {
        12 | 13 => 4,
        4 => 3,
        6 => 2,
        0 => 1,
        _ => 0,
    }
}

fn glyph_index_in_subtable(subtable: &[u8], codepoint: u32) -> Option<u16> {
    match read_u16(subtable, 0)? {
        0 => {
            let codepoint = u8::try_from(codepoint).ok()?;
            let glyph = *subtable.get(6usize.checked_add(usize::from(codepoint))?)?;
            Some(u16::from(glyph))
        }
        4 => glyph_index_format_4(subtable, u16::try_from(codepoint).ok()?),
        6 => {
            let codepoint = u16::try_from(codepoint).ok()?;
            let first = read_u16(subtable, 6)?;
            let count = usize::from(read_u16(subtable, 8)?);
            let index = usize::from(codepoint.checked_sub(first)?);
            if index >= count {
                return None;
            }
            read_u16(subtable, 10usize.checked_add(index.checked_mul(2)?)?)
        }
        12 | 13 => glyph_index_format_12(subtable, codepoint),
        _ => None,
    }
}

fn glyph_index_format_4(subtable: &[u8], codepoint: u16) -> Option<u16> {
    let segment_count = usize::from(read_u16(subtable, 6)?) / 2;
    let end_codes = 14usize;
    let start_codes = end_codes.checked_add(segment_count.checked_mul(2)?)?.checked_add(2)?;
    let deltas = start_codes.checked_add(segment_count.checked_mul(2)?)?;
    let range_offsets = deltas.checked_add(segment_count.checked_mul(2)?)?;
    for index in 0..segment_count {
        let end = read_u16(subtable, end_codes.checked_add(index.checked_mul(2)?)?)?;
        let start = read_u16(subtable, start_codes.checked_add(index.checked_mul(2)?)?)?;
        if codepoint < start || codepoint > end {
            continue;
        }
        let delta = read_i16(subtable, deltas.checked_add(index.checked_mul(2)?)?)?;
        let range_offset = read_u16(subtable, range_offsets.checked_add(index.checked_mul(2)?)?)?;
        if range_offset == 0 {
            return Some(codepoint.wrapping_add_signed(delta));
        }
        let offset = range_offsets.checked_add(index.checked_mul(2)?)?.checked_add(usize::from(range_offset))?;
        let glyph = read_u16(subtable, offset)?;
        return Some(if glyph == 0 { 0 } else { glyph.wrapping_add_signed(delta) });
    }
    None
}

fn glyph_index_format_12(subtable: &[u8], codepoint: u32) -> Option<u16> {
    let group_count = usize::try_from(read_u32(subtable, 12)?).ok()?;
    for index in 0..group_count {
        let offset = 16usize.checked_add(index.checked_mul(12)?)?;
        let start = read_u32(subtable, offset)?;
        let end = read_u32(subtable, offset.checked_add(4)?)?;
        if codepoint < start || codepoint > end {
            continue;
        }
        let glyph = read_u32(subtable, offset.checked_add(8)?)?;
        let value =
            if read_u16(subtable, 0)? == 13 { glyph } else { glyph.checked_add(codepoint.checked_sub(start)?)? };
        return u16::try_from(value).ok();
    }
    None
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(offset..offset.checked_add(2)?)?.try_into().ok()?))
}

fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    Some(i16::from_be_bytes(data.get(offset..offset.checked_add(2)?)?.try_into().ok()?))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(offset..offset.checked_add(4)?)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_six_cmap_proves_used_characters() {
        let cmap = format_six_cmap();
        let data = sfnt_with_cmap(&cmap);
        assert!(mapping_is_proven(&data, &['A', 'B']));
        assert!(!mapping_is_proven(&data, &['C']));
    }

    #[test]
    fn invalid_font_does_not_prove_mapping() {
        assert!(!mapping_is_proven(&[0, 1, 2], &['A']));
    }

    #[test]
    fn empty_used_set_is_trivially_proven_for_valid_font() {
        let cmap = format_six_cmap();
        assert!(mapping_is_proven(&sfnt_with_cmap(&cmap), &[]));
    }

    fn sfnt_with_cmap(cmap: &[u8]) -> Vec<u8> {
        let table_offset = 28usize;
        let mut font = vec![0; table_offset + cmap.len()];
        font[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        font[4..6].copy_from_slice(&1u16.to_be_bytes());
        font[12..16].copy_from_slice(b"cmap");
        font[20..24].copy_from_slice(&u32::try_from(table_offset).unwrap_or_default().to_be_bytes());
        font[24..28].copy_from_slice(&u32::try_from(cmap.len()).unwrap_or_default().to_be_bytes());
        font[table_offset..].copy_from_slice(&cmap);
        font
    }

    fn format_six_cmap() -> Vec<u8> {
        let mut cmap = vec![0, 0, 0, 1, 0, 3, 0, 1, 0, 0, 0, 12, 0, 6, 0, 14, 0, 0, 0, 65, 0, 2];
        cmap.extend_from_slice(&[0, 1, 0, 2]);
        cmap
    }
}
