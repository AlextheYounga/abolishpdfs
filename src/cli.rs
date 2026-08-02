use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "abolishpdfs",
    version,
    about = "Convert PDF documents to high-fidelity HTML"
)]
pub struct Cli {
    /// PDF document to convert.
    pub input: PathBuf,

    /// Path to the PDFium shared library. Overrides ABOLISHPDFS_PDFIUM_PATH.
    #[arg(long, env = "ABOLISHPDFS_PDFIUM_PATH", value_name = "PATH")]
    pub pdfium_path: Option<PathBuf>,

    /// Inspect PDFium's text and page-object capabilities without writing HTML.
    #[arg(long)]
    pub probe: bool,
}
