use std::path::{Path, PathBuf};

use fontnorm::config::Config;
use fontnorm::core::resolve::ResolveOptions;

const FIXTURES: &str = "tests/fixtures";

fn unique_tmp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("fontnorm_test_{tag}_{nanos}"))
}

/// Copy the fixture fonts into a fresh writable input dir.
fn setup_input(tag: &str) -> PathBuf {
    let dir = unique_tmp(tag);
    std::fs::create_dir_all(&dir).unwrap();
    for f in std::fs::read_dir(FIXTURES).unwrap() {
        let f = f.unwrap().path();
        if f.extension().and_then(|e| e.to_str()) == Some("ttf")
            || f.extension().and_then(|e| e.to_str()) == Some("otf")
        {
            let dest = dir.join(f.file_name().unwrap());
            std::fs::copy(&f, &dest).unwrap();
        }
    }
    dir
}

fn base_config(input: &Path, output: &Path) -> Config {
    Config {
        input_dir: input.to_path_buf(),
        output_dir: output.to_path_buf(),
        dry_run: false,
        recursive: false,
        rename: true,
        family_aware: true,
        resolve_options: ResolveOptions::default(),
        verbosity: 0,
    }
}

#[test]
fn run_writes_normalized_outputs_and_never_mutates_inputs() {
    let input = setup_input("write");
    let output = input.join("normalized");

    // Snapshot input bytes.
    let mut before: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for f in std::fs::read_dir(&input).unwrap() {
        let p = f.unwrap().path();
        if p.is_file() {
            before.push((p.clone(), std::fs::read(&p).unwrap()));
        }
    }

    let cfg = base_config(&input, &output);
    let report = fontnorm::run(&cfg).unwrap();

    assert!(report.failures.is_empty(), "no per-font failures expected");
    assert!(output.is_dir(), "output dir created");
    let out_count = std::fs::read_dir(&output).unwrap().count();
    assert_eq!(out_count, before.len(), "one output per input font");

    // Inputs must be byte-identical after the run.
    for (p, bytes) in &before {
        let now = std::fs::read(p).unwrap();
        assert_eq!(&now, bytes, "input mutated: {p:?}");
    }

    std::fs::remove_dir_all(&input).ok();
}

#[test]
fn dry_run_writes_nothing() {
    let input = setup_input("dry");
    let output = input.join("normalized");

    let mut cfg = base_config(&input, &output);
    cfg.dry_run = true;
    let report = fontnorm::run(&cfg).unwrap();

    assert!(report.dry_run);
    assert!(!output.exists(), "dry-run must not create the output dir");

    std::fs::remove_dir_all(&input).ok();
}

#[test]
fn no_rename_keeps_original_filenames() {
    let input = setup_input("norename");
    let output = input.join("normalized");

    let mut cfg = base_config(&input, &output);
    cfg.rename = false;
    let report = fontnorm::run(&cfg).unwrap();
    assert!(report.failures.is_empty());

    // The original filename must be present in the output.
    assert!(output.join("LiberationSans-Regular.ttf").exists());

    std::fs::remove_dir_all(&input).ok();
}

#[test]
fn second_run_on_output_reports_no_changes() {
    let input = setup_input("idem");
    let output = input.join("normalized");
    let output2 = input.join("normalized2");

    let cfg = base_config(&input, &output);
    fontnorm::run(&cfg).unwrap();

    let cfg2 = base_config(&output, &output2);
    let report2 = fontnorm::run(&cfg2).unwrap();

    assert_eq!(
        report2.normalized_count(),
        0,
        "second run must report zero normalizations"
    );
    assert!(report2.unchanged_count() > 0);

    // Byte-identical outputs.
    for f in std::fs::read_dir(&output).unwrap() {
        let p = f.unwrap().path();
        let name = p.file_name().unwrap();
        let a = std::fs::read(&p).unwrap();
        let b = std::fs::read(output2.join(name)).unwrap();
        assert_eq!(a, b, "output not stable across runs: {name:?}");
    }

    std::fs::remove_dir_all(&input).ok();
}
