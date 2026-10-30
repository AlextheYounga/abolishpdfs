# Plan

## Current behavior

`src/text/runs.rs::prepare_page()` invokes `prepare_object()` independently for
every `TextObject`. `prepare_object()` can merge compatible glyphs only within
that object. Its `continues()` predicate already enforces style, normalized
transform, baseline, direction, and gap compatibility, and `build_run()` already
derives browser placement, letter spacing, and exceptional local offsets.

The resume fixture has 2,501 native text objects: 2,242 contain visible glyphs
and 264 are empty positioning objects. Because Chrome emitted nearly every glyph
as a separate object, the generated HTML has 2,237 text-run spans. Empty objects
are intentionally harmless for text integrity but disappear during run
preparation, so word spaces are absent from run text.

`src/output/document.rs` serializes prepared runs directly into the faithful page
view. It emits compact markup through string concatenation and never makes layout
decisions. `tools/fontforge_worker.py` subsets embedded fonts from glyph records
prepared by `src/fonts/worker.rs`; synthesized spaces therefore need an explicit
space record in processed fonts.

## Intended behavior

Run preparation will consume the page's ordered native text objects as one
sequence. Adjacent compatible glyphs will remain in one prepared run even when
they came from different PDF text objects. Empty native objects will mark a
pending semantic space; that space will be inserted only when the next visible
glyph continues the current run. A run boundary discards the pending marker.

Prepared runs will contain literal spaces and spacing compensation that places
every following source glyph at its observed PDF position. Embedded font subsets
will include U+0020 so browser metrics for the inserted character come from the
same processed font. The writer will emit indented structural markup while
keeping each text run's content contiguous.

## Approach

Replace object-local grouping in `src/text/runs.rs` with a page-level run builder.
The builder will preserve source order, skip non-native objects, track empty
native objects as pending space markers, and use the existing `continues()` rules
for every visible glyph. It will retain the first contributing object's source
and render mode as the run identity; render-mode incompatibility will be included
in the continuation decision.

Represent inserted spaces as private run-building characters alongside source
glyphs. Compute observed source-glyph positions and browser advances across that
character sequence so the existing median letter-spacing and local-offset model
also compensates for the space's natural advance. Keep this representation
private to text preparation; emit the existing `PreparedRun` model.

Add a U+0020 fallback record to embedded font job specifications when the source
font exposes a space glyph and no space record already exists. This makes the
processed font's reported space advance available to run preparation and keeps
browser rendering deterministic.

Refactor the writer's small rendering functions to accept indentation depth and
emit newlines between structural siblings. Do not insert formatting whitespace
inside a `text-run` span or between local offset spans and their characters.

## Responsibilities and boundaries

- `src/text/runs.rs` owns cross-object compatibility, pending-space recovery,
  run construction, and spacing compensation.
- `src/fonts/worker.rs` owns ensuring that the processed font job contains the
  synthetic character needed by prepared runs.
- `tools/fontforge_worker.py` continues to execute the supplied font job without
  making layout or semantic decisions.
- `src/model/text.rs` remains the output boundary for prepared runs; no semantic
  document hierarchy is added.
- `src/output/document.rs` owns readable serialization only and must not infer or
  regroup text.

## Affected areas

- `src/text/runs.rs`
- `src/text/runs_tests.rs`
- `src/fonts/worker.rs`
- Font worker unit tests colocated with `src/fonts/worker.rs`
- `src/output/document.rs`
- `src/output/document_tests.rs`

## Decisions

- Merge at the prepared-run layer rather than in PDF extraction. PDF text-object
  identity remains available for diagnostics, while layout normalization stays
  in its established module.
- Use existing geometric compatibility as the run boundary. This gains compact
  output without introducing linguistic heuristics.
- Treat empty native objects only as pending spaces. This recovers Chrome's
  positioning-encoded spaces while avoiding invented text at line or style
  boundaries.
- Preserve one faithful text layer. A second reading layer would duplicate
  search results, accessibility content, and selection behavior.
- Keep `PreparedRun` as the writer contract. The private richer build sequence is
  an implementation detail needed to calculate spacing correctly.
- Pretty-print only structural boundaries. Text content remains byte-for-byte
  controlled by run preparation and escaping.

## Risks

- Over-merging can combine separate visual lines or styles. Existing continuation
  checks and render-mode compatibility guard those boundaries.
- Inserted spaces can shift later glyphs if their font advance is not included in
  compensation. The font job and spacing tests must prove exact expected
  advances.
- Empty positioning objects can occur outside words. Pending spaces are discarded
  unless compatible visible glyphs occur on both sides in the same run.
- Newlines used for source formatting can become selectable text when placed
  inside text spans. Writer tests must prove text-run content has no formatting
  whitespace.
- Longer runs may expose browser shaping differences. Kerning remains disabled,
  and local offsets retain observed PDF positioning.

## Validation

- Unit tests prove compatible glyphs merge across text objects; style, baseline,
  transform, render-mode, and large-gap boundaries remain separate.
- Unit tests prove an empty native object between compatible glyphs produces one
  literal space, while empties at run boundaries produce none.
- Spacing tests prove inserted spaces retain the observed position of the next
  source glyph and processed-font space metrics are present.
- Font-job tests prove U+0020 is included once without replacing source glyph
  identity records.
- Writer tests prove structural indentation, contiguous text-run content, HTML
  escaping, and existing document-order behavior.
- The resume conversion succeeds with no text-integrity failures, contains
  `Austin Texas 78735`, and emits no more than 150 text-run spans instead of
  2,237. A prototype using the existing continuation rules predicts 99 spans.
- Browser validation confirms native text remains searchable/selectable, copied
  text contains reconstructed spaces, required assets load, and the rendered
  page remains within the configured screenshot tolerance.
- `cargo fmt --check`, `cargo test --lib`, and
  `cargo clippy --all-targets --all-features -- -D warnings` pass.
- `git diff --check` and final diff review show no debug code, generated output,
  unrelated changes, or stale planning records.
