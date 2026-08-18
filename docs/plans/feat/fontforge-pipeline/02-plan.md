# Plan

## Current behavior

`src/model/font.rs` stores embedded bytes, used Unicode, and a mapping-proof flag. `src/fonts/mapping.rs` checks whether reported Unicode values exist in a font cmap. `src/output/document.rs` writes the original bytes under `assets/font-<id>.*` and emits `@font-face`; no worker, subsetting, reencoding, or metric correction exists.

## Intended behavior

Font preparation produces a processed asset and diagnostics before HTML assembly. The writer references the processed asset and its generated family name. A font without usable data or a proven mapping remains explicitly unsupported and is surfaced to the native-text integrity boundary.

## Approach

Add a Rust font-job coordinator that serializes one request per font to a temporary FontForge script/input boundary, invokes the configured FontForge executable, validates the generated artifact and reported metrics, and returns owned bytes plus diagnostics. Use deterministic document-local job inputs and output names. Keep the HTML writer limited to consuming the prepared font result.

## Responsibilities and boundaries

- `src/fonts/mapping.rs` owns mapping proof.
- A new `src/fonts/worker.rs` owns FontForge process invocation and response validation.
- `src/model/font.rs` owns font preparation state and diagnostics.
- `src/output/document.rs` owns `@font-face` emission and asset placement.
- CLI configuration owns the FontForge executable path and environment override.

## Affected areas

- `src/fonts/mod.rs`, `src/fonts/worker.rs`, and font error types
- `src/model/font.rs` and document preparation flow
- `src/output/document.rs` and font-output tests
- `src/cli.rs` for explicit FontForge configuration
- Font fixtures and font inspection tooling
- Project documentation for the required headless FontForge toolchain

## Decisions

- Use a child-process boundary to isolate FontForge and make its failures diagnosable.
- Process one font per job to keep failures scoped and asset generation deterministic.
- Generate only fonts whose mapping is proven; do not use FontForge to conceal an unknown mapping.
- Keep processed font bytes in the owned output model until the writer writes assets.

## Risks

- FontForge versions can produce different binaries or metrics; the supported toolchain version must be pinned in validation.
- Malformed fonts can fail or hang a worker; the coordinator must enforce process errors and bounded execution.
- Metric correction can change visual layout; font geometry fixtures must compare both rendered output and measured advances.

## Validation

- Worker tests cover successful processing, malformed input, missing executable, and invalid output diagnostics.
- Font inspection verifies mappings, used-glyph retention, family naming, and metrics.
- HTML tests verify processed assets are referenced instead of raw embedded bytes.
- Corpus browser checks verify native text remains visible and selectable with processed fonts.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check` pass.
