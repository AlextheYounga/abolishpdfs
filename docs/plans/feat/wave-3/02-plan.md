# Plan

## Current behavior

Extraction (`src/pdfium/document.rs:254-276`) builds `Glyph`s
(`src/model/text.rs:52-63`) from `PdfPageTextChar` values: Unicode via
ToUnicode, origin/bounds/font-size/colors/generated flag. Character index,
source code, and glyph ID are unavailable in pdfium-render 0.9.3 — every handle
getter is `pub(crate)` (`~/.cargo/registry/.../pdfium-render-0.9.3/src/pdf/**`),
and the PDFium 7881 headers export no code/glyph function
(`vendor/pdfium-7881/include/fpdf_text.h`, audited). The capability probe
already records this honestly: `character_to_pdf_code: false`,
`character_to_glyph_id: false` (`src/pdfium/report.rs:212-224`).

Font usage accumulates only `used_unicode` per `FontSource`
(`src/model/font.rs:38-46`). Mapping proof requires each used Unicode to hit a
nonzero glyph in the source cmap (`src/fonts/mapping.rs`). Jobs are spawned per
embedded font with a decimal Unicode CSV as the last positional argv entry
(`src/fonts/worker.rs:115-130`); the worker script subsets by codepoint
(`tools/fontforge_worker.py:14-20`) and reports unordered raw widths plus
family/count (`src/fonts/worker.rs:194-211` validation). Compensation math
(`src/text/runs.rs:149-211`) reads advances from the source bytes via
ttf-parser; letter-spacing emission reuses `projection::EPSILON` as a pixel
cutoff (`src/output/document.rs:138-142`); processed assets reach only the CSS
`@font-face`. Fake-worker tests bind to argument shape
(`tests/fontforge_worker.rs`, helper `src/bin/fontforge_test_helper.rs`), and
the bootstrap smoke call hardcodes one legacy-style invocation
(`tools/bootstrap-runtime:74-86`).

## Intended behavior

1. **Identity fields on the model.** `Glyph` gains
   `source_code: Option<u32>` and `glyph_id: Option<u16>`.
   `FontSource.used_identities: Vec<GlyphIdentity>` collects
   `(code, gid, unicode)` triples during extraction. Serialization includes
   them so diagnostics expose identity state.
2. **Capability-gated access in one pass.** A vendored fork of pdfium-render
   0.9.3 flips only internal handle accessors from `pub(crate)` to `pub`
   (text page, document/page/object handles needed by extraction). New module
   `src/pdfium/identity.rs` declares two extern functions:
   `FPDFText_GetCharCode(FPDF_TEXTPAGE, i32) -> u64`,
   `FPDFText_GetGlyphId(FPDF_TEXTPAGE, i32) -> u32`, resolved dynamically with
   graceful `None` when symbols are absent (stock binaries). Extraction calls
   them inside the existing per-character loop; no second enumeration pass
   exists.
3. **Patch boundary artifact.** `patches/pdfium/glyph-identity.patch` adds both
   exports atop 7881 sources (read existing `PAGECHAR_INFO` members; GID via
   `CPDF_Font::GlyphFromCharCode(code)`), plus a documented rebuild recipe.
   Identity stays optional everywhere; patched builds simply fill the fields.
4. **Glyph-driven worker protocol.** Job requests become a JSON spec file next
   to the temp input: `{ "family_name", "output_format", "records": [ {
   "unicode": <cp>, "source_glyph_name": <ps name|null>, "source_code":
   <int|null>, "source_glyph_id": <int|null> } ] }`. Rust resolves
   glyph-ID→PostScript names using ttf-parser `post` data on the embedded font
   before invoking FontForge. The Python worker opens the source program,
   selects glyphs by PS name (falling back to codepoint selection for records
   without names), moves/reencodes each selected outline onto its semantic
   Unicode slot (including remapping when the ToUnicode value differs from the
   shaped source glyph), always retains `.notdef`, then generates WOFF2.
   Responses report `{ family_name, glyph_count, units_per_em, advance_widths
   }` where `advance_widths[i]` is the width of slot `records[i].unicode` in
   font units. Validation enforces record-keyed counts; the unicode CSV argv
   protocol is removed end-to-end (fake helper + bootstrap smoke updated).
5. **Processed metrics consumed.** `ProcessedFont` keeps the ordered semantic
   slots with units-per-em so run preparation measures compensation against
   delivered font metrics first, then source bytes, else none. Letter-spacing
   gating uses a dedicated pixel epsilon constant. Text runs emit
   `font-kerning:none` so browser shaping cannot diverge from hmtx-based
   expectations.

## Approach

Phase order follows the dependency spine; each phase is self-contained and
testable offline.

**A. pdfium-glyph-identity**: vendor crate + accessor diff → extern shim +
probe integration → model field extension through extraction → tests covering
propagation and degradation. Diagnostic output shows `null` codes/GIDs until a
patched binary is supplied, keeping every snapshot stable elsewhere.

**B. fontforge-glyph-remapping**: PS-name resolution utility + unit tests
against pinned Liberation fonts → request-spec builder replacing the CSV path
across worker/validation/helper/bootstrap → Python rewrite for selection +
reencoding + keyed widths → contract tests that run the pipeline through the
fake worker asserting the new schema, plus one FontForge-independent check that
spec contents preserve fixture identity facts (code `0x41`, unicode `B`) when
identity exists and degrade to Unicode-only records when it does not.

**C. browser-font-metrics foundations**: advance lookup chain swap → threshold
constant + CSS kerning suppression → unit tests constructing runs with
known processed advances verifying emitted CSS values.

## Responsibilities and boundaries

- Identity acquisition belongs in `src/pdfium/*`; the owned model carries the
  data but never calls PDFium itself (unchanged boundary).
- PostScript-name resolution is a pure function over font bytes under
  `src/fonts/`; the FontForge invocation stays the sole job driver in
  `src/fonts/worker.rs`.
- Glyph outline surgery lives entirely in `tools/fontforge_worker.py`; Rust
  never parses or edits processed outlines.
- Run serialization keeps single ownership of CSS in `src/output/document.rs`;
  metric decisions stay computed in `src/text/runs.rs`.

## Affected areas

- `Cargo.toml` (`[patch.crates-io]`), new `vendor/pdfium-render/` tree (upstream
  verbatim minus published-accessor diff).
- `src/pdfium/identity.rs` (new), `src/pdfium/document.rs`, `report.rs` fields,
  `probe.rs` capability wiring.
- `src/model/text.rs`, `src/model/font.rs`, `src/text/runs.rs`,
  `src/output/document.rs`.
- `src/fonts/worker.rs`, new `src/fonts/psnames.rs`,
  `tools/fontforge_worker.py`, `src/bin/fontforge_test_helper.rs`,
  `tests/fontforge_worker.rs`, `tools/bootstrap-runtime`.
- `patches/pdfium/glyph-identity.patch` + rebuild docs.
- `CONTEXT.md`, plan completion notes.

## Decisions

- **Vendored micro-fork instead of bindings-decorator or double enumeration.**
  The decorator costs ~500 maintained stubs; a second-pass mini-loader duplicates
  subtle traversal logic. Flipping ~7 accessors keeps one canonical enumeration
  pass, reviewable as a diff against upstream, trivially removable when upstream
  publishes handles. Vendored sources are imported verbatim at version 0.9.3
  with provenance recorded in the patch note.
- **Dynamically probed additive exports rather than vendoring patched binaries.**
  Building Chromium/PDFium here is out of scope; the shim makes symbol presence
  orthogonal so the same build accepts stock and patched libraries.
- **Reencode onto semantic Unicode slots (visual fix, textual continuity).**
  DOM/copy text remains the ToUnicode semantics (`"B"`), while the WOFF2 renders
  the source-shaped outline; wave-2 text gates intentionally keep passing, and
  visual parity is provable later through screenshot baselines. This preserves
  copy-text legitimacy (PDF semantics claim `B`) without inventing new UI
  semantics now.
- **PS-name bridging instead of raw glyph-index addressing.** FontForge scripts
  address glyphs reliably by name after encoding changes; Rust-side `post`
  parsing keeps Python simple and deterministic; missing names fall back to
  codepoint-only selection (documented degradation identical to today's
  behavior).
- **Spec file replaces argv CSV wholesale.** Carrying triples in argv invites
  shell/length hazards; JSON beside the input binary matches the existing
  response-file convention; bootstrap smoke switches to one canonical form.
- **Kerning suppressed in output rather than modeled.** Reproducing browser kern
  selection deterministically exceeds this branch's mandate; disabling aligns
  rendered spacing with the hmtx+compensation model deterministically across
  platforms.

## Risks

- Vendored-crate drift on future dependency bumps → mitigated by pinning
  `=0.9.3` (unchanged) and recording the exact diff list in the provenance doc;
  any upgrade re-applies it mechanically.
- Extern misuse crossing threads → binding lookup happens once behind
  initialization used by existing library handling; all declarations take plain
  integers/handles already guaranteed valid by the owning objects within the
  same call scope.
- FontForge `post` name formats beyond Mac Roman → resolver returns `None`,
  which degrades to legacy behavior, so exotic-but-nameless fonts never fail a
  conversion that succeeds today.
- Reencoding edge cases (slot collisions: two records sharing a semantic
  Unicode) → Rust deduplicates records by Unicode target before writing the
  spec, keeping the last identity-bearing record, matching how downstream HTML
  consumes one char per slot.
- Width array misalignment if slots reorder → single construction site orders
  records exactly once, validated by count checks plus fake-helper assertions
  on ordering.

## Validation

- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D
  warnings`, `cargo test` (unit suite, `fontforge_worker`, `corpus_manifest`,
  cleancode files) — all green.
- `python3 tools/generate_corpus.py` twice byte-stable;
  `python3 -m unittest tests/embedded_font_fixtures.py`;
  `python3 tools/corpus.py validate` green with untouched adversarial
  expectations.
- New focused tests: PS-name resolution against fixture TTFs; request-spec
  construction with and without identity (assert exact schema + fallback);
  worker validation rejecting malformed responses under the new schema;
  compensation values sourced from processed metrics; kerning CSS presence;
  pixel-threshold behavior boundary.
- Degradation proof: with the stock `.runtime` binary present, a diagnostic run
  of the mismatch fixture completes with null identity fields and unchanged
  classification/fallback decisions; absent runtime, unit-level shims prove the
  same via controlled dynamic lookups.
- Diff review confirms no other serializer/output drift except documented CSS
  addition, and vendor diff limited to published accessors.
