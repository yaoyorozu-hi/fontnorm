use std::path::PathBuf;

use crate::core::resolve::ResolveOptions;

/// Validated, I/O-free settings flowing through the pipeline. No clap types here.
#[derive(Clone, Debug)]
pub struct Config {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub dry_run: bool,
    pub recursive: bool,
    pub rename: bool,
    pub family_aware: bool,
    pub resolve_options: ResolveOptions,
    pub verbosity: u8,
}
