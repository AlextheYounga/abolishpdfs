I’d split them into **small, mergeable capability branches**, not one branch per roadmap phase. Some roadmap phases are too large to be safe feature branches.

I’d also make each branch leave `develop` in a valid state. No branch should require three subsequent branches before its code makes sense.

Here’s the breakdown I’d use:

1. **`feat/font-integrity-contract`**
   Fix the current correctness problems before further development: FontForge failure invalidates dependent native text, remove silent sans-serif substitution where a reconstructed font is required, introduce explicit font processing state, and make output fail correctly when font preparation fails. This should be the first branch.

2. **`feat/font-resource-identity`**
   Replace font-name-only deduplication with real document-local font identity. Add source/resource identity, embedded-data hash where useful, and make glyphs/text objects reference that stable identity. Keep this separate from the previous branch because it changes the model fairly deeply.

3. **`feat/cross-platform-fontforge-tests`**
   Remove Unix-only test assumptions, make the FontForge test harness work on Windows/macOS/Linux, and establish a portable fake-worker mechanism. Small branch, easy review.

4. **`feat/runtime-bootstrap`**
   Add reproducible provisioning for the pinned PDFium build, FontForge, and Playwright/Chromium. The goal is that a build agent or CI environment can actually run the integration suite rather than continually reporting “runtime unavailable.”

5. **`feat/embedded-font-fixtures`**
   Add genuine embedded TrueType/OpenType fixtures instead of relying primarily on built-in Helvetica. Include a multi-page vertical-slice document with graphics, links, transforms, sizes, spacing, and embedded fonts. This is test infrastructure, so I’d merge it before changing converter behavior based on its failures.

6. **`feat/end-to-end-browser-gate`**
   Actually execute PDF → PDFium → FontForge → WOFF2 → HTML → Chromium. Assert loaded web fonts, DOM text, selection, clipboard text, links, assets, and screenshots. Establish approved baselines rather than capture-only screenshots. This is the branch that turns the current prototype into something experimentally grounded.

7. **`feat/adversarial-font-mapping-fixtures`**
   Construct PDFs where PDF character codes, font cmap values, glyph IDs, and semantic Unicode intentionally do not line up trivially. The goal is to prove exactly where the current Unicode→cmap inference breaks. This branch should ideally contain tests that fail under the current implementation.

8. **`feat/pdfium-glyph-identity`**
   Add the smallest necessary PDFium extension and Rust binding changes to expose source character codes, glyph IDs, and possibly stable font-resource identity. Extend `Glyph` accordingly. This should be narrowly scoped and heavily documented because it creates a maintained PDFium patch boundary.

9. **`feat/fontforge-glyph-remapping`**
   Change the FontForge pipeline from “subset by Unicode” to real PDF glyph-driven reconstruction using source character code/glyph ID + semantic Unicode. Implement cmap rewriting, glyph reencoding, deterministic naming, and metric output around the now-reliable mapping.

10. **`feat/browser-font-metrics`**
    Make spacing compensation operate against the **processed browser font**, including actual generated metrics. Fix letter-spacing/local-offset calculations and add browser-measured regression cases. This should land before broad layout expansion because everything later depends on trustworthy font metrics.

11. **`feat/cff-fonts`**
    Add CFF/OpenType fixtures and support, including FontForge conversion behavior and browser validation. Keep each major font family separate rather than creating one giant “advanced fonts” branch.

12. **`feat/cid-fonts`**
    Add CIDFontType0/Type2 support, CID mapping fixtures, FontForge flattening/reencoding, and browser tests. This is likely substantial enough to deserve its own branch or even two branches if Type0 and Type2 diverge materially.

13. **`feat/type1-fonts`**
    Handle embedded Type 1 fonts, appropriate conversion to browser formats, metrics, mappings, and regression coverage.

14. **`feat/text-spacing-parity`**
    Finish `Tc`, `Tw`, `TJ`-style displacement behavior, explicit positioning, generated spaces, word spacing, local discontinuities, and run splitting. Keep this mostly about linear horizontal layout.

15. **`feat/text-transform-parity`**
    Rotation, skew, anisotropic scaling, mirroring, superscripts/subscripts, arbitrary affine transforms, and transform-specific screenshot tolerances.

16. **`feat/complex-text-layout`**
    Ligatures, combining marks, multi-codepoint mappings, RTL, bidi behavior, and generated text semantics. I would probably keep vertical writing out of this branch because it is distinct enough.

17. **`feat/vertical-writing`**
    Add vertical writing as a first-class layout mode with dedicated fixtures and selection/copy tests.

18. **`feat/visibility-differential-rendering`**
    Extend current bounding-box visibility analysis with render-with/render-without-object comparisons for ambiguous cases. This branch should not simultaneously try to solve every PDF graphics feature.

19. **`feat/clipping-and-transparency`**
    Improve clip-path classification, transparency groups, masks, blend modes, partially covered text, and OCR-layer handling. By this point you can use differential rendering as a fallback oracle rather than trying to analytically model everything.

20. **`feat/type3-fonts`**
    Type 3 should absolutely be isolated. Implement glyph-program extraction/rendering, SVG/path conversion where possible, FontForge replacement-font generation, Unicode/advance mapping, and explicit failure behavior for cases that cannot become normal font glyphs.

21. **`feat/background-quality`**
    Improve raster DPI, page scaling, print quality, output-size tradeoffs, and image extraction. This remains bitmap-oriented and should not be mixed with vector reconstruction.

22. **`feat/vector-backgrounds`**
    Introduce SVG/path/image reconstruction selectively for page content where it is safe. Keep difficult transparency/masks on raster regions. This is enhancement work, not required for the core text architecture.

23. **`feat/destination-semantics`**
    Complete named destinations, XYZ/Fit/FitH/FitV/FitR behavior, remote destinations, and more precise outline targeting.

24. **`feat/annotations`**
    Common annotations, appearance preservation, annotation links and metadata.

25. **`feat/forms`**
    Form appearance preservation first; interactive HTML controls second only where worthwhile. Forms are complex enough that I would avoid mixing them with annotations.

26. **`feat/pdfium-bundling`**
    Build and package the pinned/patched PDFium for Linux, macOS Intel/ARM, and Windows. Establish deterministic target artifacts.

27. **`feat/fontforge-bundling`**
    Create the controlled headless FontForge runtime for all three OS families. Decide here whether the Python scripting worker remains practical for shipping or whether the worker should become a small native bridge.

28. **`feat/cross-platform-release`**
    Assemble the actual abolishpdfs release bundles, runtime discovery, license manifests, clean-machine smoke tests, and target CI. This branch should be mostly integration because PDFium and FontForge packaging have already been solved separately.

29. **`feat/conversion-workers`**
    Move PDFium execution behind persistent worker processes while keeping the existing owned model boundary. Add request/event protocol, process isolation, cancellation, and crash handling.

30. **`feat/fontforge-worker-pool`**
    Pool FontForge processes/jobs, add caching, timeouts, retry/crash semantics, and bounded concurrency.

31. **`feat/page-parallelism`**
    Only after the worker architecture exists: split large documents into page jobs after shared font/document preprocessing and deterministically assemble results.

32. **`feat/compatibility-drive`**
    I actually would **not** make this one giant branch. At that stage I’d switch to issue-driven branches such as `fix/cid-vertical-regression-142`, `fix/soft-mask-covered-text`, etc. Every real-world failure gets minimized into a fixture and fixed independently.

The dependency spine I’d care about most is:

```text
font-integrity-contract
        ↓
font-resource-identity
        ↓
runtime-bootstrap
        ↓
embedded-font-fixtures
        ↓
end-to-end-browser-gate
        ↓
adversarial-font-mapping-fixtures
        ↓
pdfium-glyph-identity
        ↓
fontforge-glyph-remapping
        ↓
browser-font-metrics
        ↓
font/layout compatibility branches
        ↓
visibility/graphics compatibility
        ↓
cross-platform packaging
        ↓
workers/performance
        ↓
compatibility grind
```

I would **not** run all of these sequentially. Once `end-to-end-browser-gate` and `pdfium-glyph-identity` land, several lanes can proceed independently. CFF/CID/Type1 work can run alongside transform/layout work; navigation can run alongside visibility; packaging can begin once the runtime boundaries stabilize.

The first **eight branches** are the important ones right now. They take abolishpdfs from “well-designed unvalidated prototype” to “proven converter architecture with the hardest mapping problem explicitly solved.” Everything after that is mostly expansion of compatibility rather than validation of the central idea.

---

## Parallel Tasks
I’d organize the work into **waves**, where branches inside a wave can run in parallel once their prerequisites land.

The main rule is: **don’t parallelize branches that are trying to define the same core model/API at the same time**. That creates merge pain and conflicting assumptions.

### Wave 1 — can start now in parallel

These are mostly independent:

* `feat/font-integrity-contract`
* `feat/cross-platform-fontforge-tests`
* `feat/runtime-bootstrap`
* `feat/embedded-font-fixtures`

I would keep `feat/font-resource-identity` slightly behind `font-integrity-contract`, because both touch the font model and could conflict heavily.

So:

```text
                 ┌─ cross-platform-fontforge-tests
                 │
start ───────────├─ runtime-bootstrap
                 │
                 ├─ embedded-font-fixtures
                 │
                 └─ font-integrity-contract
                           ↓
                    font-resource-identity
```

That gives you 4 useful agents immediately.

### Wave 2 — integration + mapping investigation

Once the runtime and fixtures exist:

* `feat/end-to-end-browser-gate`
* `feat/adversarial-font-mapping-fixtures`

These can run in parallel.

The browser-gate agent proves the happy path.

The adversarial-mapping agent deliberately breaks the Unicode→cmap assumption.

```text
runtime + fixtures
       │
       ├── end-to-end-browser-gate
       │
       └── adversarial-font-mapping-fixtures
```

The second branch is especially nice for parallel work because it mostly creates fixtures/tests rather than changing production architecture.

### Wave 3 — mostly sequential

This is the part I would **not parallelize much**:

```text
pdfium-glyph-identity
        ↓
fontforge-glyph-remapping
        ↓
browser-font-metrics
```

These three redefine the central font pipeline.

`pdfium-glyph-identity` changes what information the model possesses.

`fontforge-glyph-remapping` consumes that new information.

`browser-font-metrics` then establishes layout behavior based on the generated font.

Trying to build all three simultaneously means each agent invents assumptions about interfaces that are still moving.

You could have someone research/test browser metrics concurrently, but I would not merge the implementation before glyph remapping stabilizes.

### Wave 4 — this is where you can fan out hard

Once glyph identity/remapping and browser metrics are solid, you have several almost-independent workstreams.

**Font lane**

* `feat/cff-fonts`
* `feat/cid-fonts`
* `feat/type1-fonts`

These can mostly happen in parallel because they all plug into the same now-stable font-processing interface.

I’d be comfortable running 3 agents here.

**Layout lane**

* `feat/text-spacing-parity`
* `feat/text-transform-parity`

These can run in parallel if the shared `PreparedRun`/layout model has settled.

Then:

* `feat/complex-text-layout`
* `feat/vertical-writing`

Those can also run largely independently afterward.

So you could have:

```text
                 ┌─ cff-fonts
                 ├─ cid-fonts
glyph pipeline ──├─ type1-fonts
                 ├─ text-spacing-parity
                 └─ text-transform-parity
```

That’s a good 5-agent parallel wave.

### Wave 5 — visibility and document features

These can also fan out.

One lane:

```text
visibility-differential-rendering
        ↓
clipping-and-transparency
```

Those should stay sequential because the latter should build on the former.

But alongside them, independently:

* `feat/destination-semantics`
* `feat/annotations`
* `feat/background-quality`

And possibly `feat/forms`, though I’d prefer forms after annotations because some model concepts may overlap.

So something like:

```text
                ┌─ visibility-differential-rendering
                │          ↓
                │  clipping-and-transparency
                │
stable core ────├─ destination-semantics
                ├─ annotations
                └─ background-quality
```

### Type 3 can run independently later

`feat/type3-fonts` is specialized enough that once the font worker and glyph identity APIs are stable, someone can work on it largely independently of:

* CFF/CID work
* visibility work
* navigation
* packaging

It may touch FontForge infrastructure, so I wouldn’t start it before the general FontForge remapping pipeline stabilizes.

### Packaging can begin earlier than full parity

You do **not** need to wait until every PDF feature is implemented to start:

* `feat/pdfium-bundling`
* `feat/fontforge-bundling`

Those two can run in parallel once you know exactly which PDFium patch and FontForge invocation model you are shipping.

Then:

```text
pdfium-bundling ───────┐
                       ├── cross-platform-release
fontforge-bundling ────┘
```

This is another excellent 2-agent parallel lane.

I would actually start those before Type 3/forms/vector-output are finished. Packaging has enough unknowns that learning about them early is useful.

### Worker/performance branches come late

These dependencies are more sequential:

```text
conversion-workers
        ↓
page-parallelism
```

But:

* `feat/fontforge-worker-pool`

can run in parallel with `conversion-workers`, because it's a different subprocess subsystem.

Then page parallelism waits until the PDFium worker architecture is established.

---

## The maximum sensible concurrency

At the beginning, I’d use about **4 agents**:

```text
Agent A: font-integrity-contract
Agent B: runtime-bootstrap
Agent C: embedded-font-fixtures
Agent D: cross-platform-fontforge-tests
```

After the glyph-mapping foundation is solved, you could safely scale to **6–8 agents**:

```text
Agent A: CFF
Agent B: CID
Agent C: Type 1
Agent D: spacing parity
Agent E: transform parity
Agent F: destination semantics
Agent G: background quality
Agent H: PDFium/FontForge packaging
```

That is the point where the architecture becomes genuinely Lego-like: everyone is extending a stable boundary instead of negotiating what the boundary is.

The branches I would keep deliberately single-threaded are the critical spine:

```text
font-integrity
→ font identity
→ end-to-end proof
→ PDFium glyph identity
→ FontForge remapping
→ browser font metrics
```

Once that spine is stable, parallelize aggressively.
