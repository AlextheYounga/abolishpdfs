# FontForge Pipeline

## Request

Convert embedded PDF fonts into deterministic browser web-font assets through an isolated FontForge worker.

## Problem

The current writer emits decoded embedded font bytes directly and references them with `@font-face`. It does not subset unused glyphs, repair browser mappings, correct PDF advances, or produce a consistent browser-compatible font artifact. Raw embedded data therefore cannot establish reliable native text fidelity.

## Definitions

**Processed web font:** A FontForge-generated WOFF2 or WOFF asset with browser-safe naming, mappings, and metrics for the glyphs used by the document.

**Font job:** One isolated request to process one document-local `FontSource`, returning an asset and structured diagnostics.

**Proven mapping:** A mapping established by the existing cmap analysis or an equivalent validated mapping step; unproven mappings are not silently rewritten.

## Desired outcome

Fonts with usable embedded data and proven mappings are processed into deterministic browser web fonts whose glyph selection and advances support native HTML text. Unsupported fonts produce explicit diagnostics and follow the native-text integrity policy.

## Scope

- Define a structured FontForge worker request/response boundary.
- Process supported embedded TrueType and OpenType fonts with used-glyph selection, browser-safe mappings, and metric correction.
- Generate deterministic WOFF2 assets, with WOFF output when the selected FontForge toolchain requires it.
- Integrate processed assets and diagnostics into the model and HTML writer.
- Add font inspection and regression tests.

## Constraints

- Run FontForge out of process; do not link its global C state into the Rust process.
- Preserve document-local font identity and deterministic asset names.
- Reject unproven Unicode-to-glyph mappings instead of guessing.
- Keep unsupported CID, CFF, Type 1, and Type 3 handling explicit.

## Exclusions

- Advanced font-format support beyond the selected initial TrueType/OpenType scope.
- Layout run formation and spacing compensation.
- Release packaging of FontForge itself.
