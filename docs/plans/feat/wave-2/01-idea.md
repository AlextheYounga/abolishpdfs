# Idea

## Request

Implement Wave 2 from `docs/forward.md`: establish the end-to-end browser gate
and add adversarial font-mapping fixtures that expose where the current
Unicode-to-cmap inference is insufficient.

## Problem

The repository has a reproducible PDFium, FontForge, and Chromium runtime plus
embedded-font fixtures, but the complete conversion path has not been made a
passing acceptance gate. The browser checker queries a DOM class the writer
does not emit, does not prove processed web fonts loaded, does not separately
assert selection, and only captures unapproved screenshots.

The corpus also exercises only fonts whose PDF character codes, semantic
Unicode, embedded cmap, and rendered glyph agree. Consequently it cannot prove
the documented limitation that PDFium's public API reports Unicode without the
source character code or glyph identity needed to verify that the same glyph is
reconstructed in the browser.

## Definitions

**End-to-end browser gate:** The repeatable workflow that generates PDFs, reads
them through PDFium, processes required embedded fonts through FontForge,
writes WOFF2-backed HTML, and validates the result in pinned Chromium. A run is
successful only when every diagnostic, DOM, font-loading, selection, clipboard,
navigation, asset, and screenshot assertion passes.

**Approved baseline:** A tracked screenshot captured by the pinned Wave 2
runtime from a successful conversion, reviewed as the expected first-page
appearance, and referenced by a `compare` manifest entry. It is not an
arbitrary image or capture-only artifact.

**Adversarial mapping fixture:** A deterministic PDF whose source character
code, ToUnicode value, embedded-font cmap lookup, semantic DOM text, and visible
glyph do not all identify the same character. Its expectations explicitly
record both the browser-native text and the intended visible source glyph.

**Mapping exposure:** A passing test that demonstrates and records a known
architectural limitation by asserting the semantic/visual disagreement. It
does not represent the disagreement as correct conversion behavior.

## Desired outcome

`./tools/run-integration` executes the complete conversion path and fails for
missing or unloaded processed fonts, incorrect DOM or selected/copied text,
broken navigation or assets, or visual output outside approved tolerances.
Every generated fixture has a tracked screenshot baseline.

The generated corpus includes an embedded TrueType fixture where PDF code
`0x41` renders the font's `A` glyph while ToUnicode reports `B`. Offline tests
prove that construction, and runtime expectations prove that the current
pipeline emits native `B` text although the source glyph is `A`, establishing
the exact evidence required before Wave 3 changes glyph identity.

## Scope

- Correct and strengthen browser DOM, loaded-font, selection, clipboard,
  navigation, asset, and screenshot assertions.
- Track and validate approved first-page screenshot baselines for every
  generated fixture under the pinned runtime.
- Generate one deterministic ToUnicode mismatch PDF using the committed
  Liberation Sans input and record semantic and visual expectations.
- Extend fixture integrity and manifest validation for adversarial mapping
  metadata.
- Run and document the complete PDFium, FontForge, WOFF2, HTML, and Chromium
  workflow.

## Constraints

- Use the versions and paths defined by `tools/runtime.toml` and the existing
  bootstrap workflow.
- Keep generated fixture inputs offline, deterministic, and redistribution-safe.
- Browser checks must inspect observable behavior rather than infer success
  from file existence alone.
- Screenshot tolerances remain perceptual and bounded; baselines must come from
  the pinned runtime.
- Preserve the current explicit text-integrity failure policy.
- Complete the auto-approved Acta planning record before implementation and do
  not commit changes.

## Exclusions

- Adding PDFium APIs for source character codes, glyph IDs, or font-resource
  identity.
- Correcting the adversarial mapping mismatch through FontForge reencoding or
  production model changes; those belong to Wave 3.
- CFF, CID, Type 1, Type 3, complex text, and vertical-writing support.
- Expanding the runtime bootstrap beyond its current Linux automated target.
- Importing unlicensed external PDF corpora.
