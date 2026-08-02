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

## Links
- [pdf2htmlex](https://github.com/pdf2htmlex/pdf2htmlex)
- [fontforge](https://github.com/fontforge/fontforge)
- [pdfium](https://github.com/ajrcarey/pdfium-render)