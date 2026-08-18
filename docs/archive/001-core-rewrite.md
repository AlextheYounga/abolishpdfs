# Core Rewrite Plan

This should be a behavioral rewrite, not a source translation. The useful ideas
in pdf2htmlEx are its font correction, text geometry, selective background
rendering, and fallback policy. Its callback-driven state machine and
`HTMLRenderer` god object should not survive.

## Reference Pipeline

pdf2htmlEx currently performs:

1. A preprocessing render pass to collect used font codes and page dimensions
   in `Preprocessor.cc:43`.
2. A Poppler `OutputDev` pass that reconstructs text in
   `HTMLRenderer/text.cc:26`.
3. State comparison and line formation in `HTMLRenderer/state.cc:168`.
4. A second page render that suppresses reconstructable text and retains
   covered or unsupported text in
   `BackgroundRenderer/SplashBackgroundRenderer.cc:56`.
5. Font extraction, Unicode remapping, width correction, and web-font
   generation in `HTMLRenderer/font.cc`.
6. HTML, CSS, links, outlines, forms, and asset assembly in
   `HTMLRenderer/general.cc:210`.

The most valuable algorithm is the text transform normalization and position
compensation in `HTMLRenderer/state.cc:241-410`. The most important behavior is
selective background text rendering in
`SplashBackgroundRenderer.cc:56-64`.

## Proposed Architecture

```text
CLI
 |
 v
Conversion coordinator
 |
 +-- PDFium worker
 |    +-- document metadata
 |    +-- page/object extraction
 |    +-- character extraction
 |    +-- selective background rendering
 |
 v
Owned Rust document model
 |
 +-- font usage and mapping
 +-- reconstruction eligibility
 +-- visibility analysis
 +-- text run formation
 +-- links, destinations, outlines
 |
 +--------> FontForge worker
 |             |
 |             v
 |         web fonts
 |
 v
Deterministic HTML/CSS writer
```

### Core Boundaries

```text
src/
  main.rs
  cli.rs
  conversion.rs

  pdfium/
    mod.rs
    document.rs
    page.rs
    text.rs
    objects.rs
    background.rs

  model/
    document.rs
    page.rs
    text.rs
    font.rs
    geometry.rs
    navigation.rs

  text/
    eligibility.rs
    visibility.rs
    runs.rs
    spacing.rs

  fonts/
    catalog.rs
    mapping.rs
    worker.rs

  output/
    document.rs
    page.rs
    css.rs
    assets.rs
```

These are conceptual boundaries, not a requirement to create every file
immediately.

### Ownership Rules

- PDFium handles remain inside the PDFium worker.
- Rust receives only owned values such as `PageModel`, `Glyph`, `FontSource`,
  and `Link`.
- Configuration becomes immutable after CLI validation.
- FontForge runs out of process behind a structured request/response boundary.
- Output generation consumes the model and never queries PDFium.
- Native-library failures become page-, font-, or document-scoped diagnostics.
- No global mutable configuration, shared formatter, or callback state.

## Stable Model

The model should record observed geometry rather than try to recreate Poppler's
graphics-state callback stream.

```rust
struct DocumentModel {
    pages: Vec<PageModel>,
    fonts: FontCatalog,
    outline: Vec<OutlineNode>,
}

struct PageModel {
    number: usize,
    size: Size,
    crop_box: Rect,
    text_objects: Vec<TextObject>,
    graphics: Vec<GraphicsObject>,
    links: Vec<Link>,
}

struct TextObject {
    source: SourceObjectId,
    paint_order: usize,
    glyphs: Vec<Glyph>,
    font: FontId,
    render_mode: TextRenderMode,
    reconstruction: ReconstructionDecision,
}

struct Glyph {
    unicode: Option<char>,
    origin: Point,
    tight_bounds: Rect,
    loose_bounds: Rect,
    transform: AffineTransform,
    font_size: f64,
    fill: Color,
    stroke: Color,
    generated_by_pdfium: bool,
}

enum ReconstructionDecision {
    NativeText,
    Background(FallbackReason),
}
```

`SourceObjectId` only needs to survive long enough to tell the PDFium worker
which text objects to suppress during background rendering. It should not wrap
an FFI handle.

## Intentional Departures From pdf2htmlEx

### 1. Do Not Emulate `OutputDev`

PDFium exposes characters and page objects directly. We should not build a fake
callback system or port dirty flags such as `font_changed`, `text_pos_changed`,
and `all_changed`.

Run formation should operate on:

- Per-character origin and bounds
- Per-character effective matrix
- Owning text object
- Font identity
- Observed distance to the next character
- Paint order

This replaces the implicit state ordering dependency documented in
`HTMLRenderer/state.cc:170`.

### 2. Replace the Preprocessor With Extraction

`Preprocessor.cc` exists because Poppler must render a page to discover used
character codes. The Rust pipeline can collect page dimensions, characters,
text objects, and font usage during model extraction.

A separate prepass is only justified if FontForge work must start before
complete page extraction.

### 3. Make Fallback Explicit and Object-Scoped Initially

pdf2htmlEx decides per character whether text remains in the background.
PDFium can associate characters with text objects, but it cannot publicly
suppress individual glyphs during page rendering.

Initial policy:

- If every glyph in a text object is reconstructable, emit it as HTML and
  suppress the entire object in the background.
- If any glyph requires fallback, retain the entire text object in the
  background.
- Do not emit an invisible duplicate HTML run in the initial implementation.
  Reconsider this only after Milestone 6 selection and clipboard tests show a
  measurable benefit without regressions.

Later, mixed objects could be split or regenerated, but that should not be part
of the first implementation.

### 4. Raster Background First

PDFium provides reliable bitmap page rendering but no direct SVG page renderer.
PDFium does expose page objects, but SVG output would still need explicit
handling for PDF-specific features such as shading, blend modes, soft masks,
and clipping. That effort and fidelity risk do not justify SVG output for v1;
revisit it in Milestone 9 if the corpus demonstrates a clear need.

Initial background support should therefore be:

- PNG as the correctness default
- JPEG as an optional size optimization
- No SVG milestone until text fidelity and bitmap fallback are stable

### 5. Deterministic Output After Parallel Work

pdf2htmlEx assigns CSS class IDs through shared mutable managers in
`StateManager.h`. That conflicts with parallel page processing.

Instead:

1. Extract pages independently.
2. Collect immutable style values.
3. Sort and intern them deterministically during final assembly.
4. Emit stable class IDs independent of worker scheduling.

### 6. Process Isolation Over Native FFI Sharing

Both PDFium and FontForge should be assumed unsafe to share concurrently until
proven otherwise.

Recommended concurrency:

- Coordinator controls bounded worker processes.
- Each PDFium worker loads the document and handles assigned pages.
- FontForge jobs run in a separate bounded queue.
- Rust page analysis and HTML generation may use threads freely after handles
  are gone.

This also contains crashes from malformed PDFs and fonts.

### PDFium Library Deployment

Platform-specific release archives will bundle the pinned PDFium shared
library beside the executable. At startup, the application will explicitly
resolve the library relative to `std::env::current_exe()`. An explicit CLI
option or `ABOLISHPDFS_PDFIUM_PATH` environment variable may override this
location for development and testing. The application will not download PDFium
at runtime or require a system-wide installation by default. This policy
applies to Linux, macOS, and Windows.

Each PDFium worker must load the same pinned library from this resolution
policy. Loading the document independently in each worker is an initial
simplicity choice, not a memory-use guarantee; Milestone 1 must measure the
I/O and memory cost and compare it with a single-worker or shared-input design.

## Text Reconstruction

### Input

Use PDFium's character APIs for:

- Unicode value
- Generated-character flag
- Tight and loose bounds
- Origin
- Effective matrix
- Font size and identity
- Fill and stroke colors
- Render mode
- Owning text object

The exact stability of each field must be recorded by the Milestone 1 probe.
Fields backed by experimental APIs must have an explicit unavailable-value
policy: geometry may fall back to looser bounds, generated-character status may
fall back to treating the character as source text, and missing paint metadata
must prevent native reconstruction rather than silently merge incompatible
runs. The project should pin a tested PDFium build.

### Run Formation

Build runs in two stages:

1. Preserve source text-object and character order.
2. Merge adjacent fragments only when geometry and style are compatible.

Compatibility should consider:

- Equivalent normalized transform
- Same baseline within tolerance
- Same font and paint style
- Forward progression along the baseline
- No significant overlap or reversal
- No clipping or visibility boundary

Port the mathematical idea from `state.cc:268-410`, but apply it to observed
per-character matrices and origins rather than reconstructed `GfxState`.

### Spacing

Prefer natural browser text plus measured compensation:

1. Emit ordinary spaces when PDFium reports real spaces.
2. Ignore PDFium-generated layout spaces as source characters.
3. Compare expected browser advance with the next observed glyph origin.
4. Use `letter-spacing` for a consistent run-wide difference.
5. Use explicit inline offsets only for local discontinuities.
6. Split the run when compensation becomes unstable.

The baseline, overlap, discontinuity, and compensation thresholds are
provisional configuration values. Milestone 0 must establish their initial
values from the compatibility corpus; Milestone 6 may tune them only with
geometry, screenshot, and clipboard regression coverage. Until then, terms
such as "within tolerance", "significant", "local", and "unstable" describe
the algorithm's decision points, not final numeric guarantees.

This is cleaner than reproducing the nested offset spans in `HTMLTextLine.cc`
wholesale.

### Copy Behavior

Visual Unicode and copied Unicode may conflict for malformed fonts. The model
should retain:

- Display code point used by the web font
- Semantic Unicode used for copy/search
- Original PDFium Unicode and mapping-error status

Do not commit to a dual-layer implementation until browser experiments
establish acceptable selection behavior.

## Font Pipeline

Keep the behavior of `HTMLRenderer/font.cc`, but isolate it from document
rendering.

### Font Catalog

Collect per font:

- Stable document-local identity
- Embedded status and decoded font bytes
- Used Unicode values
- Used glyph identifiers, if available
- PDF-reported widths
- Ascent, descent, weight, and flags
- Documents/pages using the font

### FontForge Worker

The worker should perform one transactional job per font:

1. Load the extracted font.
2. Retain used glyphs.
3. Assign browser-safe Unicode mappings.
4. Correct advances to match PDF geometry.
5. Add a space glyph when required.
6. Normalize ascent, descent, and line gap.
7. Remove problematic font names and permissions where policy permits.
8. Optionally hint.
9. Generate WOFF2, or WOFF if FontForge/platform support requires it.
10. Return metrics and diagnostics.

FontForge will run in a child process rather than being linked through its
global C state. Milestone 4 must select its scripting or Python interface based
on headless reliability on all target platforms, reproducible installation, and
whether the interface can express the required font transformations. Python is
not a release dependency unless that evaluation selects the Python interface.

### Critical Font Gap

PDFium exposes:

- Decoded embedded font data
- Unicode text
- Font handles
- Glyph widths and paths by glyph index

The public character APIs inspected so far expose decoded Unicode and geometry,
but do not provide a documented, stable mapping from each extracted character
back to its encoded character code and glyph index. pdf2htmlEx depends on that
mapping heavily in `HTMLRenderer/font.cc`; the exact gap remains a Milestone 1
spike outcome rather than a settled claim that the information is unavailable.

This is the largest technical risk. If the embedded font's `cmap` does not map
PDFium's Unicode back to the correct glyph, FontForge cannot safely reencode it
from public data alone.

The first font spike must determine whether:

- The raw font already has a usable Unicode `cmap`
- PDFium exposes enough information indirectly
- Geometry plus glyph data can provide a safe mapping
- A narrowly scoped PDFium extension is required
- Such fonts must initially fall back to the background

The public-API-only versus narrowly scoped PDFium-extension decision is made at
the end of this spike. Prefer public APIs if the compatibility corpus can prove
the mapping or if background fallback is visually acceptable; consider an
extension only when the remaining corpus failures materially affect fidelity
and cannot be handled by fallback without unacceptable output loss.

## Background And Visibility

### Selective Rendering

pdf2htmlEx suppresses reconstructed text in
`SplashBackgroundRenderer.cc:56-64`. PDFium has no equivalent render flag.

Candidate strategy:

1. Extract the complete model.
2. Mark fully reconstructable text objects.
3. Temporarily deactivate those objects.
4. Render the page bitmap.
5. Reactivate the objects.
6. Verify the document remains unchanged.

`FPDFPageObj_SetIsActive()` is experimental. The removal/reinsertion API is
another candidate but invalidates text-page handles and has more ownership
risk.

### Visibility Analysis

Do not begin by porting `DrawingTracer` and `CoveredTextDetector`.

PDFium page objects are returned in paint order and expose:

- Object bounds
- Recursive Form XObjects
- Path segments
- Clip paths
- Fill and stroke colors
- Images and shading objects

Start with conservative object-level analysis:

- Text under a later opaque object with meaningful overlap falls back.
- Text with clipping render modes falls back.
- Type 3, vertical writing, unsupported transforms, or uncertain fonts fall
  back.
- Text in mixed or unknown transparency contexts falls back.

Bounding-box overlap alone must never be interpreted as proof of complete
coverage. Ambiguous cases should fall back rather than disappear.

Path-accurate coverage or pixel sampling can be introduced after a corpus shows
where the conservative policy is too broad.

## Links, Outlines, And Forms

These should be independent model extractors.

- Links: rectangle, border, URI, local destination, remote destination
- Destinations: page plus XYZ/Fit/FitH/FitV/FitR semantics
- Outlines: recursive title and action model
- Forms: defer interactive reconstruction until normal page output is stable

Unlike pdf2htmlEx, do not write HTML directly while walking PDFium objects.

## Output

### Initial Artifact Contract

```text
output/
  index.html
  document.css
  pages/
    1.html
    2.html
  assets/
    page-1.png
    font-<id>.woff2
```

Single-file output should be an assembly mode over the same artifacts, not a
separate rendering pipeline.

### HTML Principles

- Absolute page container in PDF points scaled consistently to CSS pixels
- Background image first
- Native text runs above it
- Link hit regions above text
- Accessibility text generated from the stable model
- All text and attribute content escaped through structured writers
- Bundled static CSS and JavaScript with `include_str!`, not a line-oriented
  manifest language

## Milestones

### Milestone 0: Compatibility Corpus

Before feature work:

- Import representative pdf2htmlEx test PDFs where licensing permits.
- Add purpose-built PDFs for matrices, spacing, fonts, clipping, transparency,
  links, and page boxes.
- Establish browser screenshot and copied-text harnesses.
- Store expected feature classifications, not only screenshots.

### Milestone 1: PDFium Capability Spikes

Build disposable probes for:

1. Character extraction and owning text-object association.
2. Embedded font data and Unicode-to-glyph mapping.
3. Text-object deactivate/render/reactivate behavior.
4. Recursive Form XObjects and paint ordering.
5. Cross-platform PDFium loading and version pinning.

Do not build the production architecture until these gates pass.

### Milestone 2: Stable Model And Diagnostic Converter

- Load a document.
- Produce JSON-like diagnostic output from owned Rust structures.
- Record pages, objects, characters, fonts, links, and fallback reasons.
- Ensure no PDFium handles cross the adaptation boundary.

### Milestone 3: Basic Native Text

- Emit page containers and per-text-object runs.
- Handle ordinary horizontal text, transforms, colors, and basic spacing.
- Use system fonts temporarily.
- Add geometry and browser screenshot tests.

### Milestone 4: FontForge Pipeline

- Process straightforward embedded TrueType and OpenType fonts.
- Generate browser fonts and `@font-face`.
- Correct widths and metrics.
- Fall back explicitly for unsupported CID, CFF, Type 1, and Type 3 cases.
- Expand support one format at a time.

### Milestone 5: Selective Raster Background

- Deactivate reconstructable text objects.
- Render remaining page content through PDFium.
- Restore objects safely.
- Verify no duplicate text and no missing fallback glyphs.
- Add PNG embedding and external assets.

### Milestone 6: Layout Quality

- Merge compatible objects into runs.
- Normalize transforms.
- Infer letter spacing and local offsets.
- Improve spaces, ligatures, selection, and copied text.
- Add vertical writing only if browser-native behavior proves adequate.

### Milestone 7: Visibility And Fallback

- Add paint-order and clipping analysis.
- Add conservative overlap decisions.
- Add transparency and annotation policies.
- Introduce pixel/path precision only where corpus results justify it.

### Milestone 8: Navigation And Packaging

- Links and internal destinations
- Outlines
- Split-page output
- Single-file assembly
- Windows, macOS, and Linux release archives containing the executable and
  matching pinned PDFium shared library side by side
- Explicit sibling-library discovery from `current_exe`, plus an override for
  development and testing
- Native dependency diagnostics that identify the expected library and checked
  locations

### Milestone 9: Advanced Compatibility

- CID and CFF edge cases (feasibility to be confirmed against PDFium and
  FontForge)
- Type 1 fonts (feasibility to be confirmed against PDFium and FontForge)
- Type 3 replacement fonts (feasibility to be confirmed against PDFium and
  FontForge)
- Interactive forms
- Optional SVG/vector background research

## Test Strategy

Every behavioral milestone should cover:

- Unit tests for geometry, matrix normalization, style interning, and spacing
- Model tests using purpose-built PDFs
- Browser screenshots at multiple device pixel ratios
- Selection and clipboard tests
- Link and outline behavior
- Font inspection for cmap, widths, ascent, and descent
- Full fallback assertions ensuring unsupported content remains visible
- Cross-platform smoke tests with pinned PDFium and FontForge versions

Visual comparisons should use perceptual tolerance rather than exact pixels
because PDFium and browser antialiasing differ.

## Recommended Decisions

1. Use bitmap backgrounds for v1, with SVG explicitly deferred.
2. Use object-level fallback, not per-character PDF object rewriting.
3. Use out-of-process PDFium and FontForge workers for isolation and
   parallelism.
4. Pin a PDFium build because key character, font, and object APIs are
   experimental.
5. Fall back when Unicode-to-glyph mapping cannot be proven, unless the project
   deliberately adopts a small PDFium fork exposing character code and glyph
   ID.
6. Distribute platform-specific release archives with the pinned PDFium shared
   library beside the executable. Resolve that sibling library explicitly from
   `current_exe`; allow an explicit development/test override, but do not
   download PDFium at runtime or require a system-wide installation by default.

The fifth decision has the largest effect on fidelity. The public-API-only
versus extension choice is made at the end of Milestone 1's font spike. Use
public APIs when the compatibility corpus proves the mapping or background
fallback is acceptable. Consider a narrowly scoped extension only when the
remaining failures materially affect fidelity and cannot be handled by
fallback without unacceptable output loss.
