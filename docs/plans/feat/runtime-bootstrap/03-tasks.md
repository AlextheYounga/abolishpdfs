# Tasks

## Implementation

- [x] Add `tools/runtime.toml` with PDFium 7881 artifact/checksum, FontForge 20230101, Playwright 1.52.0, Chromium installation policy, and ignored runtime locations.
- [x] Add `tools/bootstrap-runtime` to acquire/checksum PDFium, resolve and verify headless FontForge, create the isolated Python environment, install pinned Playwright, and install Chromium with actionable errors.
- [x] Add `tools/run-integration` to require bootstrap verification, build the debug binary, regenerate fixtures, run Rust tests and manifest validation, execute diagnostic conversion, generate every fixture's HTML output with explicit runtime paths, and invoke `tools/browser_corpus.py` through the pinned environment.
- [x] Update `.gitignore` and `CONTEXT.md` so runtime binaries, Python environments, generated output, screenshots, commands, and failure categories are explicit and untracked.

## Validation

- [x] Run bootstrap on Linux twice and verify the second run reuses valid managed files and leaves the worktree free of generated artifacts.
- [x] Verify the managed PDFium checksum and confirm diagnostic and HTML commands receive its explicit path.
- [ ] Verify the configured FontForge version and complete a real headless temporary-font worker job.
- [ ] Verify the pinned Playwright package imports and launches installed Chromium.
- [ ] Run the complete integration wrapper and record whether failures are bootstrap failures or converter/corpus assertion failures.
- [ ] Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `git diff --check`.

## Completion

- [x] Review the final diff for one version source of truth, deterministic path wiring, clear prerequisite errors, and unchanged conversion semantics.
- [x] Confirm no PDFium/FontForge binaries, browser downloads, Python environment files, generated HTML, or screenshots are tracked.
- [x] Record pinned versions, commands, exercised platform, observed converter failures, and browser-gate follow-up requirements in completion notes.

## Completion notes

Implemented the manifest reader, Linux PDFium bootstrap, FontForge worker check,
isolated Playwright setup, integration wrapper, ignore rule, and developer
documentation. PDFium 7881 was downloaded and checksum-verified. A second
bootstrap reused the managed artifact without downloading it again, then stopped
with the actionable prerequisite failure that FontForge 20230101 is not installed
on this host. Consequently Playwright installation and the complete integration
run remain pending.

`cargo fmt --check`, `cargo test`, `cargo test --lib`, `cargo clippy
--all-targets --all-features -- -D warnings`, Python and shell syntax checks,
corpus manifest validation, and `git diff --check` were run. Library tests and
clippy pass. Full `cargo test` and corpus validation fail for the existing
`mixed-styles` and `layout-breaks` manifest entries because their required
expectations are incomplete; those converter/corpus fixtures were not changed by
this branch.
