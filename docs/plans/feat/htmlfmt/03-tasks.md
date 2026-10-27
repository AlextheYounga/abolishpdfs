# Tasks

## Implementation

- [x] Replace object-local run preparation with ordered page-level grouping that
      merges compatible native glyphs across text-object boundaries and preserves
      all existing run-boundary rules.
- [x] Recover a pending empty native text object as one semantic space only when
      compatible visible glyphs continue the same prepared run.
- [x] Extend run spacing calculations to include semantic-space font advances and
      preserve each following source glyph's observed PDF position.
- [x] Include one U+0020 record in embedded font jobs when the source font exposes
      a space glyph, preserving all explicit glyph-identity records.
- [x] Pretty-print structural HTML without introducing whitespace inside text-run
      content or changing writer ordering and escaping.

## Validation

- [x] Add and pass run tests for cross-object merging and every protected boundary:
      style, baseline, transform, render mode, direction, and large gap.
- [x] Add and pass run tests for one recovered semantic space, discarded boundary
      empties, and exact post-space glyph positioning compensation.
- [x] Add and pass font-job tests showing U+0020 is retained once alongside source
      glyph identities and metrics.
- [x] Add and pass writer tests for indentation and contiguous escaped run text.
- [x] Convert `AlexYounger.pdf`; verify zero integrity failures, the literal text
      `Austin Texas 78735`, and at most 150 `.text-run` spans.
- [ ] Run browser clipboard, asset, native-text, and screenshot validation against
      the converted resume and confirm the faithful page view remains acceptable.
- [x] Run `cargo fmt --check`, `cargo test --lib`, and
      `cargo clippy --all-targets --all-features -- -D warnings` successfully.

## Completion

- [x] Run `git diff --check` and review the final diff for debug code, generated
      artifacts, unrelated changes, obsolete object-local logic, and accurate
      planning records.
- [x] Record final run-count, copied-text, browser, and automated validation
      evidence in the completion notes.

## Completion notes

- Page-level preparation now merges compatible glyphs across native text objects,
  recovers only pending empty-object spaces, and compensates those spaces with
  processed-font metrics. Non-native objects terminate active native runs and
  discard pending spaces. The writer emits indented structural markup while each
  text run remains one contiguous line.
- `cargo fmt --check`, `cargo test --lib` (67 passed),
  `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check`
  passed on 2026-10-26.
- Conversion of `AlexYounger.pdf` completed successfully with the local patched
  PDFium and FontForge. The output has 99 `.text-run` spans. Chromium DOM text
  contains `Austin Texas 78735`; the generated page screenshot rendered the
  faithful page view and all four generated assets are present.
- Revalidated the conversion on 2026-10-26 against the corrected implementation:
  zero text-integrity failures, 99 `.text-run` spans, the literal present in
  Chromium DOM text, and four generated assets.
- The browser corpus clipboard validation remains unfinished: the system Chromium
  is available, but the required Python `playwright` package and managed browser
  runtime are not installed. No copied-text assertion or configured screenshot
  tolerance comparison was therefore run.
- Regression coverage verifies that non-native objects terminate runs and discard
  pending spaces, and that reverse progression under a cross-object rotated
  transform remains a run boundary.
