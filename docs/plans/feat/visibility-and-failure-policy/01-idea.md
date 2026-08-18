# Visibility And Failure Policy

## Request

Determine when reconstructed native text is visibly safe by analyzing PDF paint order, clipping, opacity, and coverage, then apply explicit fallback or failure decisions.

## Problem

The model records graphics and conservative fallback facts, but it does not yet analyze whether later PDF objects cover, clip, mask, or blend with text. Native text can therefore be emitted where the PDF's visible result differs, or retained in a background without an explainable decision.

## Definitions

**Visibility decision:** An object-scoped result describing whether native text remains visible and faithful under later paint operations.

**Conservative fallback:** Retaining the relevant PDF rendering when available because available geometry does not prove native reconstruction is safe. It is distinct from silently accepting missing text.

**Opaque coverage:** A later paint operation whose bounds, clipping, and opacity establish that an earlier text object is fully hidden.

## Desired outcome

Each reconstructable text object receives a deterministic visibility decision based on paint order and supported clipping/transparency facts. Ambiguous visibility produces an explicit fallback or native-text failure diagnostic, never silent visual corruption.

## Scope

- Analyze page object order, recursive form contents, bounds, clipping, opacity, and supported graphics kinds.
- Produce object-scoped visibility reasons in the model.
- Coordinate visibility decisions with selective raster backgrounds and native-text integrity.
- Add fixtures and tests for covered text, clipping, transparency, forms, and ambiguous geometry.

## Constraints

- Use PDFium's exposed object data and existing `GraphicsObject` model.
- Treat bounding-box overlap as insufficient proof of complete coverage.
- Prefer preserving visible content and reporting uncertainty over aggressive native reconstruction.
- Keep genuine graphics backgrounds separate from text-output success.

## Exclusions

- Full path-accurate or pixel-perfect coverage analysis.
- FontForge processing.
- General HTML layout compensation.
