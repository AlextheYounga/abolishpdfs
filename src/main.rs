use std::{env, process::ExitCode};

use abolishpdfs::{
    cli::Cli,
    fonts::{FontForgeWorker, WorkerConfig},
    output::HtmlWriter,
    pdfium::{CapabilityProbe, DocumentExtractor, PdfiumLibrary},
};
use clap::Parser;

fn main() -> ExitCode {
    env_logger::init();
    let cli = Cli::parse();

    match PdfiumLibrary::resolve(cli.pdfium_path.as_deref(), None, &env::current_exe().unwrap_or_default()) {
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
            } else if cli.diagnostic {
                match DocumentExtractor::extract(&cli.input, &library) {
                    Ok(model) => match serde_json::to_string_pretty(&model) {
                        Ok(json) => {
                            println!("{json}");
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("Could not serialize document model: {error}");
                            ExitCode::FAILURE
                        }
                    },
                    Err(error) => {
                        eprintln!("Document extraction failed: {error}");
                        ExitCode::FAILURE
                    }
                }
            } else {
                match DocumentExtractor::extract(&cli.input, &library) {
                    Ok(mut model) => {
                        model.prepare_fonts(&FontForgeWorker::new(WorkerConfig::new(cli.fontforge_path)));
                        match HtmlWriter::write_to(&model, &cli.output) {
                            Ok(()) => {
                                println!("Wrote HTML output to {}", cli.output.display());
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("HTML output failed: {error}");
                                ExitCode::FAILURE
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("Document extraction failed: {error}");
                        ExitCode::FAILURE
                    }
                }
            }
        }
        Err(error) => {
            eprintln!("PDFium initialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}
