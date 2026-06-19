// Regression tests for the audit findings (倶生 / 須佐).
mod common;
use common::Sig;

use std::path::{Path, PathBuf};

use read_fonts::types::{NameId, Tag};
use read_fonts::{FontRef, TableProvider};

use fontnorm::config::Config;
use fontnorm::core::resolve::ResolveOptions;
use fontnorm::core::resolve::{ResolveContext, resolve};
use fontnorm::font_io::{read, write};
use fontnorm::report::FontStatus;

const FIXTURES: &str = "tests/fixtures";

fn unique_tmp(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("fontnorm_def_{tag}_{nanos}"))
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

// C1: a non-ASCII family with Mac records must NOT panic; it normalizes successfully and
// the produced Mac identity records (if any) are all MacRoman-encodable.
#[test]
fn c1_non_ascii_mac_records_do_not_panic() {
    let path = Path::new(FIXTURES).join("NonAsciiMac-Regular.ttf");
    let data = std::fs::read(&path).unwrap();
    let font = FontRef::new(&data).unwrap();
    let sig = read::read_signals(&font, &path).unwrap();
    assert!(sig.has_mac_name_records, "fixture must ship Mac records");

    let res = resolve(&sig, &ResolveContext::default());
    // This call panicked before the fix.
    let out = write::apply(&font, &sig, &res.style, &path).unwrap();

    let font2 = FontRef::new(&out).unwrap();
    let name = font2.name().unwrap();
    // The Windows record carries the non-ASCII identity.
    let win_family = name
        .name_record()
        .iter()
        .find(|r| r.name_id() == NameId::FAMILY_NAME && r.platform_id() == 3)
        .map(|r| r.string(name.string_data()).unwrap().to_string());
    assert!(win_family.is_some_and(|f| !f.is_ascii()));

    // Any emitted Mac (1/0/0) record must be ASCII (MacRoman subset for these glyphs).
    for r in name.name_record() {
        if r.platform_id() == 1 && r.encoding_id() == 0 {
            let s = r.string(name.string_data()).unwrap().to_string();
            assert!(
                s.is_ascii() || s.chars().all(|c| (c as u32) < 0x100),
                "Mac record string must be MacRoman-range: {s:?}"
            );
        }
    }
}

// C1 at the batch level: the non-ASCII font must not abort the batch (other fonts survive).
#[test]
fn c1_batch_isolation_non_ascii_does_not_kill_batch() {
    let input = unique_tmp("c1batch");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("NonAsciiMac-Regular.ttf"),
        input.join("NonAsciiMac-Regular.ttf"),
    )
    .unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("LiberationSans-Regular.ttf"),
        input.join("LiberationSans-Regular.ttf"),
    )
    .unwrap();
    let output = input.join("out");

    let report = fontnorm::run(&base_config(&input, &output)).unwrap();
    assert!(
        report.failures.is_empty(),
        "batch must not fail: {:?}",
        report.failures
    );
    assert_eq!(report.successes.len(), 2, "both fonts processed");

    std::fs::remove_dir_all(&input).ok();
}

// L2: a variable font is copied through unchanged and reported as skipped, never mislabeled.
#[test]
fn l2_variable_font_passthrough() {
    // Find any variable font on the system (fvar present).
    let var_src = find_variable_font();
    let Some(var_src) = var_src else {
        // No variable font available; the unit guard below still covers the signal.
        return;
    };

    let input = unique_tmp("l2");
    std::fs::create_dir_all(&input).unwrap();
    let dest = input.join("Variable.ttf");
    std::fs::copy(&var_src, &dest).unwrap();
    let original = std::fs::read(&dest).unwrap();
    let output = input.join("out");

    let report = fontnorm::run(&base_config(&input, &output)).unwrap();
    assert_eq!(report.skipped_variable_count(), 1);
    assert!(
        report
            .successes
            .iter()
            .any(|r| r.status == FontStatus::SkippedVariable)
    );
    // Copied through byte-identical, original name preserved.
    let out_file = output.join("Variable.ttf");
    assert!(out_file.exists());
    assert_eq!(std::fs::read(&out_file).unwrap(), original);

    std::fs::remove_dir_all(&input).ok();
}

fn find_variable_font() -> Option<PathBuf> {
    let out = std::process::Command::new("fc-list").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let p = line.split(':').next().unwrap_or("").trim();
        if !p.ends_with(".ttf") {
            continue;
        }
        let Ok(data) = std::fs::read(p) else { continue };
        let Ok(font) = FontRef::new(&data) else {
            continue;
        };
        if font.data_for_tag(Tag::new(b"fvar")).is_some() {
            return Some(PathBuf::from(p));
        }
    }
    None
}

// M5: two identical Regular faces in one family get distinct PostScript names (unique ID6).
#[test]
fn m5_dup_slot_gets_distinct_postscript_names() {
    use fontnorm::core::family::{FamilyGrouping, FamilyMember};
    use fontnorm::core::model::{Weight, Width};

    let m0 = FamilyMember {
        file_index: 0,
        sort_key: "Yrsa-Regular".into(),
        weight: Weight::REGULAR,
        italic: false,
        width: Width::NORMAL,
    };
    let m1 = FamilyMember {
        file_index: 1,
        sort_key: "Yrsa-Regular-other".into(),
        weight: Weight::REGULAR,
        italic: false,
        width: Width::NORMAL,
    };
    let grouping = FamilyGrouping::new("Yrsa".into(), vec![m0, m1]);

    let sig = Sig::new()
        .family("Yrsa")
        .subfamily("Regular")
        .weight_class(400)
        .build();

    let r0 = resolve(
        &sig,
        &ResolveContext {
            family: Some(&grouping),
            file_index: Some(0),
            ..Default::default()
        },
    );
    let r1 = resolve(
        &sig,
        &ResolveContext {
            family: Some(&grouping),
            file_index: Some(1),
            ..Default::default()
        },
    );
    assert_ne!(
        r0.style.postscript_name, r1.style.postscript_name,
        "colliding faces must get distinct PostScript names"
    );
    assert!(
        r1.conflicts
            .iter()
            .any(|c| c.field == fontnorm::core::diff::Field::NamePostscriptId6),
        "the collision must be reported"
    );
}

// W2: a pre-existing foreign file in the output dir is not silently overwritten.
#[test]
fn w2_does_not_overwrite_foreign_output_file() {
    let input = unique_tmp("w2");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("LiberationSans-Regular.ttf"),
        input.join("LiberationSans-Regular.ttf"),
    )
    .unwrap();
    let output = input.join("out");
    std::fs::create_dir_all(&output).unwrap();

    // Plant a foreign file at the exact name the tool will produce.
    let target = output.join("LiberationSans-Regular.ttf");
    let foreign = b"NOT A FONT - DO NOT CLOBBER".to_vec();
    std::fs::write(&target, &foreign).unwrap();

    let report = fontnorm::run(&base_config(&input, &output)).unwrap();
    assert!(report.failures.is_empty());

    // The foreign file must be intact.
    assert_eq!(
        std::fs::read(&target).unwrap(),
        foreign,
        "foreign output file was clobbered"
    );
    // The normalized font landed under a dup name instead.
    let dup = output.join("LiberationSans-Regular-dup2.ttf");
    assert!(
        dup.exists(),
        "normalized font should be written to a dup name"
    );

    std::fs::remove_dir_all(&input).ok();
}

/// Run the normalizer on a directory and return the sorted (filename, bytes) of its output.
fn run_collect(input: &Path, output: &Path) -> Vec<(String, Vec<u8>)> {
    fontnorm::run(&base_config(input, output)).unwrap();
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(output)
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            p.is_file().then(|| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&p).unwrap(),
                )
            })
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

// D2: two colliding faces normalize to a STABLE disambiguation that is idempotent across
// runs — the second run produces byte-identical output (no oscillating -2 suffix).
#[test]
fn d2_dup_collision_is_idempotent() {
    let input = unique_tmp("d2idem");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("DupFaceA.ttf"),
        input.join("DupFaceA.ttf"),
    )
    .unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("DupFaceB.ttf"),
        input.join("DupFaceB.ttf"),
    )
    .unwrap();

    let n1 = input.join("n1");
    let n2 = input.join("n2");
    let first = run_collect(&input, &n1);
    let report2 = fontnorm::run(&base_config(&n1, &n2)).unwrap();
    let second = run_collect(&n1, &n2);

    assert_eq!(
        report2.normalized_count(),
        0,
        "second run must report zero normalizations (idempotent)"
    );
    assert_eq!(
        first.len(),
        2,
        "two distinct files must be produced (no overwrite)"
    );
    assert_eq!(
        first, second,
        "output filenames and bytes must be stable across runs (no -2 oscillation)"
    );

    std::fs::remove_dir_all(&input).ok();
}

// D2: the disambiguation does not depend on input filename order.
#[test]
fn d2_disambiguation_independent_of_input_filename_order() {
    let dir_a = unique_tmp("d2orderA");
    let dir_b = unique_tmp("d2orderB");
    std::fs::create_dir_all(&dir_a).unwrap();
    std::fs::create_dir_all(&dir_b).unwrap();

    std::fs::copy(
        Path::new(FIXTURES).join("DupFaceA.ttf"),
        dir_a.join("aaa.ttf"),
    )
    .unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("DupFaceB.ttf"),
        dir_a.join("bbb.ttf"),
    )
    .unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("DupFaceA.ttf"),
        dir_b.join("zzz.ttf"),
    )
    .unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("DupFaceB.ttf"),
        dir_b.join("yyy.ttf"),
    )
    .unwrap();

    let out_a = run_collect(&dir_a, &dir_a.join("out"));
    let out_b = run_collect(&dir_b, &dir_b.join("out"));

    assert_eq!(
        out_a, out_b,
        "disambiguation must be intrinsic to the font, not the input filename order"
    );

    std::fs::remove_dir_all(&dir_a).ok();
    std::fs::remove_dir_all(&dir_b).ok();
}
