pub mod batch;
pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod font_io;
pub mod report;

use config::Config;
use error::FatalError;
use report::RunReport;

/// Run the full normalization pipeline for a validated config.
pub fn run(cfg: &Config) -> Result<RunReport, FatalError> {
    batch::run(cfg)
}
