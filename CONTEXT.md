# abolishpdfs

A modern, cross-platform PDF-to-HTML converter built in Rust with PDFium and FontForge.

## Mission

`abolishpdfs` will convert PDF documents into high-fidelity HTML while preserving the defining qualities that make pdf2htmlEX valuable:

* Visible browser-native text
* Accurate page layout
* Searchable and selectable content
* Embedded document fonts
* Images and vector backgrounds
* Character spacing and transformations
* Links and internal destinations
* Document outlines
* Single-file and split-page output
* Reliable operation on Windows, macOS, and Linux

The project is not merely a PDF viewer with a transparent text layer. Its primary goal is to reconstruct PDF text as real HTML text as accurately as reasonably possible.

Rendered backgrounds remain available for graphics, but extractable text that cannot be reconstructed safely is reported as a conversion failure rather than hidden in an image.

You can find the `pdf2htmlEx` core code here: docs/agents/links/pdf2htmlEx-src (symlink) 

## Success standard

The project should aim to match or exceed `pdf2htmlEX` across a representative compatibility corpus.

Exact byte-for-byte HTML output is unnecessary. The relevant equivalence is behavioral:

1. Pages look substantially the same.
2. Visible text remains browser-native whenever technically possible.
3. Text selection follows visible characters.
4. Copied text is reasonable.
5. Fonts preserve the original document appearance.
6. Images, paths, clipping, and transparency remain visually correct.
7. Links, outlines, and destinations work.
8. Unsupported cases degrade without visibly corrupting the page.
9. The converter installs and runs natively on all three desktop operating systems.

## Non-goals
- Multi-language binding supports, this will be Rust-first.

## Technology choices

### Rust

Rust owns:

* Document orchestration
* PDFium adaptation
* The internal document model
* Text reconstruction
* Layout compensation
* Visibility decisions
* HTML and CSS generation
* Asset management
* Worker management
* Error reporting
* Cross-platform CLI behavior

### PDFium

PDFium replaces Poppler as the PDF parser, interpreter, page-object engine, renderer, and extraction engine.

PDFium is not expected to expose every internal event Poppler exposed through `OutputDev`. The architecture must therefore be based on PDFium’s actual public capabilities rather than pretending it is a one-for-one Poppler replacement.

### FontForge

FontForge is the primary font-processing backend for the initial implementation.

The headless `fontforge` executable must be available on `PATH`, or configured with
`--fontforge-path` / `ABOLISHPDFS_FONTFORGE_PATH`. The pipeline invokes one isolated
job per embedded TrueType or OpenType font and emits deterministic WOFF2 assets;
fonts that cannot be processed remain on the documented raster fallback path.

It will handle:

* Embedded TrueType fonts
* OpenType fonts
* CFF fonts
* CID fonts
* Type 1 fonts
* Font subsetting
* Glyph reencoding
* Unicode mapping
* Width and metric rewriting
* Missing-space insertion
* Font naming
* Hinting
* Browser-compatible output generation
* Type 3 replacement fonts where practical

FontForge supports headless command-line and Python-script execution, including opening and generating fonts without launching its user interface.


## High-level architecture

```text
PDF
 │
 ▼
PDFium worker
 │
 ▼
Stable Rust document model
 │
 ├── document preprocessor
 ├── font usage collector
 ├── visibility analyzer
 ├── text layout engine
 ├── background generator
 ├── links and outline processor
 └── HTML/CSS writer
 │
 ├──────────────► FontForge worker
 │                   │
 │                   ▼
 │              processed web fonts
 │
 ▼
HTML document and assets
```

## PDFium Capability Probe

Milestone 1 is exercised with the `--probe` CLI mode before building production
extraction or output code:

```text
abolishpdfs --pdfium-path /path/to/libpdfium.so --probe document.pdf
```

The JSON report records the pinned PDFium binding target, page and recursive Form XObject paint order, character
geometry and text-object association, decoded font-data availability, and a
deactivate/render/reactivate bitmap check for text objects. A target-platform
Milestone 1 run passes only when the report shows that suppressing active text
changes the bitmap, reactivation restores the original bitmap, and no operation
errors are reported. It also states the current public-API limitation: the
configured PDFium bindings do not expose a per-character PDF-code or glyph-ID
mapping. Font-dependent content must therefore remain a background fallback
until the compatibility corpus proves a safe mapping or justifies a narrowly
scoped PDFium extension.

Milestone 2 adds the owned diagnostic converter. It can be exercised with:

```text
abolishpdfs --pdfium-path /path/to/libpdfium.so --diagnostic document.pdf
```

This output is a serialized `DocumentModel`. It contains no PDFium handles and
records crop boxes, object-associated characters, recursive Form XObject
contents, and conservative background fallback decisions instead of silently
treating uncertain text as reconstructable. It is an inspection artifact, not
the final HTML output contract.

Milestone 3 writes the first HTML artifact set from that model:

```text
abolishpdfs --pdfium-path /path/to/libpdfium.so --output output document.pdf
```

The writer produces one `index.html` document, `document.css`, and assets under
`assets/`. Every page is emitted inline as a `.page` section in document order,
with its crop box as the CSS viewport origin. Native text is emitted per glyph
so geometry, color, stroke, and transforms do not incorrectly inherit from the
 first glyph in a mixed text object. Text that the model cannot reconstruct
 natively is reported as a conversion failure instead of being hidden in a
 raster background.

Text objects whose matrix is a pure translation use each glyph's measured
bounds as the CSS box (`left`/`top`), which is exact for horizontal text. Text
objects carrying rotation, skew, or anisotropic scale are projected the way
pdf2htmlEX does in `HTMLRenderer/state.cc`: the vertical scale is absorbed into
`font-size` (computed from the unscaled `Tf` size times `hypot(c, d)`), the
remaining matrix is y-flipped into a `transform:matrix(...)` with zero
translation, and the glyph baseline anchors the box through `left`/`bottom`
with `transform-origin: 0 100%`. This keeps rotated and mirrored runs visible
and correctly oriented without double-applying the matrix translation.

## Phase 4: Navigation and fallback foundations

The first Phase 4 increment renders extracted page links as positioned HTML anchors. HTTP, HTTPS, `mailto`, and
`tel` URI actions are emitted as navigable links; unsupported URI schemes remain visible in the diagnostic model
without becoming unsafe browser navigation. Local destinations resolve to page fragments in the single document, document bookmarks are
emitted as an outline navigation tree, and pages containing non-reconstructable text receive a selectively prepared
raster background: graphics remain visible beneath native HTML text and links. A page with extractable text that is not represented natively fails normal output conversion.

## Phase 5: Graphics fidelity

Page paths, images, shadings, forms, and unsupported PDFium objects are now retained in the owned model with paint
order, bounds, activity, and recursive form children. Pages containing graphics are rendered into a PNG background
so vector and image content remains visually faithful while reconstructed text, links, and document navigation stay
browser-native overlays. Pages without graphics continue to avoid unnecessary raster assets.

## Phase 6: Embedded font output

Embedded font bytes collected by PDFium are now written as deterministic font assets and referenced by generated
`@font-face` rules. Native glyph spans select their model font when embedded data is available, while fonts without
usable data retain the browser's sans-serif fallback. Text whose mapping or geometry is not proven safe for native
reconstruction is reported as a structured conversion failure.

## Visibility and failure policy

The extraction pass analyzes text against later paint objects using owned model data. Complete containment by an active,
opaque, unclipped path is classified as `CoveredByOpaquePaint`; partial overlap, unknown bounds, transparency, or clipping
is classified as `AmbiguousVisibility`. Overlap alone never proves coverage. Visibility-driven background fallback is
represented by `FallbackReason` and serialized with each text object; ambiguous extractable text also receives an
object-scoped diagnostic.

Milestone 0 fixture assets live under `tests/fixtures/`. The generated fixtures cover
ordinary text, transforms, spacing, page boxes, links, and clipping/transparency.
Their expected feature classifications and browser observations are stored in
`tests/fixtures/manifest.json` and validated by `cargo test --test corpus_manifest`.

The repeatable corpus gate is:

```text
python3 tools/generate_corpus.py
cargo test --test corpus_manifest
python3 tools/corpus.py validate
python3 tools/corpus.py run --binary target/debug/abolishpdfs --pdfium /path/to/libpdfium.so
python3 tools/browser_corpus.py /path/to/generated-output
```

The diagnostic command emits structured JSON and fails when page count, crop-box
geometry, object/link counts, or the recorded fallback decision differs. The browser
command checks native DOM text, copied text, page order, page fragments, URI links,
required assets, and one screenshot per fixture. Screenshots are written to the
requested directory and are capture-only until a manifest entry records a baseline.
For comparison entries, `max_diff_ratio` and `max_diff_pixels` are perceptual
tolerances rather than byte equality; Pillow is required for those comparisons.
Generated screenshots and diagnostic output should be kept outside tracked source.
Licensed upstream PDFs remain pending until provenance and redistribution terms are
recorded.

## Links
- [pdf2htmlex](https://github.com/pdf2htmlex/pdf2htmlex)
- [fontforge](https://github.com/fontforge/fontforge)
- [pdfium](https://github.com/ajrcarey/pdfium-render)
- [pdf2html source code](docs/agents/links/pdf2htmlEx-src) (symlink)
