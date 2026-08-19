# Runtime Bootstrap

## Request

Create reproducible development and integration bootstrap tooling for PDFium,
headless FontForge, and Playwright/Chromium, then document and run the existing
integration corpus through those dependencies.

## Problem

Integration work is difficult to execute from a clean checkout because required
native dependencies are implicit. The application accepts explicit PDFium and
FontForge paths, but no shared runtime contract provisions or verifies those
dependencies. Corpus commands therefore fail before they can distinguish
environment failures from converter failures.

## Definitions

**Development runtime:** Native binaries and the Python environment used for
local tests and corpus checks. It is not a redistributable application runtime.

**Runtime manifest:** The single tracked configuration file containing supported
development versions, artifact locations, checksums, and runtime directories.

**Bootstrap:** The idempotent process that creates or locates the development
runtime, verifies executable versions and compatibility, and reports actionable
failures. It does not alter source code.

**Integration run:** The ordered execution of fixture generation, Rust tests,
diagnostic corpus checks, HTML generation, and Playwright browser checks using the
verified development runtime.

## Desired outcome

From a clean checkout on the supported Linux development platform, a developer
can run:

```text
./tools/bootstrap-runtime
./tools/run-integration
```

The first command obtains pinned PDFium, verifies headless FontForge, creates the
pinned Playwright Python environment, and installs its pinned Chromium browser.
The second command passes resolved paths to existing Rust and Python tooling
without source edits, and reports bootstrap failures separately from converter or
corpus assertion failures.

## Scope

This change includes the runtime manifest, Linux bootstrap and integration
orchestration scripts, dependency/version checks, ignored local runtime storage,
and developer documentation. It records exact versions and commands used during
validation, including converter failures exposed by the run.

## Constraints

- PDFium must match `pdfium-render = 0.9.3` and its `pdfium_7881` feature.
- PDFium resolution uses the managed runtime first, then an explicit developer
  override, and never silently selects an arbitrary system library.
- FontForge is invoked headlessly through `tools/fontforge_worker.py`.
- Existing `tools/corpus.py` and `tools/browser_corpus.py` remain the source of
  diagnostic and browser assertions.
- Version policy lives in one tracked manifest, not individual scripts.
- Linux is the automated bootstrap target; application path selection remains
  platform-neutral.
- Runtime binaries, virtual environments, browser caches, generated output, and
  screenshots remain outside Git.

## Exclusions

This change does not alter font-processing semantics, font identity, text layout,
glyph mapping, PDFium internals, production HTML behavior, final installer
layout, or converter correctness beyond exposing existing failures.
