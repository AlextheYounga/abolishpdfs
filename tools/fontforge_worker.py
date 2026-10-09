import json
import sys

import fontforge


input_path, output_path, response_path, request_path = sys.argv[1:]
with open(request_path, encoding="utf-8") as request_file:
    spec = json.load(request_file)

font = fontforge.open(input_path)
font.encoding = "UnicodeFull"
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
    occupant = font[target]
    if occupant is not None and occupant.glyphname != source.glyphname and occupant.glyphname in designated:
        continue
    if occupant is not None and occupant.glyphname != source.glyphname and occupant.glyphname != ".notdef":
        font.removeGlyph(occupant)
    planned_moves.append((source, target))
for source, target in planned_moves:
    source.unicode = target

font.selection.none()
for _, target in named:
    font.selection.select(("unicode", target))
for codepoint in fallback_codes:
    font.selection.select(("unicode", codepoint))
font.selection.select(("more", ".notdef"))
font.selection.invert()
font.clear()
font.generate(output_path)

upm = int(getattr(font, "em", 1000) or 1000)
widths = []
for record in records:
    slot = font[ord(record["unicode"])]
    widths.append(int(slot.width) if slot is not None else 0)
with open(response_path, "w", encoding="utf-8") as response:
    json.dump({"family_name": family_name, "glyph_count": len(font.glyphs()), "units_per_em": upm, "advance_widths": widths}, response)
