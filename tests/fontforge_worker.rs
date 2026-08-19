use std::{path::Path, time::Duration};

use abolishpdfs::{
    fonts::{FontForgeWorker, FontJobError, FontJobRequest, WorkerConfig},
    model::FontSource,
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
    assert_eq!(result.font.advance_widths, vec![500, 600]);
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

fn worker(mode: &str) -> FontForgeWorker {
    FontForgeWorker::new(WorkerConfig::new(Path::new(HELPER)).with_executable_args([mode]))
}

fn source() -> FontSource {
    FontSource {
        name: "Example".to_owned(),
        embedded: Some(true),
        data: Some(vec![0, 1, 0, 0]),
        used_unicode: vec!['A'],
        mapping_proven: true,
        processed: None,
    }
}
