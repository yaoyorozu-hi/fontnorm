use std::path::PathBuf;

/// Fatal errors abort the whole run (exit 1). Setup problems only.
#[derive(thiserror::Error, Debug)]
pub enum FatalError {
    #[error("input directory not found or not a directory: {0}")]
    BadInputDir(PathBuf),
    #[error("cannot create/write output directory {0}: {1}")]
    OutputDir(PathBuf, std::io::Error),
    #[error("output directory would overlap input files: {0}")]
    OutputOverlapsInput(PathBuf),
    #[error("failed to scan input directory {0}: {1}")]
    Scan(PathBuf, std::io::Error),
}

/// Per-font errors: skip the font, collect, continue the batch.
#[derive(thiserror::Error, Debug)]
pub enum FontError {
    #[error("unsupported container (woff/woff2/ttc) in v1: {0}")]
    UnsupportedContainer(PathBuf),
    #[error("failed to read file {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("failed to parse font tables: {0}")]
    Parse(PathBuf, String),
    #[error("missing required table {table} in {path}")]
    MissingTable { path: PathBuf, table: &'static str },
    #[error("write/round-trip failed for {0}: {1}")]
    Write(PathBuf, String),
}

impl FontError {
    pub fn path(&self) -> &std::path::Path {
        match self {
            FontError::UnsupportedContainer(p)
            | FontError::Io(p, _)
            | FontError::Parse(p, _)
            | FontError::Write(p, _) => p,
            FontError::MissingTable { path, .. } => path,
        }
    }
}
