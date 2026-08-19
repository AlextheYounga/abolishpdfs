use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    version: u32,
    fixtures: Vec<Fixture>,
    external_corpus: Vec<ExternalCorpus>,
}

#[derive(Deserialize)]
struct Fixture {
    id: String,
    path: String,
    classifications: Vec<String>,
    expected: Expected,
    clipboard: Clipboard,
    navigation: Navigation,
    assets: Assets,
    fonts: Option<Fonts>,
    screenshot: Screenshot,
}

#[derive(Deserialize)]
struct Expected {
    pages: usize,
    text_objects_min: usize,
    links: usize,
    native_text: Vec<String>,
    page_sizes: Vec<PageSize>,
    fallback: String,
}

#[derive(Deserialize)]
struct PageSize {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct Clipboard {
    expected_text: String,
    status: String,
}

#[derive(Deserialize)]
struct Navigation {
    hrefs: Vec<String>,
    fragments: Vec<String>,
}

#[derive(Deserialize)]
struct Assets {
    required: Vec<String>,
}

#[derive(Deserialize)]
struct Fonts {
    source: Vec<String>,
    embedded_count: usize,
    processed_assets: Vec<String>,
    processing: String,
}

#[derive(Deserialize)]
struct Screenshot {
    status: String,
    baseline: Option<String>,
    max_diff_ratio: f32,
    max_diff_pixels: u64,
}

#[derive(Deserialize)]
struct ExternalCorpus {
    status: String,
}

#[test]
fn compatibility_manifest_has_explicit_classifications() {
    let manifest: Manifest = serde_json::from_str(include_str!("fixtures/manifest.json")).unwrap();
    assert_eq!(manifest.version, 1);
    assert!(!manifest.fixtures.is_empty());
    for fixture in manifest.fixtures {
        assert!(!fixture.id.is_empty());
        assert!(Path::new("tests/fixtures").join(&fixture.path).is_file(), "missing {}", fixture.path);
        assert!(!fixture.classifications.is_empty(), "{} has no classifications", fixture.id);
        assert!(fixture.expected.pages > 0);
        assert!(fixture.expected.text_objects_min > 0);
        assert!(fixture.expected.links <= 1);
        assert_eq!(fixture.expected.page_sizes.len(), fixture.expected.pages);
        assert!(fixture.expected.page_sizes.iter().all(|size| size.width > 0 && size.height > 0));
        assert!(!fixture.expected.native_text.iter().any(String::is_empty));
        assert!(matches!(fixture.expected.fallback.as_str(), "none" | "background"));
        assert_eq!(fixture.clipboard.status, "required");
        assert!(!fixture.clipboard.expected_text.is_empty());
        assert!(fixture.navigation.hrefs.iter().all(|href| href.starts_with("http")));
        assert!(fixture.navigation.fragments.iter().all(|fragment| fragment.starts_with("#page-")));
        assert!(fixture.assets.required.iter().all(|asset| !asset.contains("..")));
        if fixture.classifications.iter().any(|classification| classification == "embedded-truetype") {
            let fonts = fixture.fonts.as_ref().expect("embedded fixture needs font expectations");
            assert_eq!(fonts.source.len(), fonts.embedded_count);
            assert_eq!(fonts.processed_assets.len(), fonts.embedded_count);
            assert_eq!(fonts.processing, "required");
            assert!(fonts.processed_assets.iter().all(|asset| asset.ends_with(".woff2")));
        }
        assert!(matches!(fixture.screenshot.status.as_str(), "capture-only" | "compare"));
        if fixture.screenshot.status == "compare" {
            assert!(fixture.screenshot.baseline.is_some());
        }
        assert!((0.0..=1.0).contains(&fixture.screenshot.max_diff_ratio));
        assert!(fixture.screenshot.max_diff_pixels > 0);
    }
    assert!(manifest.external_corpus.iter().all(|entry| entry.status == "pending"));
}
