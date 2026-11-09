use std::{
    ffi::OsString,
    fs,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;
use tempfile::TempDir;

use super::{FontJobRequest, FontJobSpec};
use crate::model::ProcessedFont;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub executable: PathBuf,
    pub executable_args: Vec<OsString>,
    pub timeout: Duration,
    pub output_format: FontOutputFormat,
}

impl WorkerConfig {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            executable_args: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
            output_format: FontOutputFormat::Woff2,
        }
    }

    pub fn with_executable_args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.executable_args = arguments.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_format(mut self, output_format: FontOutputFormat) -> Self {
        self.output_format = output_format;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontOutputFormat {
    Woff2,
    Woff,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontJobResult {
    pub font: ProcessedFont,
}

#[derive(Debug, thiserror::Error)]
pub enum FontJobError {
    #[error("font has no embedded data")]
    MissingData,
    #[error("font format is not supported by the FontForge pipeline")]
    UnsupportedFormat,
    #[error("font mapping is not proven")]
    UnprovenMapping,
    #[error("could not create font job workspace: {0}")]
    Workspace(#[from] io::Error),
    #[error("could not start FontForge: {0}")]
    Start(#[source] io::Error),
    #[error("could not wait for FontForge: {0}")]
    Wait(#[source] io::Error),
    #[error("FontForge job timed out after {0:?}")]
    Timeout(Duration),
    #[error("FontForge exited unsuccessfully: {0}")]
    Process(String),
    #[error("FontForge did not produce a valid web-font asset")]
    InvalidOutput,
    #[error("FontForge response was invalid: {0}")]
    InvalidResponse(String),
}

pub struct FontForgeWorker {
    config: WorkerConfig,
    script: PathBuf,
}

impl FontForgeWorker {
    pub fn new(config: WorkerConfig) -> Self {
        Self { config, script: Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/fontforge_worker.py") }
    }

    pub fn process(&self, request: &FontJobRequest<'_>) -> Result<FontJobResult, FontJobError> {
        let source = request.source;
        let data = source.data.as_deref().ok_or(FontJobError::MissingData)?;
        if !is_supported_font(data) {
            return Err(FontJobError::UnsupportedFormat);
        }
        if !source.mapping_proven {
            return Err(FontJobError::UnprovenMapping);
        }

        let workspace = TempDir::new()?;
        let input = workspace.path().join("source-font.bin");
        let extension = self.config.output_format.extension();
        let output = workspace.path().join(format!("processed-font.{extension}"));
        let response = workspace.path().join("response.json");
        let family_name = format!("pdf-font-{}", request.id);
        let spec = FontJobSpec::from_source(source, family_name.clone());
        let spec_path = workspace.path().join("request.json");
        fs::write(&input, data)?;
        fs::write(
            &spec_path,
            serde_json::to_vec(&spec).map_err(|error| FontJobError::InvalidResponse(error.to_string()))?,
        )?;
        let stdout_path = workspace.path().join("stdout.log");
        let stderr_path = workspace.path().join("stderr.log");
        let mut child = Command::new(&self.config.executable)
            .args(&self.config.executable_args)
            .args(["-lang=py", "-script"])
            .arg(&self.script)
            .arg(&input)
            .arg(&output)
            .arg(&response)
            .arg(&spec_path)
            // Files cannot fill a pipe and block the child while we wait.
            .stdout(fs::File::create(&stdout_path)?)
            .stderr(fs::File::create(&stderr_path)?)
            .spawn()
            .map_err(FontJobError::Start)?;
        let status = wait_for_child(&mut child, self.config.timeout)?;
        if !status.success() {
            let stderr = read_log(&stderr_path)?;
            let stdout = read_log(&stdout_path)?;
            return Err(FontJobError::Process(format!("{status}\nstderr:\n{stderr}\nstdout:\n{stdout}")));
        }
        let bytes = fs::read(&output).map_err(|_| FontJobError::InvalidOutput)?;
        if !self.config.output_format.has_signature(&bytes) {
            return Err(FontJobError::InvalidOutput);
        }
        let response = fs::read_to_string(&response)
            .map_err(|_| FontJobError::InvalidResponse("missing response file".to_owned()))?;
        let report: WorkerReport =
            serde_json::from_str(&response).map_err(|error| FontJobError::InvalidResponse(error.to_string()))?;
        validate_report(&report, &spec)?;
        let advances_by_unicode =
            spec.records.iter().zip(&report.advance_widths).map(|(record, &width)| (record.unicode, width)).collect();
        Ok(FontJobResult {
            font: ProcessedFont {
                asset_name: format!("font-{}.{}", request.id, extension),
                family_name,
                data: bytes,
                glyph_count: report.glyph_count,
                advance_widths: report.advance_widths,
                units_per_em: report.units_per_em,
                advances_by_unicode,
            },
        })
    }
}

impl FontOutputFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Woff2 => "woff2",
            Self::Woff => "woff",
        }
    }

    fn has_signature(self, data: &[u8]) -> bool {
        match self {
            Self::Woff2 => data.starts_with(b"wOF2"),
            Self::Woff => data.starts_with(b"wOFF"),
        }
    }
}

fn is_supported_font(data: &[u8]) -> bool {
    matches!(data.get(..4), Some([0, 1, 0, 0]) | Some([b'O', b'T', b'T', b'O']))
}

fn read_log(path: &Path) -> io::Result<String> {
    const MAX_LOG_BYTES: u64 = 64 * 1024;
    let mut file = fs::File::open(path)?;
    let start = file.metadata()?.len().saturating_sub(MAX_LOG_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(MAX_LOG_BYTES).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> Result<ExitStatus, FontJobError> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(error) => {
                terminate_child(child)?;
                return Err(FontJobError::Wait(error));
            }
        }
        if started.elapsed() >= timeout {
            terminate_child(child)?;
            return Err(FontJobError::Timeout(timeout));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_child(child: &mut Child) -> Result<(), FontJobError> {
    if let Err(error) = child.kill()
        && child.try_wait().map_err(FontJobError::Wait)?.is_none()
    {
        return Err(FontJobError::Wait(error));
    }
    child.wait().map_err(FontJobError::Wait)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct WorkerReport {
    family_name: String,
    glyph_count: usize,
    units_per_em: u16,
    advance_widths: Vec<u16>,
}

fn validate_report(report: &WorkerReport, spec: &FontJobSpec) -> Result<(), FontJobError> {
    if report.family_name != spec.family_name {
        return Err(FontJobError::InvalidResponse("family name does not match the request".to_owned()));
    }
    if report.glyph_count < spec.records.len() + 1 {
        return Err(FontJobError::InvalidResponse("glyph count does not include notdef and all records".to_owned()));
    }
    if report.units_per_em == 0 {
        return Err(FontJobError::InvalidResponse("units_per_em must be greater than zero".to_owned()));
    }
    if report.advance_widths.len() != spec.records.len() {
        return Err(FontJobError::InvalidResponse("advance width count does not match request records".to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::model::{FontProcessingState, FontSource, GlyphIdentity};

    fn source(identities: Vec<GlyphIdentity>) -> FontSource {
        FontSource {
            name: "Fixture".to_owned(),
            embedded: Some(true),
            data: fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts/LiberationSans-Regular.ttf"),
            )
            .ok(),
            used_unicode: identities.iter().map(|identity| identity.unicode).collect(),
            used_identities: identities,
            mapping_proven: true,
            processing: FontProcessingState::Pending,
        }
    }

    #[test]
    fn font_job_adds_one_space_without_replacing_source_glyph_identity() {
        let source = source(vec![GlyphIdentity { unicode: 'A', source_code: Some(0x41), glyph_id: Some(36) }]);
        assert!(source.data.is_some(), "Liberation Sans fixture is required");

        let spec = FontJobSpec::from_source(&source, "fixture".to_owned());

        assert_eq!(spec.records.iter().filter(|record| record.unicode == ' ').count(), 1);
        assert!(spec.records.iter().any(|record| {
            record.unicode == 'A' && record.source_code == Some(0x41) && record.source_glyph_id == Some(36)
        }));
    }

    #[test]
    fn font_job_does_not_duplicate_an_explicit_space_identity() {
        let source = source(vec![GlyphIdentity { unicode: ' ', source_code: Some(0x20), glyph_id: Some(3) }]);
        assert!(source.data.is_some(), "Liberation Sans fixture is required");

        let spec = FontJobSpec::from_source(&source, "fixture".to_owned());

        assert_eq!(spec.records.iter().filter(|record| record.unicode == ' ').count(), 1);
        assert_eq!(spec.records[0].source_code, Some(0x20));
        assert_eq!(spec.records[0].source_glyph_id, Some(3));
    }
}
