# Plan

## Current behavior

`Cargo.toml` selects `pdfium-render` 0.9.3 with the `pdfium_7881` feature.
`src/pdfium/library.rs` resolves an explicit path, a sibling library beside the
executable, and reports checked paths when no library exists. The CLI exposes
`--pdfium-path` and `ABOLISHPDFS_PDFIUM_PATH`.

`src/cli.rs` exposes `--fontforge-path` and
`ABOLISHPDFS_FONTFORGE_PATH`, defaulting to `fontforge`. `src/fonts/worker.rs`
invokes that executable with `tools/fontforge_worker.py` using the headless
`-lang=py -script` interface.

`tools/generate_corpus.py` creates checked-in license-free fixtures.
`tools/corpus.py` validates the manifest and runs diagnostic conversion when
given a binary and PDFium path. `tools/browser_corpus.py` imports Playwright at
runtime, opens generated HTML, checks native text, copied text, navigation,
assets, and screenshots, and requires its caller to provide generated output.
`run.sh` assumes an untracked `vendor/pdfium-7881` directory, while `.gitignore`
already excludes `vendor`, `bin`, and `out`.

## Intended behavior

`tools/runtime.toml` becomes the tracked runtime manifest. It pins PDFium build
7881 and its artifact checksum, FontForge 20230101, and Python Playwright 1.52.0;
Chromium is the browser revision installed by that Playwright release. It also
names the managed runtime directory and PDFium library filename.

`tools/bootstrap-runtime` creates the ignored runtime directory, downloads and
checksums the PDFium artifact, verifies the required headless FontForge
executable, creates an isolated Python environment, installs pinned Playwright,
and runs `playwright install chromium`. Every failure names the dependency, the
expected version or location, and the corrective command.

`tools/run-integration` requires a successful bootstrap, builds the debug binary,
regenerates fixtures, runs the Rust test suite and manifest validation, runs the
diagnostic corpus with managed PDFium, generates one HTML output per fixture with
managed PDFium and FontForge paths, and invokes the existing browser corpus
script using the managed Python environment. It does not duplicate assertions.

## Approach

Use small Bash entry points for orchestration and a TOML manifest for policy.
Scripts resolve the repository root from their own location, use strict error
handling, and pass absolute runtime paths to child commands. Managed PDFium is
the default. `ABOLISHPDFS_PDFIUM_PATH` and `ABOLISHPDFS_FONTFORGE_PATH` remain
explicit developer overrides.

Bootstrap verifies FontForge by executing the configured binary with `-version`
and running the same headless script invocation used by the worker against a
temporary font job. It verifies Playwright by importing the installed Python
package and launching Chromium. Existing browser assertions remain authoritative.

## Responsibilities and boundaries

- `tools/runtime.toml` owns development version, artifact, checksum, and runtime directory policy.
- `tools/bootstrap-runtime` owns acquisition, executable verification, and prerequisite errors.
- `tools/run-integration` owns ordering and path wiring for existing gates.
- `src/pdfium/library.rs` and `src/cli.rs` remain responsible for application path resolution and CLI/environment configuration.
- `tools/corpus.py` owns diagnostic corpus assertions.
- `tools/browser_corpus.py` owns browser DOM, clipboard, navigation, asset, and screenshot assertions.
- `CONTEXT.md` owns setup and failure interpretation.

## Affected areas

- `tools/runtime.toml`
- `tools/bootstrap-runtime`
- `tools/run-integration`
- `.gitignore` for managed runtime and generated integration artifacts
- `CONTEXT.md` setup documentation
- The temporary-font verification command used by bootstrap

## Decisions

- Use a tracked TOML manifest because the project has no existing runtime configuration format and these values are human-maintained development policy.
- Support Linux as the automated target because the repository currently assumes a Linux shared-library name in `run.sh`; keep application path selection delegated to the cross-platform Rust resolver.
- Manage PDFium locally and checksum it because arbitrary system PDFium can be ABI-incompatible with the selected binding feature.
- Require FontForge discovery plus executable verification rather than packaging FontForge; final redistribution is outside this change.
- Use an isolated Python environment for Playwright so the pinned package is independent of global Python state.
- Keep generated fixture output and screenshots ignored and invoke existing corpus scripts so assertions have one source of truth.

## Risks

- PDFium artifacts, checksums, or shared-library loaders can differ across hosts; bootstrap must fail before integration and print the expected artifact and path.
- A host FontForge version can differ from the supported version or lack Python scripting support; executable and temporary-job checks must identify this as a bootstrap failure.
- Playwright browser revisions can be missing or incompatible; browser launch must fail before corpus assertions run.
- Existing converter behavior may fail after prerequisites are available; the wrapper must preserve the failing command and classify it as a converter/corpus failure.
- Runtime downloads must remain ignored so repeated bootstrap runs do not add binaries or generated artifacts.

## Validation

- Parse the manifest and run bootstrap twice; the second run reuses verified files without changing tracked files.
- Verify the managed PDFium file exists, matches the manifest checksum, and is passed explicitly to diagnostic and HTML commands.
- Verify FontForge version output and a real headless worker invocation.
- Verify the pinned Playwright package imports and launches its installed Chromium executable.
- Run `./tools/run-integration` and retain structured diagnostic/browser output under ignored runtime paths.
- Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check`.
- Review the final diff and confirm no runtime blobs, virtual environment files, generated HTML, or screenshots are tracked.
- Record pinned versions, exercised platform, commands run, bootstrap failures, converter failures, and browser-gate follow-up needs.
