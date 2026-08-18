# 004: Single-DOM Document Output

## Objective

Replace iframe-based split-page output with one browser document containing all
pages inline, following the default structure used by pdf2htmlEX.

## Implemented Design

- `index.html` contains the outline navigation and every page in document order.
- Each page is a `section.page` with a stable `id="page-N"` and a nested
  `.page-content` coordinate system.
- Native glyphs, links, and optional raster backgrounds remain page-local
  children of `.page-content`.
- Local PDF destinations and outline entries use `#page-N` fragments.
- `document.css` and files under `assets/` remain external artifacts.
- Fresh output no longer creates `pages/N.html` or emits iframes.
- The browser corpus opens `index.html`, allowing selection and search across
  page boundaries.

## Verification

- [x] Remove the per-page HTML collection from `HtmlDocument`.
- [x] Render page fragments directly into `index.html`.
- [x] Use document-relative asset paths.
- [x] Convert local links and outline entries to page fragments.
- [x] Add print page-break rules and page-content layout styles.
- [x] Update writer tests, CLI help, project documentation, and browser tooling.
- [x] Run the full formatter, test, and clippy checks.

## Deferred

Optional split-page output, lazy loading, viewer JavaScript, and embedded
single-file assets are not part of this migration.
