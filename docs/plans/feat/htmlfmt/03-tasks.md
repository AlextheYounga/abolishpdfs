# Tasks

## Implementation

- [ ] Replace object-local run preparation with ordered page-level grouping that
      merges compatible native glyphs across text-object boundaries and preserves
      all existing run-boundary rules.
- [ ] Recover a pending empty native text object as one semantic space only when
      compatible visible glyphs continue the same prepared run.
- [ ] Extend run spacing calculations to include semantic-space font advances and
      preserve each following source glyph's observed PDF position.
- [ ] Include one U+0020 record in embedded font jobs when the source font exposes
      a space glyph, preserving all explicit glyph-identity records.
- [ ] Pretty-print structural HTML without introducing whitespace inside text-run
      content or changing writer ordering and escaping.

## Validation

- [ ] Add and pass run tests for cross-object merging and every protected boundary:
      style, baseline, transform, render mode, direction, and large gap.
- [ ] Add and pass run tests for one recovered semantic space, discarded boundary
      empties, and exact post-space glyph positioning compensation.
- [ ] Add and pass font-job tests showing U+0020 is retained once alongside source
      glyph identities and metrics.
- [ ] Add and pass writer tests for indentation and contiguous escaped run text.
- [ ] Convert `AlexYounger.pdf`; verify zero integrity failures, the literal text
      `Austin Texas 78735`, and at most 150 `.text-run` spans.
- [ ] Run browser clipboard, asset, native-text, and screenshot validation against
      the converted resume and confirm the faithful page view remains acceptable.
- [ ] Run `cargo fmt --check`, `cargo test --lib`, and
      `cargo clippy --all-targets --all-features -- -D warnings` successfully.

## Completion

- [ ] Run `git diff --check` and review the final diff for debug code, generated
      artifacts, unrelated changes, obsolete object-local logic, and accurate
      planning records.
- [ ] Record final run-count, copied-text, browser, and automated validation
      evidence in the completion notes.

## Completion notes
