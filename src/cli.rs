use std::path::PathBuf;

use clap::Parser;

use crate::config::Config;
use crate::core::resolve::ResolveOptions;
use crate::error::FatalError;

/// Normalize and correct font metadata for .ttf and .otf files.
#[derive(Parser, Debug)]
#[command(name = "fontnorm", version, about, long_about = None)]
pub struct Args {
    /// Directory containing fonts (non-recursive by default).
    pub input_dir: PathBuf,

    /// Output subfolder name, created under INPUT_DIR.
    #[arg(short, long, default_value = "normalized")]
    pub output: String,

    /// Explicit output directory (overrides --output; may be outside input).
    #[arg(long)]
    pub out_dir: Option<PathBuf>,

    /// Resolve and report changes, write nothing.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Recurse into subdirectories of INPUT_DIR.
    #[arg(short, long)]
    pub recursive: bool,

    /// Keep original filenames; fix embedded metadata only.
    #[arg(long)]
    pub no_rename: bool,

    /// Disable whole-family analysis; per-file resolution only.
    #[arg(long)]
    pub no_family: bool,

    /// Skip monospace measurement and correction.
    #[arg(long)]
    pub no_monospace: bool,

    /// Force fsSelection USE_TYPO_METRICS (bit 7) on. Never clears it.
    #[arg(long)]
    pub use_typo_metrics: bool,

    /// Increase log verbosity (-v, -vv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Errors only.
    #[arg(short, long)]
    pub quiet: bool,
}

impl Args {
    /// Map parsed args into a validated, I/O-free Config. clap types stop here.
    pub fn into_config(self) -> Result<Config, FatalError> {
        if !self.input_dir.is_dir() {
            return Err(FatalError::BadInputDir(self.input_dir));
        }

        let output_dir = match self.out_dir {
            Some(p) => p,
            None => self.input_dir.join(&self.output),
        };

        let verbosity = if self.quiet { 0 } else { self.verbose };

        Ok(Config {
            input_dir: self.input_dir,
            output_dir,
            dry_run: self.dry_run,
            recursive: self.recursive,
            rename: !self.no_rename,
            family_aware: !self.no_family,
            resolve_options: ResolveOptions {
                monospace_enabled: !self.no_monospace,
                force_use_typo_metrics: self.use_typo_metrics,
            },
            verbosity,
        })
    }
}
