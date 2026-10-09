import json
import sys

import fontforge


input_path, output_path, response_path, request_path = sys.argv[1:]
with open(request_path, encoding="utf-8") as request_file:
    spec = json.load(request_file)

font = fontforge.open(input_path)
font.encoding = "UnicodeFull"
glyphs_by_id = list(font.glyphs())
family_name = spec["family_name"]
font.familyname = family_name
font.fontname = family_name.replace("-", "")
font.fullname = family_name
records = spec["records"]
named = []
fallback_codes = set()
for record in records:
    name = record.get("source_glyph_name")
    try:
        named_glyph = font[name] if name else None
    except (KeyError, ValueError):
        named_glyph = None
    if named_glyph is None:
        glyph_id = record.get("source_glyph_id")
        if glyph_id is not None and 0 <= glyph_id < len(glyphs_by_id):
            named_glyph = glyphs_by_id[glyph_id]
    if named_glyph is not None:
        named.append((named_glyph, ord(record["unicode"])))
    else:
        fallback_codes.add(ord(record["unicode"]))

# An occupant already designated by another explicit-identity record wins this
# collision deterministically; both records have identity, so source order is
# represented by retaining the existing designated occupant.
designated = {glyph.glyphname for glyph, _ in named}
planned_moves = []
for source, target in named:
    try:
        occupant = font[target]
    except (KeyError, TypeError, ValueError):
        occupant = None
    if occupant is not None and occupant.glyphname != source.glyphname and occupant.glyphname in designated:
        continue
    if occupant is not None and occupant.glyphname != source.glyphname and occupant.glyphname != ".notdef":
        font.removeGlyph(occupant)
    planned_moves.append((source, target))
for source, target in planned_moves:
    source.unicode = target

font.selection.none()
for _, target in named:
    font.selection.select(("more", "unicode"), target)
for codepoint in fallback_codes:
    font.selection.select(("more", "unicode"), codepoint)
font.selection.select(("more",), ".notdef")

upm = int(getattr(font, "em", 1000) or 1000)
widths = []
for record in records:
    codepoint = ord(record["unicode"])
    slot = next((glyph for glyph, target in named if target == codepoint), None)
    if slot is None:
        try:
            slot = font[record["unicode"]]
        except (KeyError, TypeError, ValueError):
            slot = None
    widths.append(int(slot.width) if slot is not None else 0)

font.selection.invert()
font.clear()
font.generate(output_path)

with open(response_path, "w", encoding="utf-8") as response:
    json.dump({"family_name": family_name, "glyph_count": len(list(font.glyphs())), "units_per_em": upm, "advance_widths": widths}, response)
