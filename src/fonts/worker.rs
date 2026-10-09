use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::{
    fonts::glyph_name,
    model::{FontId, FontSource, GlyphIdentity, ProcessedFont},
};

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

#[derive(Debug, Clone)]
pub struct FontJobRequest<'a> {
    pub id: FontId,
    pub source: &'a FontSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontJobSpec {
    pub family_name: String,
    pub records: Vec<GlyphJobRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlyphJobRecord {
    pub unicode: char,
    pub source_glyph_name: Option<String>,
    pub source_code: Option<u32>,
    pub source_glyph_id: Option<u16>,
}

impl FontJobSpec {
    pub fn from_source(source: &FontSource, family_name: String) -> Self {
        let mut records = Vec::new();
        for identity in &source.used_identities {
            let candidate = GlyphJobRecord::from_identity(identity, source.data.as_deref());
            if let Some(existing) =
                records.iter_mut().find(|record: &&mut GlyphJobRecord| record.unicode == candidate.unicode)
            {
                merge_record(existing, candidate);
            } else {
                records.push(candidate);
            }
        }
        if records.is_empty() {
            records = source
                .used_unicode
                .iter()
                .map(|&unicode| GlyphJobRecord {
                    unicode,
                    source_glyph_name: None,
                    source_code: None,
                    source_glyph_id: None,
                })
                .collect();
        }
        Self { family_name, records }
    }
}

impl GlyphJobRecord {
    fn from_identity(identity: &GlyphIdentity, data: Option<&[u8]>) -> Self {
        Self {
            unicode: identity.unicode,
            source_glyph_name: identity
                .glyph_id
                .and_then(|id| data.and_then(|bytes| glyph_name(bytes, id).ok().flatten()).map(str::to_owned)),
            source_code: identity.source_code,
            source_glyph_id: identity.glyph_id,
        }
    }
}

fn merge_record(existing: &mut GlyphJobRecord, candidate: GlyphJobRecord) {
    if existing.source_glyph_name.is_none() {
        existing.source_glyph_name = candidate.source_glyph_name;
    }
    if existing.source_code.is_none() {
        existing.source_code = candidate.source_code;
    }
    if existing.source_glyph_id.is_none() {
        existing.source_glyph_id = candidate.source_glyph_id;
    }
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
        let mut child = Command::new(&self.config.executable)
            .args(&self.config.executable_args)
            .args(["-lang=py", "-script"])
            .arg(&self.script)
            .arg(&input)
            .arg(&output)
            .arg(&response)
            .arg(&spec_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(FontJobError::Start)?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(FontJobError::Start)? {
                if !status.success() {
                    let stderr = child.stderr.take().map(read_stream).transpose()?.unwrap_or_default();
                    return Err(FontJobError::Process(stderr));
                }
                break;
            }
            if started.elapsed() >= self.config.timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FontJobError::Timeout(self.config.timeout));
            }
            thread::sleep(Duration::from_millis(10));
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

fn read_stream(mut stream: impl io::Read) -> io::Result<String> {
    let mut output = String::new();
    stream.read_to_string(&mut output)?;
    Ok(output)
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
