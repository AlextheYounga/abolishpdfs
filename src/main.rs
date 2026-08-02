use std::process::ExitCode;

use abolishpdfs::{cli::Cli, pdfium::PdfiumLibrary};
use clap::Parser;

fn main() -> ExitCode {
    env_logger::init();
    let cli = Cli::parse();

    match PdfiumLibrary::resolve(
        cli.pdfium_path.as_deref(),
        None,
        &std::env::current_exe().unwrap_or_default(),
    ) {
        Ok(library) => {
            println!("PDFium library: {}", library.path().display());
            println!("input: {}", cli.input.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("PDFium initialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}
