use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;
use tempfile::TempDir;

use crate::model::{FontId, FontSource, ProcessedFont};

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
        fs::write(&input, data)?;
        let family_name = format!("pdf-font-{}", request.id);
        let unicode =
            source.used_unicode.iter().map(|character| u32::from(*character).to_string()).collect::<Vec<_>>().join(",");
        let mut child = Command::new(&self.config.executable)
            .args(&self.config.executable_args)
            .args(["-lang=py", "-script"])
            .arg(&self.script)
            .arg(&input)
            .arg(&output)
            .arg(&response)
            .arg(&family_name)
            .arg(unicode)
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
        validate_report(&report, &family_name, source.used_unicode.len())?;
        Ok(FontJobResult {
            font: ProcessedFont {
                asset_name: format!("font-{}.{}", request.id, extension),
                family_name,
                data: bytes,
                glyph_count: report.glyph_count,
                advance_widths: report.advance_widths,
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
    advance_widths: Vec<u16>,
}

fn validate_report(report: &WorkerReport, family_name: &str, used_count: usize) -> Result<(), FontJobError> {
    if report.family_name != family_name
        || report.glyph_count < used_count
        || report.advance_widths.len() != report.glyph_count
    {
        return Err(FontJobError::InvalidResponse(
            "family, glyph count, or metrics do not match the request".to_owned(),
        ));
    }
    Ok(())
}
