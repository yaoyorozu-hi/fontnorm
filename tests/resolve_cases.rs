mod common;
use common::Sig;

use fontnorm::core::diff::Field;
use fontnorm::core::model::{RibbiSlot, Weight, Width};
use fontnorm::core::names::canonical_names;
use fontnorm::core::resolve::{
    ResolveContext, panose_weight_needs_fix, resolve, target_fs_selection, target_mac_style,
};
use fontnorm::core::signals::{FS_BOLD, FS_ITALIC, FS_OBLIQUE, FS_REGULAR};

fn ctx() -> ResolveContext<'static> {
    ResolveContext::default()
}

// --- PANOSE bWeight correction policy: fix only meaningful disagreements ---
#[test]
fn panose_fix_policy() {
    // Kobo bug: Bold (target 8) with Book (5) crosses the bold boundary -> fix.
    assert!(panose_weight_needs_fix(5, 8));
    // DemiBold(7) on a 700 face -> crosses boundary -> fix.
    assert!(panose_weight_needs_fix(7, 8));
    // Unset (0) -> fix.
    assert!(panose_weight_needs_fix(0, 5));
    // Adjacent rung (Book 5 vs Medium 6), both non-bold -> leave alone.
    assert!(!panose_weight_needs_fix(6, 5));
    assert!(!panose_weight_needs_fix(5, 6));
    // Exact match -> no fix.
    assert!(!panose_weight_needs_fix(8, 8));
}

// --- The exact Kobo Yrsa-Bold bug: PANOSE bWeight=5 (Book) on a Bold face ---
#[test]
fn bold_with_book_panose_resolves_bold_and_flags_conflict() {
    let s = Sig::new()
        .family("Yrsa")
        .subfamily("Bold")
        .weight_class(700)
        .fs_bold()
        .panose_weight(5)
        .filename("Yrsa-Bold")
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.weight, Weight::BOLD);
    assert_eq!(r.style.ribbi_slot, RibbiSlot::Bold);
    assert!(
        r.conflicts.iter().any(|c| c.field == Field::PanoseWeight),
        "panose disagreement must be reported"
    );
    // The corrected panose weight is 8.
    assert_eq!(r.style.weight.panose_weight(), 8);
}

// --- I1: name says Italic, bits don't -> resolves italic, flags conflict ---
#[test]
fn name_says_italic_bits_dont_resolves_italic_and_flags_conflict() {
    let s = Sig::new()
        .family("Foo")
        .subfamily("Italic")
        .weight_class(400)
        .fs_raw(0) // no italic bit
        .mac_raw(0)
        .build();
    let r = resolve(&s, &ctx());
    assert!(r.style.italic);
    assert_eq!(r.style.ribbi_slot, RibbiSlot::Italic);
    let fs = target_fs_selection(&s, &r.style);
    assert_ne!(fs & FS_ITALIC, 0, "ITALIC bit must be set");
    let mac = target_mac_style(&s, &r.style);
    assert_ne!(mac & 0x0002, 0, "macStyle italic must be set");
    assert!(r.conflicts.iter().any(|c| c.field == Field::FsSelection));
}

// --- I1: fsSelection.ITALIC <-> macStyle.Italic always consistent on output ---
#[test]
fn italic_drives_both_fsselection_and_macstyle() {
    let s = Sig::new()
        .family("Foo")
        .subfamily("Italic")
        .weight_class(400)
        .build();
    let r = resolve(&s, &ctx());
    let fs = target_fs_selection(&s, &r.style);
    let mac = target_mac_style(&s, &r.style);
    assert_eq!((fs & FS_ITALIC != 0), (mac & 0x0002 != 0));
    assert!(fs & FS_ITALIC != 0);
}

// --- W1/W2: bold drives bit5 == macStyle0 == weight 700 ---
#[test]
fn bold_drives_weight_class_and_bits() {
    let s = Sig::new().family("Foo").subfamily("Bold").build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.weight, Weight::BOLD);
    let fs = target_fs_selection(&s, &r.style);
    let mac = target_mac_style(&s, &r.style);
    assert_ne!(fs & FS_BOLD, 0);
    assert_ne!(mac & 0x0001, 0);
}

// --- R1: REGULAR set => ITALIC and BOLD clear ---
#[test]
fn regular_slot_clears_bold_italic_sets_regular() {
    let s = Sig::new()
        .family("Foo")
        .subfamily("Regular")
        .weight_class(400)
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.ribbi_slot, RibbiSlot::Regular);
    let fs = target_fs_selection(&s, &r.style);
    assert_ne!(fs & FS_REGULAR, 0, "REGULAR bit set");
    assert_eq!(fs & FS_ITALIC, 0, "ITALIC clear");
    assert_eq!(fs & FS_BOLD, 0, "BOLD clear");
}

// --- R2: BoldItalic sets both bits, not regular ---
#[test]
fn bold_italic_sets_both_bits_not_regular() {
    let s = Sig::new().family("Foo").subfamily("Bold Italic").build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.ribbi_slot, RibbiSlot::BoldItalic);
    let fs = target_fs_selection(&s, &r.style);
    assert_ne!(fs & FS_BOLD, 0);
    assert_ne!(fs & FS_ITALIC, 0);
    assert_eq!(fs & FS_REGULAR, 0);
}

// --- W4: legacy usWeightClass 250 -> Thin(100), 275 -> ExtraLight(200) ---
#[test]
fn legacy_weightclass_250_normalizes_to_thin_100() {
    let s = Sig::new().family("Foo").weight_class(250).build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.weight, Weight::THIN);
    assert!(
        r.changes
            .iter()
            .any(|c| c.field == Field::UsWeightClass && c.before.contains("legacy"))
    );
}

#[test]
fn legacy_weightclass_275_normalizes_to_extralight_200() {
    let s = Sig::new().family("Foo").weight_class(275).build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.weight, Weight::EXTRA_LIGHT);
}

// --- W3: typographic name wins over a wrong weight class ---
#[test]
fn typo_name_semibold_wins_over_weightclass_400() {
    let s = Sig::new()
        .typo_family("Adobe Caslon Pro")
        .typo_subfamily("SemiBold")
        .weight_class(400)
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.weight, Weight::SEMI_BOLD);
    assert!(r.conflicts.iter().any(|c| c.field == Field::UsWeightClass));
}

// --- N1: SemiBold splits into its own ID1 family (Caslon example) ---
#[test]
fn semibold_splits_into_own_id1_family() {
    let s = Sig::new()
        .typo_family("Adobe Caslon Pro")
        .typo_subfamily("SemiBold")
        .weight_class(600)
        .build();
    let r = resolve(&s, &ctx());
    let names = canonical_names(&r.style);
    assert_eq!(names.family, "Adobe Caslon Pro SemiBold");
    assert_eq!(names.subfamily, "Regular");
    assert_eq!(names.typo_family.as_deref(), Some("Adobe Caslon Pro"));
    assert_eq!(names.typo_subfamily.as_deref(), Some("SemiBold"));
}

#[test]
fn semibold_italic_splits_with_italic_slot() {
    let s = Sig::new()
        .typo_family("Adobe Caslon Pro")
        .typo_subfamily("SemiBold Italic")
        .weight_class(600)
        .fs_italic()
        .build();
    let r = resolve(&s, &ctx());
    let names = canonical_names(&r.style);
    assert_eq!(names.family, "Adobe Caslon Pro SemiBold");
    assert_eq!(names.subfamily, "Italic");
    assert_eq!(names.typo_subfamily.as_deref(), Some("SemiBold Italic"));
}

// --- N2: ID4 drops trailing Regular ---
#[test]
fn id4_drops_trailing_regular() {
    let s = Sig::new()
        .family("Yrsa")
        .subfamily("Regular")
        .weight_class(400)
        .build();
    let r = resolve(&s, &ctx());
    let names = canonical_names(&r.style);
    assert_eq!(names.full, "Yrsa");
}

#[test]
fn id4_black_omits_regular_qualifier_via_split() {
    // "Arial Black" -> split family, ID4 = "Arial Black" (no trailing Regular)
    let s = Sig::new()
        .typo_family("Arial")
        .typo_subfamily("Black")
        .weight_class(900)
        .build();
    let r = resolve(&s, &ctx());
    let names = canonical_names(&r.style);
    assert_eq!(names.family, "Arial Black");
    assert_eq!(names.full, "Arial Black");
}

// --- N3: PostScript name sanitized, no spaces, no forbidden chars ---
#[test]
fn postscript_name_sanitized() {
    let s = Sig::new()
        .typo_family("My (Weird) Font/Name")
        .subfamily("Bold")
        .build();
    let r = resolve(&s, &ctx());
    let ps = &r.style.postscript_name;
    assert!(!ps.contains(' '));
    for c in ['(', ')', '/', '[', ']', '%', '<', '>', '{', '}'] {
        assert!(!ps.contains(c), "ps name must not contain {c}");
    }
    assert!(ps.len() <= 63);
}

// --- M1: measured monospace overrides flags ---
#[test]
fn measured_monospace_sets_fixedpitch_and_panose() {
    let s = Sig::new()
        .family("IBM Plex Mono")
        .weight_class(300)
        .monospace_measured(true, true)
        .fixed_pitch(0)
        .panose_proportion(2)
        .build();
    let r = resolve(&s, &ctx());
    assert!(r.style.monospace);
    assert!(r.changes.iter().any(|c| c.field == Field::PostIsFixedPitch));
    assert!(r.changes.iter().any(|c| c.field == Field::PanoseProportion));
    assert!(
        r.conflicts
            .iter()
            .any(|c| c.field == Field::PostIsFixedPitch)
    );
}

#[test]
fn measured_proportional_clears_false_monospace_flag() {
    let s = Sig::new()
        .family("Some Sans")
        .weight_class(400)
        .monospace_measured(true, false)
        .fixed_pitch(1)
        .build();
    let r = resolve(&s, &ctx());
    assert!(!r.style.monospace);
    assert!(r.changes.iter().any(|c| c.field == Field::PostIsFixedPitch));
}

#[test]
fn cjk_no_latin_does_not_force_monospace() {
    let s = Sig::new()
        .family("Noto CJK")
        .weight_class(400)
        .monospace_measured(false, true) // seems mono but no latin
        .fixed_pitch(0)
        .build();
    let r = resolve(&s, &ctx());
    assert!(
        !r.style.monospace,
        "CJK without latin must not be forced monospace"
    );
}

// --- Width handling ---
#[test]
fn condensed_width_from_class() {
    let s = Sig::new()
        .family("Helvetica")
        .width_class(3)
        .weight_class(400)
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.width, Width(3));
    let names = canonical_names(&r.style);
    // Condensed is non-RIBBI -> splits.
    assert_eq!(names.family, "Helvetica Condensed");
}

// --- Oblique handling ---
#[test]
fn oblique_name_sets_oblique_and_italic() {
    let s = Sig::new().family("Foo").subfamily("Oblique").build();
    let r = resolve(&s, &ctx());
    assert!(r.style.italic);
    assert!(r.style.oblique);
    let fs = target_fs_selection(&s, &r.style);
    assert_ne!(fs & FS_OBLIQUE, 0);
    assert_ne!(fs & FS_ITALIC, 0);
}

// --- USE_TYPO_METRICS preservation ---
#[test]
fn use_typo_metrics_preserved_when_set() {
    let s = Sig::new()
        .family("Foo")
        .weight_class(400)
        .fs_use_typo_metrics()
        .build();
    let r = resolve(&s, &ctx());
    assert!(r.style.use_typo_metrics);
    let fs = target_fs_selection(&s, &r.style);
    assert_ne!(fs & 0x0080, 0);
}

#[test]
fn use_typo_metrics_never_force_cleared() {
    // Original has it; we never clear it.
    let s = Sig::new().family("Foo").fs_use_typo_metrics().build();
    let r = resolve(&s, &ctx());
    let fs = target_fs_selection(&s, &r.style);
    assert_ne!(fs & 0x0080, 0);
}

// --- Reserved bit preservation ---
#[test]
fn reserved_fsselection_bits_preserved() {
    // bit 2 NEGATIVE (0x0004) is not managed; must survive.
    let s = Sig::new()
        .family("Foo")
        .subfamily("Bold")
        .fs_raw(0x0004 | FS_BOLD)
        .build();
    let r = resolve(&s, &ctx());
    let fs = target_fs_selection(&s, &r.style);
    assert_ne!(fs & 0x0004, 0, "reserved NEGATIVE bit preserved");
    assert_ne!(fs & FS_BOLD, 0);
}

// --- Idempotency: already-canonical input yields no changes ---
#[test]
fn idempotent_canonical_regular_yields_no_changes() {
    // Construct a fully-canonical Regular face.
    let s = Sig::new()
        .family("Yrsa")
        .subfamily("Regular")
        .full("Yrsa")
        .postscript("Yrsa-Regular")
        .weight_class(400)
        .width_class(5)
        .fs_raw(FS_REGULAR)
        .mac_raw(0)
        .panose([2, 0, 5, 0, 0, 0, 0, 0, 0, 0])
        .fixed_pitch(0)
        .filename("Yrsa-Regular")
        .build();
    let r = resolve(&s, &ctx());
    assert!(
        r.changes.is_empty(),
        "expected no changes, got: {:?}",
        r.changes
    );
}

#[test]
fn idempotent_canonical_bold_yields_no_changes() {
    let s = Sig::new()
        .family("Yrsa")
        .subfamily("Bold")
        .full("Yrsa Bold")
        .postscript("Yrsa-Bold")
        .weight_class(700)
        .width_class(5)
        .fs_raw(FS_BOLD)
        .mac_raw(0x0001)
        .panose([2, 0, 8, 0, 0, 0, 0, 0, 0, 0])
        .fixed_pitch(0)
        .filename("Yrsa-Bold")
        .build();
    let r = resolve(&s, &ctx());
    assert!(
        r.changes.is_empty(),
        "expected no changes, got: {:?}",
        r.changes
    );
}

// --- Missing name table: reconstruct from filename + weight class (lenient) ---
#[test]
fn missing_names_reconstruct_from_filename() {
    let s = Sig::new()
        .filename("Roboto-BoldItalic")
        .weight_class(700)
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Roboto");
    assert_eq!(r.style.weight, Weight::BOLD);
    assert!(r.style.italic);
}

// --- Default weight when nothing is present ---
#[test]
fn no_weight_signal_defaults_regular() {
    let s = Sig::new().family("Foo").build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.weight, Weight::REGULAR);
}

// --- H2: uncorroborated trailing weight word is NOT stripped (Archivo Black) ---
#[test]
fn archivo_black_keeps_family_when_weight_uncorroborated() {
    let s = Sig::new()
        .family("Archivo Black")
        .subfamily("Regular")
        .weight_class(400)
        .fs_regular()
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Archivo Black");
    let n = canonical_names(&r.style);
    assert_eq!(n.family, "Archivo Black");
}

// --- H2: corroborated trailing weight word IS stripped (Open Sans Bold) ---
#[test]
fn open_sans_bold_strips_corroborated_weight() {
    let s = Sig::new()
        .family("Open Sans Bold")
        .subfamily("Bold")
        .weight_class(700)
        .fs_bold()
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Open Sans");
    let n = canonical_names(&r.style);
    assert_eq!(n.family, "Open Sans");
    assert_eq!(n.subfamily, "Bold");
}

// --- H3: width word in family + wrong usWidthClass -> width recovered ---
#[test]
fn condensed_width_recovered_from_family_name() {
    let s = Sig::new()
        .typo_family("Gotham Condensed")
        .typo_subfamily("Bold")
        .family("Gotham Condensed Bold")
        .subfamily("Bold")
        .weight_class(700)
        .width_class(5) // wrong: says Normal
        .fs_bold()
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(
        r.style.width,
        Width(3),
        "width must be recovered to Condensed"
    );
}

// --- M1/M2: width word already in family is not double-counted ---
#[test]
fn condensed_family_does_not_double_count() {
    let s = Sig::new()
        .typo_family("Barlow Condensed")
        .typo_subfamily("ExtraLight")
        .family("Barlow Condensed ExtraLight")
        .subfamily("Regular")
        .weight_class(200)
        .width_class(3)
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Barlow");
    let n = canonical_names(&r.style);
    // No "Condensed Condensed" anywhere.
    assert!(!n.family.contains("Condensed Condensed"));
    assert_eq!(n.family, "Barlow Condensed ExtraLight");
}

// --- D1: width word in ID1 only, usWidthClass=5, no ID16/ID17 -> single, not doubled ---
#[test]
fn d1_width_in_family_only_not_duplicated() {
    // Gotham Condensed Bold: ID1 carries "Condensed", usWidthClass=5, no ID16/17.
    let s = Sig::new()
        .family("Gotham Condensed")
        .weight_class(700)
        .width_class(5)
        .fs_bold()
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Gotham");
    assert_eq!(r.style.width, Width(3));
    let n = canonical_names(&r.style);
    assert!(
        !n.family.contains("Condensed Condensed"),
        "no duplicated width word; got {:?}",
        n.family
    );
    assert_eq!(n.family, "Gotham Condensed Bold");
}

// D1: the chained case (Semi Condensed) must collapse to one width descriptor.
#[test]
fn d1_semicondensed_in_family_not_duplicated() {
    let s = Sig::new()
        .family("Fira Sans SemiCondensed")
        .weight_class(500)
        .width_class(5)
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Fira Sans");
    assert_eq!(r.style.width, Width(4));
    let n = canonical_names(&r.style);
    assert!(!n.family.to_lowercase().contains("condensed condensed"));
    assert!(!n.family.contains("SemiCondensed SemiCondensed"));
}

// D1/H2 boundary: "Archivo Black" with resolved weight Regular keeps "Black".
#[test]
fn d1_resolved_facet_corroboration_preserves_archivo_black() {
    let s = Sig::new()
        .family("Archivo Black")
        .subfamily("Regular")
        .weight_class(400)
        .fs_regular()
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Archivo Black");
}

// D1/H2 boundary: "Open Sans Bold" with resolved weight 700 strips "Bold".
#[test]
fn d1_resolved_facet_corroboration_strips_open_sans_bold() {
    let s = Sig::new()
        .family("Open Sans Bold")
        .subfamily("Bold")
        .weight_class(700)
        .fs_bold()
        .build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Open Sans");
}

// --- M3: stale panose bProportion=9 cleared on a proportional font ---
#[test]
fn stale_monospace_panose_cleared_when_proportional() {
    let s = Sig::new()
        .family("Some Sans")
        .weight_class(400)
        .monospace_measured(true, false) // measured proportional
        .fixed_pitch(0)
        .panose_proportion(9) // stale monospace marker
        .build();
    let r = resolve(&s, &ctx());
    assert!(!r.style.monospace);
    assert!(
        r.changes
            .iter()
            .any(|c| c.field == Field::PanoseProportion && c.before == "9" && c.after == "4"),
        "stale panose proportion 9 must be reset to 4; changes: {:?}",
        r.changes
    );
}

// --- M4: empty family name falls back to Unknown ---
#[test]
fn empty_family_falls_back_to_unknown() {
    let s = Sig::new().family("").weight_class(400).build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Unknown");
}

#[test]
fn whitespace_family_falls_back_to_unknown() {
    let s = Sig::new().family("   ").weight_class(400).build();
    let r = resolve(&s, &ctx());
    assert_eq!(r.style.typographic_family, "Unknown");
}

// --- oblique implies italic style-linking (research I2) ---
#[test]
fn oblique_implies_italic_slot() {
    let s = Sig::new()
        .family("Foo")
        .subfamily("Oblique")
        .fs_oblique()
        .build();
    let r = resolve(&s, &ctx());
    assert!(r.style.italic);
    assert!(r.style.oblique);
    assert_eq!(r.style.ribbi_slot, RibbiSlot::Italic);
}
