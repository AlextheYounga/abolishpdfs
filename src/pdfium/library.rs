use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfiumLibrary {
    path: PathBuf,
}

impl PdfiumLibrary {
    pub fn resolve(
        explicit_path: Option<&Path>,
        env_path: Option<&Path>,
        current_exe: &Path,
    ) -> Result<Self, PdfiumLibraryError> {
        let candidates = candidate_paths(explicit_path, env_path, current_exe);
        if let Some(path) = candidates.iter().find(|path| path.is_file()) {
            return Ok(Self { path: path.clone() });
        }

        Err(PdfiumLibraryError {
            expected_name: library_name().to_owned(),
            checked_paths: candidates,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn candidate_paths(
    explicit_path: Option<&Path>,
    env_path: Option<&Path>,
    current_exe: &Path,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = explicit_path.or(env_path) {
        candidates.push(path.to_owned());
    }
    if let Some(parent) = current_exe.parent() {
        candidates.push(parent.join(library_name()));
    }
    candidates
}

fn library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PdfiumLibraryError {
    expected_name: String,
    checked_paths: Vec<PathBuf>,
}

impl fmt::Display for PdfiumLibraryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "could not find {} in:", self.expected_name)?;
        for path in &self.checked_paths {
            write!(formatter, "\n  {}", path.display())?;
        }
        Ok(())
    }
}

impl std::error::Error for PdfiumLibraryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn explicit_library_path_takes_precedence() {
        let directory = tempdir().unwrap();
        let explicit = directory.path().join("custom-pdfium.so");
        std::fs::write(&explicit, []).unwrap();

        let resolved = PdfiumLibrary::resolve(
            Some(&explicit),
            Some(Path::new("/missing/from-env")),
            Path::new("/missing/bin/abolishpdfs"),
        )
        .unwrap();

        assert_eq!(resolved.path(), explicit);
    }

    #[test]
    fn environment_path_is_used_when_explicit_path_is_absent() {
        let directory = tempdir().unwrap();
        let environment_path = directory.path().join("pdfium.so");
        std::fs::write(&environment_path, []).unwrap();

        let resolved = PdfiumLibrary::resolve(
            None,
            Some(&environment_path),
            Path::new("/missing/bin/abolishpdfs"),
        )
        .unwrap();

        assert_eq!(resolved.path(), environment_path);
    }

    #[test]
    fn sibling_library_is_used_as_the_default() {
        let directory = tempdir().unwrap();
        let executable = directory.path().join("abolishpdfs");
        let sibling = directory.path().join(library_name());
        std::fs::write(&sibling, []).unwrap();

        let resolved = PdfiumLibrary::resolve(None, None, &executable).unwrap();

        assert_eq!(resolved.path(), sibling);
    }

    #[test]
    fn missing_library_reports_all_checked_paths() {
        let error = PdfiumLibrary::resolve(
            Some(Path::new("/missing/explicit")),
            None,
            Path::new("/missing/bin/abolishpdfs"),
        )
        .unwrap_err();

        assert_eq!(error.checked_paths.len(), 2);
        assert!(error.to_string().contains("/missing/explicit"));
        assert!(error.to_string().contains("libpdfium.so"));
    }
}
