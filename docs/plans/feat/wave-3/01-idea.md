# Idea

## Request

Begin Wave 3 of `docs/forward.md`: the mostly sequential font-pipeline spine
`pdfium-glyph-identity` → `fontforge-glyph-remapping` → `browser-font-metrics`,
landed together on the `feat/wave-3` branch, with as many subagents as necessary.

## Problem

Wave 2 proved with `embedded-tounicode-mismatch.pdf` that the converter's
Unicode→cmap inference is unprovable: PDFium reports per-character Unicode
(ToUnicode-decoded), the pipeline trusts it end to end, and a PDF whose visible
glyph is `A` while ToUnicode claims `B` currently renders `B` in the browser —
the wrong shape in browser-native text with no way to detect or correct it,
because nothing in the model knows which source glyph drew which character:

1. `Glyph` (src/model/text.rs) records only mapped Unicode plus geometry; the
   content-stream character code and the font glyph ID are lost at extraction.
2. The FontForge worker subsets solely by requested Unicode codepoints
   (`src/fonts/worker.rs`, `tools/fontforge_worker.py`), so the WOFF2 keeps
   whichever outline the misleading Unicode points at.
3. Layout compensation in `src/text/runs.rs` measures advances against the raw
   source font bytes while the browser loads the FontForge-processed asset, and
   no kerning policy exists; processed-font metrics returned by the worker are
   validated and then ignored.

The upstream evidence is documented: the public PDFium 7881 surface exposes no
per-character original code, glyph ID, or font-resource handle
(`vendor/pdfium-7881/include/fpdf_text.h`), which CONTEXT.md already names as
the central public-API limitation.

## Definitions

**Source character code**: the character-code value carried by the PDF content
stream for one text-showing operation byte (`Tj` operand element), distinct
from any ToUnicode-mapped Unicode value. For the adversarial fixture this is
`0x41` regardless of what `FPDFText_GetUnicode` reports.

**Glyph ID**: the glyph identifier in the embedded font program that draws a
given character code, per that font's internal mapping (`CPDF_Font::GlyphFromCharCode`
semantics). Range is font-program-specific and stable for the document lifetime.

**Identity record**: `(source_character_code, glyph_id, semantic_unicode)` for
one extracted character. A field is optional when its origin cannot be proven;
an identity record with all fields absent equals today's extraction.

**Capability-gated binding**: an extern declaration looked up dynamically
against the loaded `libpdfium.so`; missing symbols degrade to `None` values
without failing conversion, mirroring the existing capability-probe culture in
`src/pdfium/probe.rs`.

**Maintained PDFium patch boundary**: additive `.patch` files under
`patches/pdfium/`, applied when building patched binaries, kept ABI-additive so
stock binaries remain loadable and functional.

**Reencoded glyph slot**: after FontForge processing, a glyph program that lives
at the semantic Unicode encoding point while containing the outline drawn by
the source glyph ID — restoring visual fidelity without changing DOM/copy text.

**Processed metrics**: advance widths of the font actually shipped to the
browser (the processed WOFF2), keyed by semantic Unicode and normalized by that
font's units-per-em, as reported by the worker response.

## Desired outcome

1. Extracted `Glyph`s optionally carry source character code and glyph ID
   whenever the loaded PDFium provides them; on stock PDFium 7881 builds every
   existing behavior is unchanged and the capability report honestly shows the
   disabled state.
2. `patches/pdfium/glyph-identity.patch` adds two exported functions
   (`FPDFText_GetCharCode`, `FPDFText_GetGlyphId`) to PDFium, documented as the
   maintained patch boundary, buildable against the pinned 7881 sources using
   the vendored `args.gn`.
3. The FontForge worker receives a per-job request specification of identity
   records instead of a bare Unicode list, selects source glyphs by glyph ID
   (post-table PS name resolution done in Rust), reencodes them into the
   semantic Unicode slots, and reports ordered, units-normalized advances for
   exactly the emitted slots.
4. Layout compensation measures against processed metrics with graceful
   fallbacks (processed → source → none), letter-spacing thresholds are pixel
   semantics rather than reused matrix epsilon, and text runs suppress browser
   kerning so compensation stays deterministic.
5. All offline gates pass unchanged where behavior is intentionally preserved:
   native_text/clipboard expectations for the adversarial fixture keep `"B"`
   (DOM/copy semantics do not move), fixture bytes stay byte-stable, and the
   full `cargo test`, corpus manifest, and clippy suites are green.

## Scope

- Extend `Glyph` and usage collection with optional identity fields sourced
  through new capability-gated extern bindings over the same loaded library.
- Vendor the pinned `pdfium-render` crate behind `[patch.crates-io]` with a
  minimal published-accessor diff so identity work can reach live handles in a
  single enumeration pass.
- Author the additive PDFium patch file and its rebuild documentation.
- Replace the FontForge job request/response protocol (Unicode CSV → identity
  specification file carrying PS names + Unicode; response gains units-per-em
  and ordered per-record advances), including fake-worker tests and the
  bootstrap smoke call.
- Reencode glyphs onto semantic Unicode slots inside `tools/fontforge_worker.py`
  with deterministic ordering.
- Route run compensation through processed metrics; add the pixel threshold
  constant; emit `font-kerning:none` on text runs.
- Update CONTEXT.md documenting the patch boundary, protocol, and metric model.

## Constraints

- `develop` output behavior must not regress: stock-binary runs keep emitting
  identical HTML/CSS/assets except where explicitly specified (kerning CSS).
- Edition 2024 lints stand: deny `unwrap_used`, `expect_used`, `panic`,
  `todo`, `unimplemented`, `allow_attributes`, `undocumented_unsafe_blocks`,
  `missing_safety_doc`; files ≤400 lines; thiserror-style errors.
- No test may depend on a rebuilt patched PDFium being present; degradation is
  exercised with the stock vendored binary only when present in `.runtime/`.
- Fixture generation remains offline and byte-for-byte deterministic.
- Follow the Acta record; implementation lands only in the wave-3 worktree.

## Exclusions

- Rebuilding/running a patched PDFium binary in this environment; the branch
  ships patch + tooling and proves everything observable without it.
- Merging or closing Wave 2 follow-ups beyond what identity/remapping needs.
- Per-character screenshot baselines for remapped visual output; baselines
  remain capture-only until the runtime gate can execute here again.
- Stable font-resource identity hashing and font-name-based deduplication
  replacement (`feat/font-resource-identity` stays a later roadmap branch);
  jobs continue keying off existing `FontId`s.
- TJ/Tw/generated-space semantics, transform parity, CFF/CID/Type1 support —
  later roadmap branches are untouched.
