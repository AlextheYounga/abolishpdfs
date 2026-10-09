mod background;
mod document;
mod library;
mod navigation;
mod paint;
mod probe;
mod report;

pub use document::{DocumentExtractionError, DocumentExtractor};
pub use library::{PdfiumLibrary, PdfiumLibraryError};
pub use probe::{CapabilityProbe, CapabilityProbeError};
pub use report::CapabilityReport;

use std::path::PathBuf;

use pdfium_render::prelude::{Pdfium, PdfiumError};

/// Expected abolishpdfs extension ABI version of the runtime.
///
/// Re-exported from the maintained `pdfium-render` fork so that callers keep one
/// vocabulary for both sides of the contract.
pub use pdfium_render::prelude::ABOLISHPDFS_EXTENSION_VERSION as EXTENSION_ABI_VERSION;

#[derive(Debug, thiserror::Error)]
pub enum PdfiumBindError {
    #[error("could not bind PDFium: {0}")]
    Library(#[from] PdfiumError),
    #[error(
        "PDFium at {} does not provide the abolishpdfs glyph identity extension \
         (extension version {found}, expected {expected}); install the patched \
         runtime built by abolishpdfs-pdfium-build",
        .path.display()
    )]
    ExtensionIncompatible { path: PathBuf, found: u32, expected: u32 },
}

/// Binds the resolved runtime and verifies its extension ABI before use.
///
/// Stock PDFium binaries fail during binding because the extension symbols are
/// required binding-table entries; anything else must report exactly the ABI
/// version this build was compiled against.
pub fn bind_runtime(library: &PdfiumLibrary) -> Result<Pdfium, PdfiumBindError> {
    let bindings = Pdfium::bind_to_library(library.path()).map_err(PdfiumBindError::Library)?;
    // SAFETY: `FPDF_GetAbolishPdfsExtensionVersion` reads only library-global
    // state owned by the bound bindings object and performs no memory access.
    let found = unsafe { bindings.FPDF_GetAbolishPdfsExtensionVersion() };

    if found != EXTENSION_ABI_VERSION {
        return Err(PdfiumBindError::ExtensionIncompatible {
            path: library.path().to_path_buf(),
            found,
            expected: EXTENSION_ABI_VERSION,
        });
    }

    Ok(Pdfium::new(bindings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_abi_version_is_single_sourced_from_the_fork() {
        assert_eq!(EXTENSION_ABI_VERSION, 1);
    }

    #[test]
    fn extension_incompatible_error_names_the_runtime_expectation() {
        let error = PdfiumBindError::ExtensionIncompatible { path: "/tmp/libpdfium.so".into(), found: 0, expected: 1 };
        assert!(error.to_string().contains("glyph identity extension"));
        assert!(error.to_string().contains("expected 1"));
    }
}
