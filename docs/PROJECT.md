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

Rendered backgrounds and invisible text overlays remain available as correctness fallbacks for cases that cannot yet be reconstructed safely.

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

The writer produces `index.html`, `document.css`, and split page files under
`pages/`. The index displays the split pages, while each page uses its crop box
as the CSS viewport origin. Native text is emitted per glyph so geometry, color,
stroke, and transforms do not incorrectly inherit from the first glyph in a
mixed text object. Text that the model marks for background fallback is
intentionally not duplicated in the native layer until selective raster
backgrounds are implemented.

## Phase 4: Navigation and fallback foundations

The first Phase 4 increment renders extracted page links as positioned HTML anchors. HTTP, HTTPS, `mailto`, and
`tel` URI actions are emitted as navigable links; unsupported URI schemes remain visible in the diagnostic model
without becoming unsafe browser navigation. Local destinations resolve to split page files, document bookmarks are
emitted as an outline navigation tree, and pages containing non-reconstructable text receive a selectively prepared
raster background: native text is suppressed during rendering, while fallback text and graphics remain visible.
Background PNGs are written to `assets/` and layered beneath native HTML text and links.

## Phase 5: Graphics fidelity

Page paths, images, shadings, forms, and unsupported PDFium objects are now retained in the owned model with paint
order, bounds, activity, and recursive form children. Pages containing graphics are rendered into a PNG background
so vector and image content remains visually faithful while reconstructed text, links, and document navigation stay
browser-native overlays. Pages without graphics continue to avoid unnecessary raster assets.

## Phase 6: Embedded font output

Embedded font bytes collected by PDFium are now written as deterministic font assets and referenced by generated
`@font-face` rules. Native glyph spans select their model font when embedded data is available, while fonts without
usable data retain the browser's sans-serif fallback. Background text remains raster-backed when its mapping or
geometry is not proven safe for native reconstruction.

Milestone 0 fixture assets live under `tests/fixtures/`. The generated fixtures cover
ordinary text, transforms, spacing, page boxes, links, and clipping/transparency.
Their expected feature classifications are stored in `tests/fixtures/manifest.json` and
validated by `cargo test --test corpus_manifest`. Diagnostic classification is
available through `tools/corpus.py`; optional Playwright screenshot and copied
text checks are provided by `tools/browser_corpus.py`. Licensed upstream PDFs
remain pending until provenance and redistribution terms are recorded.

## Links
- [pdf2htmlex](https://github.com/pdf2htmlex/pdf2htmlex)
- [fontforge](https://github.com/fontforge/fontforge)
- [pdfium](https://github.com/ajrcarey/pdfium-render)
- [pdf2html source code](docs/agents/links/pdf2htmlEx-src) (symlink)
