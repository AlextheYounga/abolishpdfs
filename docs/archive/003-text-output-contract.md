# Text Output Contract

The core goal of `abolishpdfs` is to get data **out** of PDFs: extract the
document's text as real, browser-native HTML. Generating a page image and
wrapping it in HTML is contrary to this goal and is never acceptable output.

This document records that rule and the boundaries around it. It governs all
current and future output work.

## The rule

> A page screenshot must never be presented as a successful text conversion.

`abolishpdfs` targets the `pdf2htmlEX` model: layout-preserving HTML with
genuine browser text. Specifically:

- Extractable PDF text must become real, selectable, searchable, copyable HTML text.
- Embedded fonts must become browser web fonts referenced by `@font-face`.
- Layout is preserved with positioned CSS, coordinates, transforms, spacing, and font metrics.
- Links become HTML anchors.
- SVG or raster assets may represent genuine PDF graphics (paths, photographs,
  shading) and content that cannot be reconstructed as native HTML.

## What is prohibited

- Generating a full-page PNG/JPEG of the document and placing it inside HTML
  as if it were the conversion result.
- Using an image as a substitute for extractable PDF text.
- Emitting output that "looks correct" while carrying none of the document's
  text in the DOM, solely because raster fallback masked the absence.
- Silent degradation that hides unsupported text or mappings inside a rendered bitmap.

Raster assets are acceptable only for real graphic content, mirroring
pdf2htmlEX. They are never a fallback for text.

## Acceptance criteria

- Selecting all page content copies meaningful document text.
- Expected text exists in the page DOM.
- Disabling background/graphic assets does not remove that text.
- Graphic assets contain graphics only, not the document's text.
- Screenshots remain close to the source PDF visually.
- Unsupported text or an unmappable font produces an explicit report of the
  affected page and object. If a PDF contains text but the converter emits
  none of it as HTML, conversion must fail rather than succeed silently.

## Partially supported documents

Pragmatic behavior matching `pdf2htmlEX` is preferred: conversion may succeed
with explicit warnings when only some text requires fallback, provided:

- all normally extractable Unicode text remains native HTML, and
- the affected pages and objects are reported loudly rather than hidden.

## Final word

An image inside HTML is only ever an image of the document, never the document's
data. If a PDF's text cannot be faithfully delivered as HTML, fail — do not fake
success with a screenshot.
