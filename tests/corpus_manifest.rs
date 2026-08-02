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
}

#[derive(Deserialize)]
struct Expected {
    pages: usize,
    text_objects_min: usize,
    links: usize,
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
        assert!(
            Path::new("tests/fixtures").join(&fixture.path).is_file(),
            "missing {}",
            fixture.path
        );
        assert!(
            !fixture.classifications.is_empty(),
            "{} has no classifications",
            fixture.id
        );
        assert!(fixture.expected.pages > 0);
        assert!(fixture.expected.text_objects_min > 0);
        assert!(fixture.expected.links <= 1);
    }
    assert!(
        manifest
            .external_corpus
            .iter()
            .all(|entry| entry.status == "pending")
    );
}
