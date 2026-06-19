use std::path::Path;

use read_fonts::types::{NameId, Tag};
use read_fonts::{FontRef, TableProvider};

use fontnorm::core::resolve::{ResolveContext, resolve};
use fontnorm::font_io::{read, write};

const FIXTURES: &str = "tests/fixtures";

fn outline_bytes(font: &FontRef) -> Vec<u8> {
    let mut out = Vec::new();
    for tag in [b"glyf", b"loca", b"CFF "] {
        if let Some(d) = font.data_for_tag(Tag::new(tag)) {
            out.extend_from_slice(d.as_bytes());
        }
    }
    out
}

fn name_string(font: &FontRef, id: NameId) -> Option<String> {
    let name = font.name().ok()?;
    let data = name.string_data();
    name.name_record()
        .iter()
        .find(|r| r.name_id() == id && r.platform_id() == 3)
        .and_then(|r| r.string(data).ok())
        .map(|s| s.to_string())
}

/// Resolve + apply a single font file, returning the produced bytes.
fn normalize_bytes(path: &Path) -> Vec<u8> {
    let data = std::fs::read(path).unwrap();
    let font = FontRef::new(&data).unwrap();
    let sig = read::read_signals(&font, path).unwrap();
    let res = resolve(&sig, &ResolveContext::default());
    write::apply(&font, &sig, &res.style, path).unwrap()
}

#[test]
fn ttf_roundtrip_preserves_outlines() {
    let path = Path::new(FIXTURES).join("LiberationSans-Regular.ttf");
    let orig = std::fs::read(&path).unwrap();
    let font = FontRef::new(&orig).unwrap();
    let orig_outlines = outline_bytes(&font);

    let out = normalize_bytes(&path);
    let font2 = FontRef::new(&out).unwrap();
    assert_eq!(
        orig_outlines,
        outline_bytes(&font2),
        "glyf/loca must be byte-identical"
    );
    // Output re-parses.
    font2.head().unwrap();
    font2.os2().unwrap();
    font2.post().unwrap();
    font2.name().unwrap();
}

#[test]
fn otf_cff_roundtrip_preserves_outlines() {
    let path = Path::new(FIXTURES).join("Edmondsans-Regular.otf");
    let orig = std::fs::read(&path).unwrap();
    let font = FontRef::new(&orig).unwrap();
    let orig_outlines = outline_bytes(&font);
    assert!(!orig_outlines.is_empty(), "fixture must have CFF outlines");

    let out = normalize_bytes(&path);
    let font2 = FontRef::new(&out).unwrap();
    assert_eq!(
        orig_outlines,
        outline_bytes(&font2),
        "CFF must be byte-identical (never re-serialized)"
    );
}

#[test]
fn name_change_applies() {
    // DejaVu Sans Mono should get monospace marking; we assert the family survives
    // and the subfamily becomes canonical.
    let path = Path::new(FIXTURES).join("LiberationSans-Bold.ttf");
    let out = normalize_bytes(&path);
    let font2 = FontRef::new(&out).unwrap();
    assert_eq!(
        name_string(&font2, NameId::SUBFAMILY_NAME).as_deref(),
        Some("Bold")
    );
    let os2 = font2.os2().unwrap();
    assert_eq!(os2.us_weight_class(), 700);
    assert_ne!(os2.fs_selection().bits() & 0x0020, 0, "BOLD bit set");
}

#[test]
fn monospace_detected_and_marked() {
    let path = Path::new(FIXTURES).join("DejaVuSansMono.ttf");
    let data = std::fs::read(&path).unwrap();
    let font = FontRef::new(&data).unwrap();
    let sig = read::read_signals(&font, &path).unwrap();
    assert!(
        sig.monospace_measure.has_latin,
        "DejaVu Sans Mono has Latin"
    );
    assert!(
        sig.monospace_measure.seems_monospaced,
        "DejaVu Sans Mono must measure as monospaced"
    );

    let res = resolve(&sig, &ResolveContext::default());
    assert!(res.style.monospace);

    let out = write::apply(&font, &sig, &res.style, &path).unwrap();
    let font2 = FontRef::new(&out).unwrap();
    assert_eq!(font2.post().unwrap().is_fixed_pitch(), 1);
    assert_eq!(
        font2.os2().unwrap().panose_10()[3],
        9,
        "panose bProportion=9"
    );
}

#[test]
fn proportional_not_marked_monospace() {
    let path = Path::new(FIXTURES).join("DejaVuSans.ttf");
    let data = std::fs::read(&path).unwrap();
    let font = FontRef::new(&data).unwrap();
    let sig = read::read_signals(&font, &path).unwrap();
    assert!(!sig.monospace_measure.seems_monospaced);
    let res = resolve(&sig, &ResolveContext::default());
    assert!(!res.style.monospace);
}

// H1: a CJK font with full-width Latin (all 52 letters share one advance) must be gated
// OUT of monospace classification, so the measurement never drives an isFixedPitch change.
// (The font's own isFixedPitch flag is preserved either way.)
#[test]
fn cjk_fullwidth_latin_gated_out_of_monospace() {
    let path = Path::new(FIXTURES).join("BIZUDMincho-Regular.ttf");
    let data = std::fs::read(&path).unwrap();
    let font = FontRef::new(&data).unwrap();
    let sig = read::read_signals(&font, &path).unwrap();
    assert!(
        !sig.monospace_measure.has_latin,
        "CJK-primary font must be gated out of monospace classification (has_latin=false)"
    );
    assert!(
        !sig.monospace_measure.seems_monospaced,
        "CJK font must not measure as monospaced"
    );

    let res = resolve(&sig, &ResolveContext::default());
    // The tool preserves the font's own isFixedPitch; it never CHANGES it from the bogus
    // full-width-Latin measurement.
    let original_fixed = font.post().unwrap().is_fixed_pitch();
    let out = write::apply(&font, &sig, &res.style, &path).unwrap();
    let font2 = FontRef::new(&out).unwrap();
    assert_eq!(
        font2.post().unwrap().is_fixed_pitch(),
        original_fixed,
        "CJK font's isFixedPitch must be preserved, not driven by measurement"
    );
}

#[test]
fn idempotency_second_run_is_byte_identical_and_no_changes() {
    for fixture in [
        "LiberationSans-Regular.ttf",
        "LiberationSans-Bold.ttf",
        "LiberationSans-Italic.ttf",
        "LiberationSans-BoldItalic.ttf",
        "Edmondsans-Regular.otf",
        "DejaVuSansMono.ttf",
    ] {
        let path = Path::new(FIXTURES).join(fixture);

        // First pass.
        let first = normalize_bytes(&path);

        // Write to a temp file so the second pass reads from disk with the canonical name.
        let tmp = std::env::temp_dir().join(format!("fontnorm_idem_{fixture}"));
        std::fs::write(&tmp, &first).unwrap();

        // Second pass on the output.
        let font = FontRef::new(&first).unwrap();
        let sig = read::read_signals(&font, &tmp).unwrap();
        let res = resolve(&sig, &ResolveContext::default());
        assert!(
            res.changes.is_empty(),
            "{fixture}: second run reported changes: {:?}",
            res.changes
        );
        let second = write::apply(&font, &sig, &res.style, &tmp).unwrap();
        assert_eq!(first, second, "{fixture}: second run not byte-identical");

        std::fs::remove_file(&tmp).ok();
    }
}

#[test]
fn dsig_dropped_on_modify() {
    // If any fixture carries DSIG, ensure it is dropped when changes are applied.
    for fixture in ["LiberationSans-Regular.ttf", "Edmondsans-Regular.otf"] {
        let path = Path::new(FIXTURES).join(fixture);
        let data = std::fs::read(&path).unwrap();
        let font = FontRef::new(&data).unwrap();
        if font.data_for_tag(Tag::new(b"DSIG")).is_none() {
            continue;
        }
        let sig = read::read_signals(&font, &path).unwrap();
        let res = resolve(&sig, &ResolveContext::default());
        let out = write::apply(&font, &sig, &res.style, &path).unwrap();
        let font2 = FontRef::new(&out).unwrap();
        assert!(
            font2.data_for_tag(Tag::new(b"DSIG")).is_none(),
            "{fixture}: DSIG must be dropped"
        );
    }
}
