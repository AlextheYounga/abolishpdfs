# Native Text Integrity

## Request

Enforce the text-output contract so extractable PDF text is delivered as browser-native HTML or reported as an explicit conversion failure.

## Problem

The converter can render raster backgrounds for unsupported content, but the output contract prohibits presenting an image as a successful substitute for extractable text. The current model does not provide one authoritative outcome for missing Unicode, unproven font mappings, or text that cannot be reconstructed safely.

## Definitions

**Native text:** Visible, selectable, searchable HTML text emitted in the document DOM, backed by the model's semantic Unicode.

**Text failure:** A conversion error identifying the affected page and text object when extractable PDF text cannot be emitted as native text. A text failure is distinct from a warning for non-text graphics.

**Graphic fallback:** A raster asset containing PDF graphics or text that is intrinsically non-reconstructable after the text failure has been reported. It never satisfies the native-text requirement.

## Desired outcome

Every extractable text object either appears as native HTML text or causes conversion to fail with a page/object diagnostic. Successful output never silently hides extractable text inside a raster background.

## Scope

- Define reconstruction outcome and diagnostics for missing Unicode, unsafe mapping, unsupported text modes, and failed native emission.
- Propagate object-scoped text failures through the model, converter, CLI, and diagnostic output.
- Ensure raster backgrounds do not conceal a failed text conversion.
- Add tests proving native DOM text, failure reporting, and absence of silent success.

## Constraints

- Preserve native text as the primary output goal.
- Keep graphic backgrounds available for genuine graphics and explicitly non-reconstructable content.
- Use existing `DocumentModel`, `ReconstructionDecision`, `OutputError`, and CLI error-reporting boundaries.
- Do not add an invisible duplicate text layer.

## Exclusions

- FontForge font transformation.
- New layout compensation algorithms.
- A full visibility analyzer; this branch consumes its decisions through a stable model outcome.
