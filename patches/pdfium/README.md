# PDFium Glyph Identity Fork

The converter uses a maintained `pdfium-render` 0.9.3 fork to access the native text-page
handle and typed bindings for the additive PDFium glyph-identity exports. The dependency is
pinned to the published Git revision in the root `Cargo.toml`.

## Provenance

The fork publishes:

- `PdfPageText::text_page_handle()`
- typed `PdfGlyphIdentity` bindings
- the expected abolishpdfs extension ABI version

The root `Cargo.toml` applies the fork through `[patch.crates-io]`, while the dependency
continues to pin `pdfium-render = "=0.9.3"` and its existing features. The native PDFium
fork and local development checkout are documented separately in the project runtime
documentation.

When upstream publishes equivalent APIs, remove the patch section after confirming the
upstream version and the native extension integration still compile against the released API.
