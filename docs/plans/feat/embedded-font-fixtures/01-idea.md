# Idea

## Request

Create a compatibility-corpus fixture set that exercises the embedded-font
pipeline rather than only the existing built-in `/Helvetica` fixtures. The set
must include one multi-page vertical-slice PDF and several focused PDFs with
real embedded TrueType fonts, deterministic generation, provenance, and
manifest expectations.

## Problem

The current generated fixtures use a built-in Type 1 font resource. They test
text extraction and layout classifications, but they do not prove that a PDF
contains embedded font bytes or exercise the path from PDFium font extraction
through mapping validation, FontForge processing, generated WOFF2 output,
`@font-face`, and browser-native text rendering.

## Definitions

**Embedded font:** A PDF font resource with font-program bytes stored in the PDF
font descriptor. A font name or a `/BaseFont` value alone does not qualify.

**Embedded-font fixture:** A checked-in or reproducibly generated PDF whose
manifest identifies its embedded-font behavior and whose integrity test proves
that the PDF contains the declared font program.

**Vertical-slice fixture:** The required multi-page PDF that combines embedded
font text with spacing, transforms, an image, a vector shape, and an external
URI link so one artifact exercises the complete corpus path.

**Fixture integrity validation:** Checks that run without the converter and
prove file existence, deterministic regeneration, embedded TrueType bytes,
font metadata, and manifest coverage. They do not claim that runtime
conversion succeeds.

**Runtime validation:** Diagnostic, output, and browser-corpus checks performed
with the configured PDFium, FontForge, and browser dependencies. A documented
runtime failure on a new fixture is an accurate exposed defect, not a reason to
weaken the fixture.

## Desired outcome

The repository contains a deterministic embedded-font corpus with one
multi-page vertical-slice PDF and focused PDFs for basic embedded text,
subset-style naming, multiple embedded fonts, and repeated characters at
different sizes or transforms. Each fixture has explicit page, text, clipboard,
link, generated-font, background, processing, and screenshot expectations.

The corpus test proves that at least one fixture contains actual embedded TTF
bytes and that all fixture paths and provenance records are valid. Existing
fixtures remain covered without weakened assertions. Runtime pass/fail status
for the new fixtures is recorded separately when the external toolchain is
available.

## Scope

- Add the embedded-font PDF fixtures and their deterministic generator.
- Commit redistribution-safe Liberation TrueType source assets and their license and
  provenance record.
- Add integrity checks that inspect PDF resources instead of trusting filenames.
- Extend the existing manifest and its Rust/Python validators with the smallest
  font-specific expectation fields required by these fixtures.
- Record expected native text, clipboard text, URI links, generated font assets,
  graphics backgrounds, font-processing status, and screenshot status.
- Document the fixture commands, provenance, isolated behavior, and runtime
  results.

## Constraints

- Use Liberation Sans and Liberation Serif version 2.1.5, distributed under
  the SIL Open Font License 1.1; commit the exact TTF inputs and license text
  under the fixture source directory.
- Generate PDFs with stable object ordering, byte content, and output names.
- Use actual embedded TrueType program streams, not built-in PDF fonts or
  filename-based assumptions.
- Keep the primary vertical-slice PDF at least two pages and include several
  text runs, two sizes, ordinary spaces, explicit spacing, rotated text, one
  raster image, one vector shape, and one external URI link.
- Keep CFF, CID, Type 1, and Type 3 coverage outside this change.
- Do not change converter behavior, FontForge failure semantics, font catalog
  identity, glyph-mapping policy, PDFium bindings, or layout algorithms.
- Do not import arbitrary internet PDFs or weaken existing corpus assertions.

## Exclusions

- Fixes to embedded-font extraction, mapping, FontForge processing, WOFF2
  generation, HTML output, or browser rendering.
- New production font formats or a general PDF authoring framework.
- Baseline screenshot promotion for fixtures whose runtime output is not yet
  established.
- Importing upstream or third-party PDF corpora before license review.
