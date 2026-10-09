# Tasks

## Implementation

### Phase A — pdfium-glyph-identity

- [x] A1. Vendor pdfium-render 0.9.3 verbatim into `vendor/pdfium-render/`,
      wire `[patch.crates-io]` in `Cargo.toml`, apply the minimal accessor
      publication diff (text-page and ownership handles used by extraction),
      record provenance + diff list in `patches/pdfium/README.md`.
- [x] A2. Add `src/pdfium/identity.rs`: extern declarations for
      `FPDFText_GetCharCode`/`FPDFText_GetGlyphId`, dynamic symbol lookup with
      graceful `None`, SAFETY documentation on all unsafe blocks.
- [x] A3. Extend `Glyph` with `source_code: Option<u32>`, `glyph_id:
      Option<u16>`; extend `FontSource` with identity triples; populate during
      extraction in the existing char loop; extend capability report/probe to
      reflect resolved symbols.
- [x] A4. Update test constructors/serializers for new fields (model, output,
      probe tests) so existing assertions hold.

### Phase B — fontforge-glyph-remapping

- [x] B1. Implement `src/fonts/psnames.rs`: glyph-ID → PostScript name from
      `post` table data via ttf-parser; unit tests against the pinned
      Liberation Sans TTF and a synthetic nameless font fallback case.
- [x] B2. Replace the Unicode-CSV request path: build the JSON spec file per
      job (records = deduplicated semantic Unicode slots carrying PS names,
      codes, GIDs when known), drop CSV construction, update argument list,
      validation, and error types in `src/fonts/worker.rs`.
- [x] B3. Rewrite `tools/fontforge_worker.py`: read spec file, select by PS
      name with codepoint fallback, reencode onto semantic slots, keep
      `.notdef`, emit `{family_name, glyph_count, units_per_em,
      advance_widths}` ordered by spec records.
- [x] B4. Update `src/bin/fontforge_test_helper.rs` +
      `tests/fontforge_worker.rs` for the spec protocol including ordering and
      rejection cases; update the bootstrap smoke call in
      `tools/bootstrap-runtime`.
- [x] B5. Carry processed metrics into `ProcessedFont` (units_per_em + keyed
      advances) and thread through model preparation.

### Phase C — browser-font-metrics foundations

- [x] C1. Swap compensation source chain to processed metrics first, then
      source bytes, then none; unit tests with synthetic catalogs asserting
      chosen values and fallback behavior.
- [x] C2. Introduce dedicated pixel epsilon constant for letter-spacing
      emission (replacing `projection::EPSILON` reuse); boundary test at the
      threshold.
- [x] C3. Emit `font-kerning:none` on text-run styling; update writer tests.

## Validation

- [x] V1. `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings`.
- [x] V2. `cargo test` full suite green (unit + integration incl. corpus_manifest).
- [x] V3. `python3 tools/generate_corpus.py` twice byte-stable;
      `python3 -m unittest tests/embedded_font_fixtures.py`;
      `python3 tools/corpus.py validate` green unchanged for adversarial gates.
- [x] V4. Diagnostic degradation check: run the mismatch fixture against stock
      libpdfium.so when available in `.runtime/`; confirm null identity fields
      and stable classification/fallback output (skip documented when runtime
      absent).
- [x] V5. Diff review: vendor tree limited to published accessors; no HTML/CSS
      drift beyond kerning property; serializer changes limited to new fields.

## Completion

- [x] C6. Update `CONTEXT.md`: capability status of PDFium exports, patch
      boundary, worker protocol v2, metric model notes.
- [x] C7. Fill completion notes here: deviations, discoveries, deliberately
      deferred work (patched-binary build/run, screenshot baselines).

## Completion notes

Implemented across three phases by delegated subagents, verified independently.

Deviations from the plan:

- `src/pdfium/navigation.rs` was split out of the extractor during Phase A to
  keep files under 400 lines; a mechanical move with no behavior change.
- Run tests moved to `src/text/runs_tests.rs` in Phase C for the same reason.
- The identity shim adds a direct `libloading` dependency to the root manifest.
- Vendored pdfium-render keeps two upstream dead-code warnings; the verbatim
  import policy forbids touching unrelated crate code. Workspace targets build
  warning-free otherwise (`clippy -D warnings` green).
- V4 executed as a documented skip: no `.runtime/pdfium-7881/lib/libpdfium.so`
  exists in this worktree (bootstrap could not complete without host FontForge),
  and offline tests already prove the disabled-source path. Re-run when runtime
  bootstrap succeeds.

Discoveries recorded along the way:

- Liberation Sans glyph IDs used by the new resolver tests: gid 36 -> "A",
  gid 37 -> "B" (discovered from the pinned fixture's post table). Processed
  fonts are stored as `FontSource.processing: FontProcessingState::Ready(...)`;
  run preparation consults that boundary for processed advances.
- Glyph remapping semantics keep DOM/copy text equal to ToUnicode semantics:
  wave-2 text gates stay green by design; visual parity proof belongs to future
  screenshot baselines against a patched-PDFium runtime.

Deliberately unfinished work:

- Building and running an actually patched PDFium binary (the patch file and
  recipe live under `patches/pdfium/`), executing the real FontForge worker
  end-to-end here (host lacks FontForge), and promoting any screenshot
  baselines. All are runtime-environment constraints, not design gaps.

Validation results: cargo fmt/clippy/test green (77 tests pass), corpus gates
green twice byte-stable, embedded-fixture unittest green, corpus validate green,
fontforge_worker.py compiles and was hardened after review (defensive PS-name
lookup plus a zero-width width sentinel keeping response widths aligned).
