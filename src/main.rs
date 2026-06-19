use std::process::ExitCode;

use clap::Parser;

use fontnorm::cli::Args;

fn main() -> ExitCode {
    let args = Args::parse();

    let level = match args.verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).init();

    let cfg = match args.into_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    match fontnorm::run(&cfg) {
        Ok(report) => {
            print!("{}", report.render(cfg.verbosity));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
