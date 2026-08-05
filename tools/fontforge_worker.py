import json
import sys

import fontforge


input_path, output_path, response_path, family_name, codepoints = sys.argv[1:]
font = fontforge.open(input_path)
font.encoding = "UnicodeFull"
font.familyname = family_name
font.fontname = family_name.replace("-", "")
font.fullname = family_name

requested = {int(value) for value in codepoints.split(",") if value}
font.selection.none()
for codepoint in requested:
    font.selection.select(("unicode", codepoint))
font.selection.select(("more", ".notdef"))
font.selection.invert()
font.clear()
font.generate(output_path)

widths = [glyph.width for glyph in font.glyphs() if glyph.glyphname == ".notdef" or glyph.unicode in requested]
with open(response_path, "w", encoding="utf-8") as response:
    json.dump({"family_name": family_name, "glyph_count": len(widths), "advance_widths": widths}, response)
