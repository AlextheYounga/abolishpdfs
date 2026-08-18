# abolishpdfs

A modern, cross-platform PDF-to-HTML converter built in Rust with PDFium and FontForge.

## Mission

`abolishpdfs` will convert PDF documents into high-fidelity HTML while preserving the defining qualities that make pdf2htmlEX valuable:

* Visible browser-native text
* Accurate page layout
* Searchable and selectable content
* Embedded document fonts
* Images and vector backgrounds
* Character spacing and transformations
* Links and internal destinations
* Document outlines
* Single-file and split-page output
* Reliable operation on Windows, macOS, and Linux

The project is not merely a PDF viewer with a transparent text layer. Its primary goal is to reconstruct PDF text as real HTML text as accurately as reasonably possible.

Rendered backgrounds and invisible text overlays remain available as correctness fallbacks for cases that cannot yet be reconstructed safely.

## Project identity

`abolishpdfs` is an independent reimplementation inspired by the behavior and goals of pdf2htmlEX.


## Success standard

The project should aim to match or exceed pdf2htmlEX across a representative compatibility corpus.

Exact byte-for-byte HTML output is unnecessary. The relevant equivalence is behavioral:

1. Pages look substantially the same.
2. Visible text remains browser-native whenever technically possible.
3. Text selection follows visible characters.
4. Copied text is reasonable.
5. Fonts preserve the original document appearance.
6. Images, paths, clipping, and transparency remain visually correct.
7. Links, outlines, and destinations work.
8. Unsupported cases degrade without visibly corrupting the page.
9. The converter installs and runs natively on all three desktop operating systems.

## Technology choices

### Rust

Rust owns:

* Document orchestration
* PDFium adaptation
* The internal document model
* Text reconstruction
* Layout compensation
* Visibility decisions
* HTML and CSS generation
* Asset management
* Worker management
* Error reporting
* Cross-platform CLI behavior

### PDFium

PDFium replaces Poppler as the PDF parser, interpreter, page-object engine, renderer, and extraction engine.

PDFium is not expected to expose every internal event Poppler exposed through `OutputDev`. The architecture must therefore be based on PDFium’s actual public capabilities rather than pretending it is a one-for-one Poppler replacement.

### FontForge

FontForge is the primary font-processing backend for the initial implementation.

It will handle:

* Embedded TrueType fonts
* OpenType fonts
* CFF fonts
* CID fonts
* Type 1 fonts
* Font subsetting
* Glyph reencoding
* Unicode mapping
* Width and metric rewriting
* Missing-space insertion
* Font naming
* Hinting
* Browser-compatible output generation
* Type 3 replacement fonts where practical

FontForge supports headless command-line and Python-script execution, including opening and generating fonts without launching its user interface.


## High-level architecture

```text
PDF
 │
 ▼
PDFium worker
 │
 ▼
Stable Rust document model
 │
 ├── document preprocessor
 ├── font usage collector
 ├── visibility analyzer
 ├── text layout engine
 ├── background generator
 ├── links and outline processor
 └── HTML/CSS writer
 │
 ├──────────────► FontForge worker
 │                   │
 │                   ▼
 │              processed web fonts
 │
 ▼
HTML document and assets
```

The architecture separates three concerns:

1. PDF interpretation
2. Font reconstruction
3. HTML reconstruction

## Stable document model

No PDFium handles should escape the PDFium adapter.

```rust
pub struct Document {
    pub metadata: DocumentMetadata,
    pub pages: Vec<Page>,
    pub fonts: Vec<FontResource>,
    pub outline: Vec<OutlineItem>,
    pub destinations: Vec<NamedDestination>,
}

pub struct Page {
    pub number: PageNumber,
    pub size: PageSize,
    pub rotation: Rotation,
    pub objects: Vec<PageObject>,
    pub characters: Vec<Character>,
    pub links: Vec<Link>,
    pub annotations: Vec<Annotation>,
}

pub struct Character {
    pub unicode: String,
    pub source_code: Option<u32>,
    pub glyph_id: Option<u32>,
    pub bounds: Quad,
    pub transform: AffineTransform,
    pub origin: Point,
    pub advance: Vector,
    pub font_id: FontId,
    pub font_size: f32,
    pub render_mode: TextRenderMode,
    pub text_object_id: Option<ObjectId>,
}

pub struct FontResource {
    pub id: FontId,
    pub family_name: Option<String>,
    pub subtype: FontSubtype,
    pub embedded_data: Option<FontData>,
    pub glyph_usage: GlyphUsage,
}
```

This model should contain the information needed by the converter, not every value PDFium can expose.

## Conversion pipeline

## Pass 1: Document inspection

The first pass gathers document-wide information:

* Page count
* Page dimensions
* Page rotations
* Font resources
* Embedded font data
* Used characters and glyphs
* Named destinations
* Outlines
* Links
* Annotations
* Unsupported features
* Encryption and permissions

PDF conversion requires some document-wide knowledge before final fonts and CSS can be emitted. The new implementation should preserve a deliberate preprocessing pass without inheriting the old implementation’s global mutable renderer.

The result is an immutable `DocumentPlan`.

```rust
pub struct DocumentPlan {
    pub pages: Vec<PagePlan>,
    pub fonts: Vec<FontPlan>,
    pub destinations: Vec<NamedDestination>,
    pub warnings: Vec<ConversionWarning>,
}
```

## Pass 2: Font preparation

Each document font receives a stable identifier and complete usage set.

```rust
pub struct FontPlan {
    pub font_id: FontId,
    pub source: ExtractedFont,
    pub used_glyphs: BTreeSet<GlyphId>,
    pub unicode_map: BTreeMap<GlyphId, UnicodeSequence>,
    pub widths: BTreeMap<GlyphId, f32>,
    pub output_family: String,
}
```

The FontForge worker processes each font before page HTML is finalized.

## Pass 3: Page reconstruction

Each page is converted into:

* Reconstructed HTML text
* A non-text background
* Links and annotations
* Page-specific CSS references
* Diagnostics
* Fallback regions

```rust
pub struct PageResult {
    pub page_number: PageNumber,
    pub html_fragment: PathBuf,
    pub background_assets: Vec<PathBuf>,
    pub used_fonts: Vec<FontId>,
    pub links: Vec<Link>,
    pub warnings: Vec<ConversionWarning>,
}
```

## Pass 4: Document assembly

The coordinator:

* Deduplicates shared assets
* Writes font declarations
* Writes shared CSS
* Inserts pages in order
* Resolves internal destinations
* Writes outlines
* Embeds assets when requested
* Produces the final manifest
* Emits a conversion report

## Text reconstruction

Visible HTML text is the default target.

Each text object should be evaluated for reconstruction, not automatically relegated to a background.

```rust
pub enum TextDisposition {
    ReconstructedHtml,
    ReconstructedHtmlWithBackgroundAssist,
    BackgroundWithSelectionOverlay,
    BackgroundOnly,
}
```

### `ReconstructedHtml`

The visible text is emitted as browser text and removed from the rendered PDF background.

### `ReconstructedHtmlWithBackgroundAssist`

Browser text supplies most of the visible glyph, while difficult decorations or effects remain in the background.

Examples may include:

* Text shadows
* Unusual stroke effects
* Complex clipping edges
* Decorative overlays

### `BackgroundWithSelectionOverlay`

PDFium renders the visible text. Transparent HTML text provides search, selection, and copying.

This is a compatibility fallback, not the primary architecture.

### `BackgroundOnly`

Used only when no reliable text mapping exists.

Examples include:

* Text converted entirely to vector paths
* Unrecoverable mappings
* Image-only scans without OCR
* Corrupt text objects that cannot be interpreted safely

## Text layout engine

The layout engine must preserve the PDF’s glyph placement rather than asking the browser to reshape the original text freely.

Responsibilities include:

* Grouping characters into compatible text runs
* Preserving PDF writing order
* Calculating baseline position
* Applying text matrices
* Applying page transformations
* Applying font size and horizontal scaling
* Correcting browser glyph advances
* Emitting letter spacing
* Emitting word spacing
* Emitting per-character offsets where necessary
* Handling rotation and skew
* Handling RTL text
* Handling vertical writing
* Handling ligatures
* Handling decomposed Unicode sequences
* Handling combining marks
* Handling missing spaces
* Preserving text selection order

A text run should carry explicit positioning information:

```rust
pub struct TextRun {
    pub font_id: FontId,
    pub font_size: f32,
    pub transform: AffineTransform,
    pub origin: Point,
    pub characters: Vec<PositionedCharacter>,
    pub style: TextStyle,
}

pub struct PositionedCharacter {
    pub unicode: String,
    pub expected_advance: f32,
    pub correction: f32,
}
```

CSS optimization happens after correctness.

The early implementation may emit verbose per-run or per-character positioning. Once parity is established, equivalent styles can be interned and compacted.

## FontForge worker

FontForge should not be linked throughout the Rust codebase.

It should be isolated behind a versioned worker protocol.

```text
abolishpdfs
    │
    ├── writes font job
    │
    ▼
fontforge-worker
    │
    ├── loads source font
    ├── applies glyph mapping
    ├── applies Unicode mapping
    ├── adjusts widths
    ├── adjusts metrics
    ├── subsets glyphs
    ├── repairs output
    └── generates browser font
    │
    ▼
processed font + result manifest
```

The worker may initially use FontForge’s Python scripting API. Should the scripting API prove insufficient, a small native bridge may be introduced inside the worker without exposing FontForge types to the Rust core.

### Font job protocol

```rust
pub struct FontJob {
    pub protocol_version: u32,
    pub job_id: JobId,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub input_format: FontFormat,
    pub output_format: FontFormat,
    pub family_name: String,
    pub glyphs: Vec<GlyphRequest>,
    pub metrics: FontMetricOverrides,
    pub options: FontOptions,
}

pub struct GlyphRequest {
    pub source_code: Option<u32>,
    pub source_glyph_id: Option<u32>,
    pub output_unicode: UnicodeSequence,
    pub expected_width: f32,
}
```

### Font result protocol

```rust
pub struct FontResult {
    pub protocol_version: u32,
    pub job_id: JobId,
    pub status: FontJobStatus,
    pub output_path: Option<PathBuf>,
    pub generated_family_name: Option<String>,
    pub glyph_map: BTreeMap<GlyphId, GlyphId>,
    pub warnings: Vec<FontWarning>,
    pub error: Option<FontError>,
}
```

Large binary font data should travel through files, not JSON or standard output.

Structured control messages may use JSON Lines initially. The protocol should remain versioned so the worker can later be replaced without changing the converter core.

## Font pipeline

The initial font pipeline should use FontForge for all supported embedded fonts.

```text
embedded PDF font
        │
        ▼
extract source bytes
        │
        ▼
identify font format and PDF encoding
        │
        ▼
collect used source codes and glyphs
        │
        ▼
construct Unicode mapping
        │
        ▼
FontForge worker
        │
        ├── flatten CID font where needed
        ├── reencode glyph order
        ├── rewrite cmap
        ├── rewrite widths
        ├── adjust ascent and descent
        ├── add missing space glyph
        ├── subset
        ├── hint where appropriate
        └── generate browser font
        │
        ▼
TTF, OTF, WOFF, or WOFF2
```

The priority is parity and browser acceptance, not immediately reducing native dependencies.

## Type 3 fonts

Type 3 fonts require a dedicated subsystem because their glyphs are PDF graphics programs rather than ordinary TrueType or CFF outlines.

The target solution is:

1. Identify each used Type 3 glyph.
2. Execute its PDF glyph program.
3. Capture its vector appearance.
4. Convert the result into SVG-compatible outlines where possible.
5. Import the glyph into a newly generated FontForge font.
6. Assign Unicode and metrics.
7. Emit a browser-compatible replacement font.

Where vector capture is unavailable, the affected Type 3 text may temporarily remain in the background with a selection overlay.

Type 3 parity is a planned compatibility milestone, not a permanent non-goal.

## Background generation

The background should contain page content that is not reconstructed as browser text:

* Images
* Paths
* Fills and strokes
* Shadings
* Patterns
* Form objects
* Complex clipping
* Unsupported text effects
* Text assigned to a fallback mode

### Initial background mode

Render a high-resolution bitmap through PDFium after suppressing eligible reconstructed text objects.

### Advanced background mode

Add vector background output where PDFium exposes enough information to recreate paths, images, transforms, clipping, and paint order safely.

The long-term output options should include:

```rust
pub enum BackgroundFormat {
    Png,
    WebP,
    Jpeg,
    Svg,
    Auto,
}
```

`Auto` should select the highest-quality safe representation for each page or region.

## Text suppression

To avoid drawing visible PDF text underneath reconstructed HTML text, eligible PDF text objects must be excluded from the background.

Potential mechanisms include:

* Deactivating or deleting page text objects
* Rendering from a cloned page with selected objects removed
* Regenerating page content before rendering
* Separately reconstructing non-text page objects
* Masking eligible text regions only where deletion is impossible

Text suppression must be proven against:

* Nested form objects
* Transparency groups
* Clipping paths
* Text used as a clipping mask
* Text mixed with image masks
* Type 3 fonts
* Repeated form instances
* Shared resource objects

This is one of the first major PDFium feasibility gates.

## Visibility and covered-text detection

The converter must determine whether extracted text should appear visibly in HTML.

Questions include:

* Is the text fill or stroke visible?
* Is it clipped?
* Is it fully or partially covered?
* Is it an invisible OCR layer?
* Is it duplicated?
* Is it used only as a clipping path?
* Is it underneath an opaque image or fill?
* Is it inside an unsupported transparency group?

Use a layered approach:

1. Text render-mode inspection
2. Object-order analysis
3. Bounding-box and clip intersection
4. Opacity and paint-style checks
5. Raster visibility verification for ambiguous cases

For difficult cases, compare renders with and without selected text objects to determine whether they contribute visible pixels.

Correctness takes priority over speed during the first implementation. Expensive verification can later be limited to ambiguous regions.

## HTML structure

Generated pages should use simple, inspectable HTML:

```html
<div class="page" data-page-number="1">
  <img class="page-background" src="page-1.webp" alt="">
  <div class="text-layer">
    <span class="text-run font-3 style-8">Example text</span>
  </div>
  <div class="link-layer">
    <a href="#destination-4" class="internal-link"></a>
  </div>
</div>
```

Support:

* Embedded assets
* External assets
* One HTML file
* Split-page output
* Lazy page loading
* Internal links
* External links
* Outlines
* Page zooming
* Printing
* Search
* Selection
* Copying

The generated HTML should not require a heavy frontend framework.

## Worker architecture

PDFium recommends process-level parallelism rather than concurrent calls from multiple threads. `pdfium-render` protects PDFium with a mutex, serializing calls within one process.

Use persistent converter workers:

```text
abolishpdfs coordinator
├── pdfium worker 1
├── pdfium worker 2
├── pdfium worker 3
└── pdfium worker 4
```

Each worker owns:

* One PDFium library instance
* Its own document handles
* Its own temporary directory
* Its own page jobs
* No mutable state shared with another worker

Independent documents can be converted in parallel.

A large document may later be divided into page ranges after document-wide preprocessing and font planning are complete.

## Converter protocol

```rust
pub enum WorkerRequest {
    InspectDocument {
        job_id: JobId,
        input: PathBuf,
        password: Option<String>,
    },

    ConvertPage {
        job_id: JobId,
        document: DocumentHandle,
        page: PageNumber,
        plan: PagePlan,
    },

    RenderBackground {
        job_id: JobId,
        document: DocumentHandle,
        page: PageNumber,
        excluded_objects: Vec<ObjectId>,
        render: RenderOptions,
    },

    Cancel {
        job_id: JobId,
    },

    Shutdown,
}
```

```rust
pub enum WorkerEvent {
    Started {
        job_id: JobId,
    },

    Progress {
        job_id: JobId,
        stage: ConversionStage,
        current: usize,
        total: usize,
    },

    PageCompleted {
        job_id: JobId,
        result: PageResult,
    },

    Warning {
        job_id: JobId,
        warning: ConversionWarning,
    },

    Failed {
        job_id: JobId,
        error: ConversionError,
    },

    Completed {
        job_id: JobId,
    },
}
```

The initial protocol may use JSON Lines over standard input and output.

Binary assets should be written to assigned job directories and referenced by path.

## Development milestones

## Milestone 0: Independence and regression foundation

Deliverables:

* Independence policy
* Compatibility corpus
* Reference pdf2htmlEX output
* Reference PDFium renders
* Browser screenshot harness
* Text-selection and copy tests
* Font inventory
* Link and outline fixtures

The corpus should cover:

* TrueType
* OpenType
* CFF
* CID
* Type 1
* Type 3
* CJK
* RTL
* Vertical text
* Ligatures
* Combining marks
* Missing `ToUnicode`
* Rotated and skewed text
* Clipped text
* Covered text
* Invisible OCR text
* Nested form objects
* Images
* Paths
* Transparency
* Shadings
* Patterns
* Forms
* Annotations
* Encryption
* Malformed PDFs
* Scanned documents

### Gate

No compatibility claim is accepted without a reproducible fixture.

## Milestone 1: PDFium capability map

Build:

```text
abolishpdfs inspect document.pdf
```

Export:

* Page objects
* Character geometry
* Text-object associations
* Font identities
* Embedded font bytes
* Render modes
* Object ordering
* Clipping
* Links
* Destinations
* Outlines
* Annotations

### Gate

Determine whether PDFium exposes enough information for:

* Text-object suppression
* Font-to-text association
* Glyph mapping
* Nested-object traversal
* Visibility decisions

Document every confirmed API gap.

## Milestone 2: First real-text vertical slice

Convert a simple one-page PDF containing:

* Embedded TrueType font
* Several text runs
* One image
* One link
* Basic vector content

Output must contain:

* Real visible HTML text
* Processed embedded font
* Correct non-text background
* Working link
* Browser screenshot close to PDFium’s reference rendering

A complete raster page with transparent text does not satisfy this milestone.

## Milestone 3: FontForge pipeline

Implement:

* Font extraction
* Font job protocol
* FontForge worker
* Unicode remapping
* Width rewriting
* Subsetting
* Browser-font output
* Font-result validation
* Font caching
* Crash and timeout handling

### Gate

Ordinary TrueType, OpenType, CFF, CID, and Type 1 fixtures must render successfully or have a documented technical blocker.

## Milestone 4: Document-level conversion

Implement:

* Preprocessing pass
* Shared fonts
* Multiple pages
* Deterministic CSS
* Shared assets
* Named destinations
* Outlines
* Single-file output
* Split-page output

### Gate

A multi-page office or academic PDF should convert with real visible HTML text across all normal pages.

## Milestone 5: Layout parity

Implement:

* Character advances
* Baseline positioning
* Word spacing
* Letter spacing
* Horizontal scaling
* Transform matrices
* Rotation
* Skew
* Ligatures
* Combining marks
* RTL
* Vertical text

### Gate

Visual text-position differences must remain within defined tolerances across the layout corpus.

## Milestone 6: Visibility and paint order

Implement:

* Hidden-text detection
* Covered-text detection
* Clip handling
* Duplicate-text handling
* OCR-layer handling
* Transparency checks
* Ambiguous-region raster verification

### Gate

No fixture may incorrectly reveal hidden text or place covered text above foreground graphics.

## Milestone 7: Advanced font compatibility

Implement and validate:

* Difficult CID fonts
* Broken Unicode mappings
* Malformed embedded fonts
* Font substitutions
* Missing glyph recovery
* Type 3 fonts
* Unusual font metrics

### Gate

The advanced font corpus should either reconstruct accurately or produce an explicitly documented localized fallback.

## Milestone 8: Background parity

Implement:

* High-resolution bitmap backgrounds
* Region-level backgrounds
* SVG background generation where possible
* Correct printing behavior
* Background asset optimization

### Gate

Non-text page content should remain visually equivalent at normal zoom and printing resolutions.

## Milestone 9: Links, forms, and annotations

Implement:

* External links
* Internal destinations
* Outlines
* Common annotations
* Form appearance preservation
* Optional interactive form support

### Gate

Navigation and common document interactions should match the source PDF.

## Milestone 10: Cross-platform distribution

Targets:

* Windows x86-64
* macOS Apple Silicon
* macOS Intel
* Linux x86-64

Bundle:

* `abolishpdfs`
* PDFium
* FontForge worker
* FontForge runtime dependencies
* Shared resources
* Version manifest

### Gate

A clean machine on each platform must convert the same regression corpus without external installations.

## Milestone 11: Performance and process workers

Implement:

* Persistent PDFium worker pool
* Persistent or pooled FontForge workers
* Concurrent independent documents
* Optional page-range processing
* Cancellation
* Timeouts
* Worker crash recovery
* Asset caching
* Font-result caching

Performance optimization occurs only after parity measurements exist.

## Testing

## Visual tests

Compare:

```text
PDFium full-page rendering
versus
browser screenshot of abolishpdfs output
```

Metrics:

* Pixel difference
* Changed-region area
* Text-boundary alignment
* Missing visible content
* Unexpected visible content
* Background resolution

Also compare against pdf2htmlEX as a behavioral reference.

## Text tests

Measure:

* Search results
* Copied text
* Unicode correctness
* Reading order
* Selection bounds
* Ligature behavior
* RTL behavior
* Vertical-text behavior

## Font tests

Measure:

* Browser font-loading success
* Glyph coverage
* Output glyph mapping
* Width accuracy
* Baseline accuracy
* Family-name isolation
* Font-subset size
* FontForge warnings
* Font-generation reproducibility

## Cross-platform tests

Run identical fixtures on:

* Windows
* macOS
* Linux

Compare generated HTML, metadata, screenshots, fonts, and warnings.

## Compatibility report

Every conversion should optionally emit:

```json
{
  "document": "example.pdf",
  "pages": 12,
  "reconstructed_text_objects": 483,
  "assisted_text_objects": 4,
  "selection_overlay_objects": 2,
  "background_only_objects": 0,
  "fonts": {
    "processed": 7,
    "failed": 0
  },
  "warnings": []
}
```

This makes fallback measurable rather than invisible.

## Version goals

## Version 0.1

* Cross-platform PDFium loading
* FontForge worker
* One-page real-text conversion
* TrueType and OpenType support
* Bitmap non-text background
* Basic links
* Regression framework

## Version 0.5

* Multi-page conversion
* CFF and CID support
* Shared fonts and CSS
* Strong layout compensation
* Links and outlines
* Windows, macOS, and Linux builds
* Controlled fallback reporting

## Version 1.0

* Near-pdf2htmlEX fidelity across the supported corpus
* Real visible HTML text as the normal result
* Broad FontForge-backed font support
* Covered and hidden-text handling
* Type 3 strategy
* Reliable links and outlines
* Single-file and split-page output
* Deterministic conversion
* Native cross-platform packages
* Documented compatibility and fallback behavior

## Engineering priorities

When tradeoffs arise, use this order:

1. Preserve visible correctness.
2. Preserve real HTML text.
3. Preserve text selection and copying.
4. Preserve font fidelity.
5. Preserve links and document structure.
6. Preserve vector output.
7. Reduce output size.
8. Improve conversion speed.
9. Replace native dependencies.

The project is successful because it modernizes and broadens the availability of high-fidelity PDF-to-HTML conversion—not because it maximizes the percentage of code written in Rust.
