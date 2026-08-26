# Tasks

## Implementation

- [ ] Extend `tools/embedded_font_pdf.py` with deterministic one-byte
  ToUnicode CMap support without changing existing fixture bytes.
- [ ] Generate `embedded-tounicode-mismatch.pdf` from Liberation Sans with PDF
  code `0x41`, ToUnicode `B`, and visible source glyph `A`; add complete
  manifest expectations for its semantic/visual disagreement.
- [ ] Extend `tests/embedded_font_fixtures.py`, `tests/corpus_manifest.rs`, and
  `tools/corpus.py` to prove and validate adversarial mapping metadata,
  deterministic bytes, source code, and CMap content.
- [ ] Correct `tools/browser_corpus.py` to observe `.text-run` output and add
  explicit loaded processed-font, selection, clipboard, navigation, asset, and
  structured failure assertions.
- [ ] Capture, review, and track one pinned-runtime first-page baseline per
  generated fixture, then promote every manifest screenshot to `compare`.
- [ ] Update `CONTEXT.md` with the Wave 2 gate, approved baseline contract, and
  adversarial fixture evidence for Wave 3.

## Validation

- [ ] Run fixture generation twice and
  `python3 -m unittest tests/embedded_font_fixtures.py` with stable bytes and a
  proven `0x41`/`B`/visible-`A` mismatch.
- [ ] Run `cargo test --test corpus_manifest` and
  `python3 tools/corpus.py validate` with complete mapping and baseline fields.
- [ ] Run `./tools/run-integration` once to create reviewed candidate captures
  and again after promotion to prove all diagnostic and browser assertions pass
  against tracked baselines.
- [ ] Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets
  --all-features -- -D warnings`, and `git diff --check` successfully.

## Completion

- [ ] Review all baseline images and the final diff for pinned provenance,
  bounded tolerances, no capture-only screenshots, no production glyph-mapping
  changes, and no weakened existing assertions.
- [ ] Record exact validation results, exposed converter behavior, and any
  deliberate Wave 3 deferral in these completion notes.

## Completion notes

Implementation has not started.
