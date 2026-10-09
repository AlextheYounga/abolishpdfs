use std::{collections::BTreeMap, fs, path::Path, time::Duration};

use abolishpdfs::{
    fonts::{FontForgeWorker, FontJobError, FontJobRequest, FontJobSpec, WorkerConfig},
    model::{FontProcessingState, FontSource, GlyphIdentity},
};

const HELPER: &str = env!("CARGO_BIN_EXE_fontforge_test_helper");

#[test]
fn worker_validates_generated_asset_and_metrics() {
    let worker = worker("success");
    let source = source();

    let result = worker.process(&FontJobRequest { id: 3, source: &source }).expect("fake worker should succeed");

    assert_eq!(result.font.asset_name, "font-3.woff2");
    assert_eq!(result.font.family_name, "pdf-font-3");
    assert_eq!(result.font.glyph_count, 2);
    assert_eq!(result.font.advance_widths, vec![600]);
    assert_eq!(result.font.units_per_em, 1000);
    assert_eq!(result.font.advances_by_unicode.get(&'A'), Some(&600));
}

#[test]
fn spec_falls_back_to_unicode_order_without_identity() {
    let spec = FontJobSpec::from_source(&source_with_unicode(&['B', 'A']), "pdf-font-3".to_owned());
    assert_eq!(spec.records.iter().map(|record| record.unicode).collect::<Vec<_>>(), vec!['B', 'A']);
    assert!(spec.records.iter().all(|record| record.source_code.is_none() && record.source_glyph_id.is_none()));
}

#[test]
fn spec_deduplicates_and_merges_identity_fields_in_first_appearance_order() {
    let mut source = source_with_unicode(&['A', 'B']);
    source.data = Some(fixture_font());
    source.used_identities = vec![
        GlyphIdentity { unicode: 'B', source_code: None, glyph_id: Some(36) },
        GlyphIdentity { unicode: 'A', source_code: Some(0x41), glyph_id: None },
        GlyphIdentity { unicode: 'B', source_code: Some(0x42), glyph_id: None },
    ];
    let spec = FontJobSpec::from_source(&source, "pdf-font-3".to_owned());
    assert_eq!(serde_json::to_string(&spec).ok(), Some(r#"{"family_name":"pdf-font-3","records":[{"unicode":"B","source_glyph_name":"A","source_code":66,"source_glyph_id":36},{"unicode":"A","source_glyph_name":null,"source_code":65,"source_glyph_id":null}]}"#.to_owned()));
}

#[test]
fn identity_spec_round_trips_through_fake_worker() {
    let mut source = source_with_unicode(&['B']);
    source.data = Some(fixture_font());
    source.used_identities = vec![GlyphIdentity { unicode: 'B', source_code: Some(0x41), glyph_id: Some(36) }];
    let result = worker("success").process(&FontJobRequest { id: 3, source: &source });
    assert!(result.is_ok());
    assert_eq!(result.ok().map(|result| result.font.advances_by_unicode), Some(BTreeMap::from([('B', 600)])));
}

#[test]
fn worker_rejects_unproven_mapping_before_starting_process() {
    let mut source = source();
    source.mapping_proven = false;

    assert!(matches!(
        worker("missing-executable").process(&FontJobRequest { id: 0, source: &source }),
        Err(FontJobError::UnprovenMapping)
    ));
}

#[test]
fn worker_reports_missing_executable() {
    let worker = FontForgeWorker::new(WorkerConfig::new("missing-fontforge"));
    let source = source();

    assert!(matches!(worker.process(&FontJobRequest { id: 0, source: &source }), Err(FontJobError::Start(_))));
}

#[test]
fn worker_reports_subprocess_failure() {
    let source = source();

    assert!(matches!(
        worker("process-failure").process(&FontJobRequest { id: 0, source: &source }),
        Err(FontJobError::Process(_))
    ));
}

#[test]
fn worker_rejects_invalid_generated_asset() {
    let source = source();

    assert!(matches!(
        worker("invalid-asset").process(&FontJobRequest { id: 0, source: &source }),
        Err(FontJobError::InvalidOutput)
    ));
}

#[test]
fn worker_rejects_missing_generated_asset() {
    let source = source();

    assert!(matches!(
        worker("missing-asset").process(&FontJobRequest { id: 0, source: &source }),
        Err(FontJobError::InvalidOutput)
    ));
}

#[test]
fn worker_rejects_malformed_response() {
    let source = source();

    assert!(matches!(
        worker("malformed-response").process(&FontJobRequest { id: 0, source: &source }),
        Err(FontJobError::InvalidResponse(_))
    ));
}

#[test]
fn worker_reports_timeout_after_terminating_process() {
    let mut config = WorkerConfig::new(Path::new(HELPER)).with_executable_args(["timeout"]);
    config.timeout = Duration::from_millis(20);
    let worker = FontForgeWorker::new(config);
    let source = source();

    assert!(matches!(worker.process(&FontJobRequest { id: 0, source: &source }), Err(FontJobError::Timeout(_))));
}

#[test]
fn worker_rejects_misaligned_response_widths() {
    assert!(
        matches!(worker("invalid-widths").process(&FontJobRequest { id: 0, source: &source() }), Err(FontJobError::InvalidResponse(message)) if message.contains("advance width count"))
    );
}

#[test]
fn worker_rejects_zero_units_per_em() {
    assert!(
        matches!(worker("invalid-units").process(&FontJobRequest { id: 0, source: &source() }), Err(FontJobError::InvalidResponse(message)) if message.contains("units_per_em"))
    );
}

fn worker(mode: &str) -> FontForgeWorker {
    FontForgeWorker::new(WorkerConfig::new(Path::new(HELPER)).with_executable_args([mode]))
}

fn source() -> FontSource {
    source_with_unicode(&['A'])
}

fn source_with_unicode(unicode: &[char]) -> FontSource {
    FontSource {
        name: "Example".to_owned(),
        embedded: Some(true),
        data: Some(vec![0, 1, 0, 0]),
        used_unicode: unicode.to_vec(),
        used_identities: Vec::new(),
        mapping_proven: true,
        processing: FontProcessingState::Pending,
    }
}

fn fixture_font() -> Vec<u8> {
    fs::read("tests/fixtures/fonts/LiberationSans-Regular.ttf").unwrap_or_default()
}
