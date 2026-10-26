# Abolish PDFs

**Turn PDFs back into documents.**

PDF is a fine final format and a lousy foundation. It freezes text into page
coordinates, buries fonts in binary objects, and makes the browser pretend a
document is a picture.

Abolish PDFs converts PDF files into high-fidelity HTML with real, selectable,
searchable text. Graphics stay faithful. Links keep working. Embedded fonts are
rebuilt for the web. The result is a document the browser understands—not a
screenshot wearing an invisible text layer.

```sh
cargo run --release --bin abolishpdfs -- \
  --pdfium-path /path/to/libpdfium.so \
  --output output \
  document.pdf

xdg-open output/index.html
```

> [!IMPORTANT]
> Abolish PDFs is under active development. It is useful today for development,
> testing, and supported documents, but it is not yet distributed as a
> batteries-included application. You currently provide the patched PDFium
> runtime and FontForge yourself.

## Why this exists

The web already has a document format. It is HTML.

Most PDF viewers render pages to pixels and add a hidden text layer for search
and selection. That is good enough for viewing. It is not conversion. The
visible words still are not words as far as the browser is concerned.

Abolish PDFs takes the harder route:

- visible text becomes browser-native HTML text;
- embedded fonts become deterministic web-font assets;
- PDF spacing and transforms become CSS placement;
- graphics remain in rendered page backgrounds where that is the honest choice;
- links, page destinations, and outlines remain navigable; and
- text that cannot be reconstructed safely causes a clear conversion failure.

That last point matters. Quietly swapping a damaged line of text for the wrong
font may produce a file, but it does not produce a faithful document. This
project would rather tell the truth.

## What it produces

Each conversion writes a small, portable artifact set:

```text
output/
├── index.html
├── document.css
└── assets/
    ├── font-*.woff2
    └── page-*.png
```

All pages live in one HTML document, in reading order. Native text and links sit
over selectively rendered graphics backgrounds. Pages without graphics do not
pay for a raster background they do not need.

## Requirements

Building the converter requires:

- Rust with support for the 2024 edition (Rust 1.85 or newer);
- a compatible **PDFium 7881** shared library with the Abolish PDFs glyph
  identity extension; and
- headless **FontForge 20230101** with Python scripting and WOFF2 support.

The development and browser test workflow additionally requires Linux x86-64,
Python 3.11 or newer, `curl`, `tar`, `sha256sum`, `realpath`, and `fc-match`.
Those exact runtime versions are recorded in [`tools/runtime.toml`](tools/runtime.toml).

## Build and convert

Clone the project and build it from source:

```sh
git clone https://github.com/AlextheYounga/abolishpdfs.git
cd abolishpdfs
cargo build --release
```

Then point the converter at the patched PDFium library:

```sh
./target/release/abolishpdfs \
  --pdfium-path /path/to/libpdfium.so \
  --fontforge-path /path/to/fontforge \
  --output output \
  document.pdf
```

`--fontforge-path` may be omitted when `fontforge` is on `PATH`. The runtime
paths can also be set once in the environment:

```sh
export ABOLISHPDFS_PDFIUM_PATH=/path/to/libpdfium.so
export ABOLISHPDFS_FONTFORGE_PATH=/path/to/fontforge

./target/release/abolishpdfs --output output document.pdf
```

PDFium may instead sit beside the `abolishpdfs` executable under the platform's
usual name: `libpdfium.so`, `libpdfium.dylib`, or `pdfium.dll`.

## Command line

```text
Usage: abolishpdfs [OPTIONS] <INPUT>

Arguments:
  <INPUT>  PDF document to convert

Options:
      --pdfium-path <PATH>     Path to the PDFium shared library
      --fontforge-path <PATH>  FontForge executable [default: fontforge]
      --probe                  Inspect PDFium capabilities and print JSON
      --diagnostic             Extract the document model and print JSON
      --output <DIR>           Output directory [default: output]
  -h, --help                   Print help
  -V, --version                Print version
```

The two JSON modes are for understanding the conversion boundary:

```sh
# See what the loaded PDFium build can extract and render.
abolishpdfs --pdfium-path /path/to/libpdfium.so --probe document.pdf

# Inspect the owned document model before HTML serialization.
abolishpdfs --pdfium-path /path/to/libpdfium.so --diagnostic document.pdf
```

## How it works

```text
PDF
 │
 ▼
PDFium extraction and rendering
 │
 ▼
Owned Rust document model
 ├── visibility analysis
 ├── text-run reconstruction
 ├── spacing and transform projection
 ├── links and outlines
 ├── selective graphics backgrounds
 └── FontForge glyph remapping ──► WOFF2 fonts
 │
 ▼
HTML + CSS + assets
```

PDFium interprets the PDF. Rust turns its output into a stable model with no
borrowed PDFium handles. FontForge reconstructs embedded fonts around the
glyphs the document actually uses. Finally, the writer emits one HTML document
with deterministic CSS and assets.

This is deliberately not a Poppler compatibility layer. The design follows the
capabilities PDFium actually has and extends one narrow boundary—source glyph
identity—where correctness requires it. The corresponding Rust binding is
pinned in [`Cargo.toml`](Cargo.toml), and the extension contract is documented
in [`patches/pdfium/README.md`](patches/pdfium/README.md).

## Development

The ordinary Rust checks are:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

The full integration gate builds fixtures, checks the document model, converts
the corpus, opens the results in Chromium, verifies DOM text and navigation,
and captures screenshots:

```sh
export ABOLISHPDFS_PDFIUM_PATH=/path/to/patched/libpdfium.so
export ABOLISHPDFS_FONTFORGE_PATH=/path/to/fontforge
./tools/run-integration
```

On Linux x86-64, `./tools/bootstrap-runtime` verifies the pinned toolchain and
installs the Playwright environment. It can download PDFium from a release
mirror when `PDFIUM_ARTIFACT_MIRROR` is set; otherwise it uses the explicit
`ABOLISHPDFS_PDFIUM_PATH`. Managed files and generated results stay under the
ignored `.runtime/` directory.

For a quick conversion of the included fixture:

```sh
ABOLISHPDFS_PDFIUM_PATH=/path/to/patched/libpdfium.so ./run.sh
```

The deeper implementation notes and current behavior live in
[`CONTEXT.md`](CONTEXT.md). The compatibility corpus is described by
[`tests/fixtures/manifest.json`](tests/fixtures/manifest.json).

## The standard

A conversion succeeds when the browser gets a real document: the pages look
right, the visible text selects and copies sensibly, the fonts preserve the
source, and navigation still works.

Generating some HTML is easy. Generating HTML worth keeping is the work.
