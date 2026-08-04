# Text Output Contract

`abolishpdfs` exists to get data out of PDFs. Its successful output must
therefore contain the document's extractable text as real browser-native HTML.

The project follows the pragmatic `pdf2htmlEX` model:

- Extractable PDF text is emitted as selectable, searchable, copyable HTML.
- Embedded fonts may be emitted as web fonts, with CSS preserving PDF layout.
- Links are emitted as HTML anchors.
- SVG or raster assets may represent genuine PDF graphics, such as paths,
  photographs, shading, or content that cannot be reconstructed as native HTML.
- A raster page image must never substitute for extractable text.

Conversion must not silently succeed when a page containing text emits only a
page image. Unsupported or unmappable text is retained through explicit
fallback and reported with its page and object. If no faithful text output is
possible, conversion fails rather than presenting a screenshot as a successful
HTML conversion.

Partially supported documents may succeed, matching `pdf2htmlEX` behavior, when
all normally extractable Unicode text remains native HTML and every fallback is
reported explicitly.
