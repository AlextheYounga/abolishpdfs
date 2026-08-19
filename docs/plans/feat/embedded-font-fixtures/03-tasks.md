# Tasks

## Implementation

- [x] Add the exact Liberation Sans and Liberation Serif 2.1.5 TTF inputs, license text,
  source URLs, checksums, and redistribution statement under
  `tests/fixtures/fonts/`.
- [x] Implement `tools/embedded_font_pdf.py` with deterministic PDF objects for
  embedded TrueType resources, font descriptors, text streams, image and
  vector paint, and URI annotations.
- [x] Extend `tools/generate_corpus.py` to generate the five named
  `embedded-*.pdf` fixtures without changing existing fixture bytes.
- [x] Add `tests/embedded_font_fixtures.py` to parse FontFile2 resources, verify
  TrueType signatures and declared font identities, and assert byte-for-byte
  deterministic regeneration.
- [x] Add complete manifest entries for all five embedded-font fixtures,
  including source fonts, expected processed assets, processing status,
  backgrounds, native text, clipboard text, links, page sizes, and screenshot
  tolerances.
- [x] Extend `tests/corpus_manifest.rs`, `tools/corpus.py`, and
  `tools/browser_corpus.py` to validate and consume the new font-specific
  manifest fields without weakening existing fixture rules.
- [x] Update `CONTEXT.md` with generation commands, provenance, fixture purpose,
  external-tool requirements, and interpretation of runtime failures.

## Validation

- [x] Run `python3 tools/generate_corpus.py` twice and confirm identical bytes
  for every generated fixture.
- [x] Run `python3 -m unittest tests/embedded_font_fixtures.py` and confirm the
  declared FontFile2 streams contain actual TrueType signatures.
- [x] Run `cargo test --test corpus_manifest` and `python3 tools/corpus.py
  validate` with every fixture present and every manifest expectation complete.
- [x] Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets
  --all-features -- -D warnings`, and `git diff --check`.
- [ ] Run the diagnostic corpus command with configured PDFium and record
  page, text, link, background, and font-processing results for each new
  fixture.
- [ ] Run the browser corpus command with configured FontForge and Playwright
  and record native text, copied text, generated assets, links, and screenshot
  results for each new fixture.

## Completion

- [x] Review the final diff to confirm no converter implementation, FontForge
  policy, PDFium binding, or existing assertion was changed.
- [x] Confirm generated PDFs, committed fonts, license text, and provenance are
  all present and no arbitrary external PDF was added.
- [x] Record fixture inventory, isolated behavior, provenance, runtime
  pass/fail status, exposed converter assumptions, and any deliberately
  deferred runtime checks in completion notes.

## Completion notes

Implemented five deterministic embedded-font PDFs using the official Liberation
Fonts 2.1.5 TTF release archive. The fixtures cover basic TrueType embedding,
subset-style naming, multiple fonts, repeated characters at different sizes and
rotation, and a two-page vertical slice with spacing, image, vector, and URI
content. The integrity test verifies FontFile2 TrueType signatures and repeated
generation equality. Manifest validation, `cargo test --test corpus_manifest`,
and the focused Python tests pass.

Rust formatting, all tests, clippy, focused integrity tests, and manifest
validation pass. PDFium/FontForge/browser corpus runs remain to be executed
because those runtime checks require external tools; they are intentionally not
represented as successful fixture behavior yet.
