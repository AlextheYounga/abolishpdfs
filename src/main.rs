use std::process::ExitCode;

use abolishpdfs::{
    cli::Cli,
    pdfium::{CapabilityProbe, PdfiumLibrary},
};
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
            if cli.probe {
                match CapabilityProbe::inspect(&cli.input, &library) {
                    Ok(report) => match serde_json::to_string_pretty(&report) {
                        Ok(json) => {
                            println!("{json}");
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Could not serialize capability report: {error}");
                            ExitCode::FAILURE
                        }
                    },
                    Err(error) => {
                        eprintln!("PDFium capability probe failed: {error}");
                        ExitCode::FAILURE
                    }
                }
            } else {
                println!("PDFium library: {}", library.path().display());
                println!("input: {}", cli.input.display());
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("PDFium initialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}
