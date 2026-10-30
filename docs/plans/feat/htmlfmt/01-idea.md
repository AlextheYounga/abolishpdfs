# Idea

## Request

Generate HTML that provides the best of both worlds: retain the PDF's faithful
visual presentation while producing substantially more readable, normal-looking
HTML and useful native text.

## Problem

The current output preserves the resume visually, but its Chrome-generated PDF
contains approximately one text object per character. The text preparation
pipeline does not merge compatible glyphs across text-object boundaries, so the
result contains 2,237 absolutely positioned `text-run` spans for 2,242 visible
glyphs. Empty PDF text objects that represent word spaces are discarded, making
copied text read as concatenated words. The generated HTML is also serialized as
one compact line, which makes its structure unnecessarily difficult to inspect.

## Definitions

**Faithful visual view:** The existing page-sized HTML presentation in which the
raster background preserves non-text graphics and native text is positioned over
it using PDF geometry. This remains the sole rendered representation of the
document.

**Prepared run:** A sequence of adjacent native glyphs with compatible font,
paint, baseline, transform, and progression that can be emitted as one
absolutely positioned HTML text span.

**Semantic space:** A literal U+0020 character inserted between adjacent glyphs
when one or more empty native PDF text objects occur between them and both glyphs
otherwise belong to the same prepared run. It restores text that the PDF encodes
as positioning rather than as a glyph. Empty objects at run boundaries do not
produce spaces.

**Readable HTML:** Indented structural markup whose text runs contain words,
phrases, or lines rather than one span per source character. It does not mean
that visual PDF layout is replaced with browser-flowing document layout.

## Desired outcome

The resume renders with the same faithful page presentation and searchable native
text, while compatible characters are grouped into approximately line- or
style-level spans. Copied and inspected text contains spaces such as
`Austin Texas 78735`. The generated source is structurally indented and remains
valid, accessible HTML without duplicated text layers.

## Scope

- Merge compatible glyphs across adjacent native text objects during prepared-run
  construction.
- Recover semantic spaces from empty native text objects within a compatible run.
- Preserve exact glyph placement through the existing letter-spacing and local
  offset model, extended to account for inserted spaces.
- Ensure generated embedded fonts contain the space glyph needed by reconstructed
  runs.
- Indent document-level, page-level, outline, run, and link markup without adding
  whitespace inside text-run content.
- Add focused run-preparation, font-job, and HTML-output regression tests.
- Validate the resume through real conversion and browser rendering.

## Constraints

- Native text integrity and glyph identity guarantees must remain unchanged.
- Text must remain browser-native, searchable, selectable, and copyable.
- The raster page background must not be used to hide text reconstruction
  failures.
- Existing style, transform, paint-order, baseline, and progression compatibility
  rules remain the authority for run boundaries.
- Source order must be preserved.
- Space recovery must be based on explicit empty PDF text objects between
  compatible glyphs, not punctuation or natural-language guessing.
- Pretty printing must not introduce visible or selectable whitespace into text
  runs.
- Layout decisions remain in `src/text`; the HTML writer only serializes prepared
  model data.

## Exclusions

- Inferring headings, paragraphs, lists, tables, columns, or general reading-order
  semantics.
- Replacing the faithful page view with reflowable HTML.
- Adding a duplicate hidden reading or accessibility layer.
- Parsing or post-processing generated HTML to recover layout semantics.
- Changing PDFium extraction, clipping policy, native text integrity policy, or
  raster fallback behavior.
