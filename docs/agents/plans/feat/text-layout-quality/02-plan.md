# Plan

## Current behavior

`src/model/text.rs` stores glyphs inside text objects, while `src/text/projection.rs` normalizes individual text-object matrices and `src/output/document.rs` emits positioned glyph spans. There is no model-level run formation, browser advance measurement, letter-spacing compensation, or explicit layout-break representation.

## Intended behavior

Text preparation converts compatible glyph sequences into ordered runs with normalized style, baseline, transform, and spacing metadata. The writer emits those runs using CSS geometry and compensation. Incompatible glyphs begin a new run or receive an explicit reconstruction failure rather than inheriting unrelated style.

## Approach

Add a text-layout preparation stage after PDFium extraction and before output. It will preserve source order, group only compatible adjacent glyphs, calculate expected versus observed advances, choose run-level letter spacing, and encode local discontinuities as positioned spans. Existing projection math remains the transform foundation, with tests covering each supported matrix class.

## Responsibilities and boundaries

- `src/pdfium` supplies observed glyph geometry and paint facts.
- `src/text` owns compatibility, run formation, spacing, and layout-break decisions.
- `src/model/text.rs` stores prepared runs and compensation values.
- `src/output/document.rs` serializes prepared runs into HTML/CSS.
- Corpus tooling owns visual, selection, and copied-text observations.

## Affected areas

- `src/text/mod.rs` and new focused text-layout modules
- `src/text/projection.rs`
- `src/model/text.rs`
- `src/output/document.rs` and document tests
- Geometry and text fixtures in `tests/fixtures/`
- Browser corpus expectations and project documentation

## Decisions

- Preserve source text-object and glyph order as the primary ordering rule.
- Merge only adjacent glyphs with compatible normalized transform, baseline, font, paint, progression, and visibility state.
- Use run-level `letter-spacing` for stable compensation and explicit positioned spans for local discontinuities.
- Start a new run at every layout break rather than carrying style or spacing across it.

## Risks

- Incorrect tolerances can merge distinct text or split ordinary words; corpus fixtures will establish and protect them.
- Browser font metrics differ from PDF metrics, so compensation must be measured with the processed font when the FontForge branch lands.
- More spans can increase output size; correctness takes priority, with optimization deferred until measurements show a problem.

## Validation

- Unit tests cover run compatibility, baseline progression, matrix normalization, spacing, and layout breaks.
- Output tests verify run styles and CSS compensation are serialized correctly.
- Browser corpus checks compare screenshots, selection order, and copied text for spacing and transform fixtures.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check` pass.
