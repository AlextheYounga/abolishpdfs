# Text Layout Quality

## Request

Improve browser-native text layout so reconstructed PDF text preserves glyph order, positions, advances, spacing, and transforms.

## Problem

The current writer emits glyphs with local geometry and transform projection, but the model has no run-formation or measured advance compensation. Text with spacing, mixed objects, discontinuities, or transformed baselines can look different, select poorly, or drift from the PDF.

## Definitions

**Text run:** An ordered sequence of compatible glyphs sharing baseline, normalized transform, font, and paint style.

**Advance compensation:** CSS spacing or local positioning derived from the difference between browser-measured advance and the next PDFium glyph origin.

**Layout break:** A geometry or style discontinuity that ends a run and starts a new one.

## Desired outcome

Supported text fixtures render with stable glyph order and PDF-aligned positions, including measured spaces and transforms, while layout breaks remain explicit and do not produce accidental merged styles.

## Scope

- Form compatible glyphs into model-level text runs.
- Normalize transforms and baseline geometry at run boundaries.
- Add measured advance, letter-spacing, and local offset compensation.
- Preserve source spaces and handle generated layout spaces deliberately.
- Add geometry, browser screenshot, selection, and copied-text regression tests.

## Constraints

- Build on observed PDFium glyph origins, bounds, matrices, and styles.
- Keep layout decisions in the text/model layer; the HTML writer only serializes prepared runs.
- Use explicit tolerances established by the compatibility corpus.
- Preserve the native-text integrity policy when geometry cannot be represented safely.

## Exclusions

- FontForge processing or font metric rewriting.
- Paint-order visibility analysis.
- Full vertical-writing support; the first pass covers horizontal and transformed runs represented by existing model data.
