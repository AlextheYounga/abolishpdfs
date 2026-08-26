# Plan

## Current behavior

`tools/run-integration` already bootstraps the pinned runtime, builds the Rust
binary, regenerates fixtures, runs Rust and diagnostic checks, converts every
generated PDF, and invokes `tools/browser_corpus.py`. Embedded TrueType
fixtures require WOFF2 assets and the writer emits native text as `.text-run`
elements with processed `pdf-font-*` families.

`tools/browser_corpus.py` currently searches for `.text-glyph`, so its native
text assertion cannot observe writer output. It simulates select-all and copy
but does not independently assert selection. It checks required font files on
disk but not `document.fonts`, and all manifest screenshots are `capture-only`
with no tracked baselines.

`tools/embedded_font_pdf.py` embeds committed Liberation TTF bytes behind a
simple WinAnsi TrueType font dictionary. It does not emit ToUnicode CMaps.
`tests/embedded_font_fixtures.py` verifies embedded programs and deterministic
regeneration, while `tests/corpus_manifest.rs` and `tools/corpus.py` validate
manifest structure. Production mapping accepts a PDFium Unicode value when the
same codepoint resolves to a nonzero glyph in the embedded cmap; PDFium-render
does not expose the source PDF code or selected glyph ID.

## Intended behavior

The browser gate reads text from `.text-run`, waits for the page and all fonts
to settle, verifies each required processed font asset is represented by a
loaded `FontFace`, separately records selected and clipboard text, and compares
every first-page screenshot with a tracked approved baseline. Its JSON result
identifies each observed behavior and any precise failure.

The embedded-font PDF builder can attach a deterministic one-byte ToUnicode
CMap to a font. The new adversarial fixture writes PDF code `0x41`, keeps
Liberation Sans' code-to-glyph behavior so the visible source glyph is `A`, and
maps that code to semantic Unicode `B`. Manifest and integrity tests identify
the source code, ToUnicode text, and visible source text. Runtime checks expect
native `B` text and a baseline of that actual converted output. The offline
fixture proof records that the PDF source glyph is `A`, making the current
inference failure explicit while leaving production behavior unchanged.

## Approach

Extend the existing fixture manifest rather than creating a second registry.
Add optional adversarial mapping expectations only to the new fixture and make
both Rust and Python validators enforce their internal consistency.

Add a narrowly scoped ToUnicode option to the existing PDF builder. Verify the
actual CMap stream and content-stream source code in the Python fixture test so
the adversarial property cannot silently disappear during generator changes.

Refactor the browser checker only enough to give font, text, selection, and
screenshot observations focused functions. Use Playwright's browser APIs and
`document.fonts` directly. Generate all output with `tools/run-integration`,
review its first-page captures, copy those captures into the tracked baseline
directory, promote manifest entries to `compare`, and rerun the gate against
the tracked images.

## Responsibilities and boundaries

- `tools/embedded_font_pdf.py` owns deterministic PDF font dictionaries and
  ToUnicode stream construction.
- `tools/generate_corpus.py` owns the adversarial fixture's content and output
  name.
- `tests/embedded_font_fixtures.py` owns proof of embedded font bytes,
  deterministic generation, source PDF code, and ToUnicode mismatch.
- `tests/fixtures/manifest.json` owns semantic text, visible source text,
  required fonts/assets, and approved screenshot expectations.
- `tests/corpus_manifest.rs` and `tools/corpus.py` own static manifest contract
  validation.
- `tools/browser_corpus.py` owns browser-observed DOM, loaded font, selection,
  clipboard, navigation, assets, and screenshots.
- `tools/run-integration` remains the thin end-to-end orchestrator.
- `src/` remains unchanged because Wave 2 establishes evidence rather than
  implementing Wave 3 glyph identity or remapping.

## Affected areas

- `tools/embedded_font_pdf.py`
- `tools/generate_corpus.py`
- `tools/browser_corpus.py`
- `tools/corpus.py`
- `tests/embedded_font_fixtures.py`
- `tests/corpus_manifest.rs`
- `tests/fixtures/manifest.json`
- `tests/fixtures/generated/embedded-tounicode-mismatch.pdf`
- `tests/fixtures/baselines/*.png`
- `CONTEXT.md`
- `docs/plans/feat/wave-2/*.md`

## Decisions

- Represent the mapping limitation with a passing exposure test instead of an
  expected failure, so CI remains actionable while preserving evidence that
  Wave 3 must solve.
- Use a ToUnicode mismatch because it exercises the exact unverified boundary
  with the existing committed font and requires no speculative custom font
  format or production API.
- Validate loaded fonts through `document.fonts` rather than only checking WOFF2
  files, because an asset can exist while CSS loading or decoding fails.
- Assert selection independently from clipboard because clipboard permissions
  and selection semantics are separate browser behaviors.
- Baseline every generated fixture's first page to remove the capture-only gap
  and keep the existing bounded corpus gate simple.
- Keep baseline creation manual within the pinned integration run; the checker
  never auto-accepts changed visual output.

## Risks

- FontFace family serialization can differ in quoting; compare normalized
  family names and loaded status rather than raw CSS strings.
- Select-all may include navigation text or browser-dependent line breaks;
  retain fixture substring expectations while separately requiring native run
  selection.
- Screenshots can drift with an unpinned browser, font processor, or PDFium;
  use only the runtime manifest versions and retain nonzero perceptual
  tolerances.
- The ToUnicode fixture can accidentally render `B` instead of `A` if its font
  encoding changes; inspect both content code and CMap bytes offline and retain
  its approved visual baseline.
- Existing outputs may expose real converter defects when the dormant gate is
  first run. Fix only Wave 2 harness or fixture defects; record production
  mapping limitations for Wave 3 rather than hiding them.

## Validation

- Regenerate fixtures twice and run
  `python3 -m unittest tests/embedded_font_fixtures.py`; all bytes are stable and
  the adversarial PDF proves code `0x41` maps to Unicode `B` while using the
  embedded font's `A` source code.
- Run `cargo test --test corpus_manifest` and
  `python3 tools/corpus.py validate`; every baseline exists and adversarial
  mapping expectations are complete.
- Run `./tools/run-integration` with the pinned runtime; every fixture passes
  diagnostics, output conversion, loaded-font, DOM, selection, clipboard,
  navigation, asset, and screenshot comparisons.
- Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets
  --all-features -- -D warnings`, and `git diff --check` successfully.
- Review baseline images and the final diff to confirm they came from the
  pinned successful run, no capture-only entries remain, production Rust code
  is unchanged, and the mapping disagreement is not described as correct
  conversion.
