use fontnorm::core::model::{ResolvedStyle, RibbiSlot, Weight, Width};
use fontnorm::core::names::{canonical_names, compose_subfamily, sanitize_postscript};

fn make(
    typo_family: &str,
    typo_subfamily: &str,
    ribbi_family: &str,
    slot: RibbiSlot,
    weight: Weight,
    width: Width,
) -> ResolvedStyle {
    ResolvedStyle {
        typographic_family: typo_family.to_string(),
        typographic_subfamily: typo_subfamily.to_string(),
        ribbi_family: ribbi_family.to_string(),
        ribbi_slot: slot,
        weight,
        width,
        italic: slot.is_italic(),
        oblique: false,
        monospace: false,
        monospace_authoritative: true,
        postscript_name: format!(
            "{}-{}",
            typo_family.replace(' ', ""),
            typo_subfamily.replace(' ', "")
        ),
        use_typo_metrics: false,
        dup_suffix: None,
    }
}

// N1: RIBBI face omits ID16/17.
#[test]
fn ribbi_regular_omits_typo_ids() {
    let s = make(
        "Yrsa",
        "Regular",
        "Yrsa",
        RibbiSlot::Regular,
        Weight::REGULAR,
        Width::NORMAL,
    );
    let n = canonical_names(&s);
    assert_eq!(n.family, "Yrsa");
    assert_eq!(n.subfamily, "Regular");
    assert!(n.typo_family.is_none());
    assert!(n.typo_subfamily.is_none());
}

#[test]
fn ribbi_bold_italic_omits_typo_ids() {
    let s = make(
        "Yrsa",
        "Bold Italic",
        "Yrsa",
        RibbiSlot::BoldItalic,
        Weight::BOLD,
        Width::NORMAL,
    );
    let n = canonical_names(&s);
    assert_eq!(n.subfamily, "Bold Italic");
    assert!(n.typo_family.is_none());
}

// N1: non-RIBBI face emits ID16/17 (Caslon SemiBold).
#[test]
fn semibold_emits_typo_ids() {
    let s = make(
        "Adobe Caslon Pro",
        "SemiBold",
        "Adobe Caslon Pro SemiBold",
        RibbiSlot::Regular,
        Weight::SEMI_BOLD,
        Width::NORMAL,
    );
    let n = canonical_names(&s);
    assert_eq!(n.family, "Adobe Caslon Pro SemiBold");
    assert_eq!(n.subfamily, "Regular");
    assert_eq!(n.typo_family.as_deref(), Some("Adobe Caslon Pro"));
    assert_eq!(n.typo_subfamily.as_deref(), Some("SemiBold"));
    assert_eq!(n.full, "Adobe Caslon Pro SemiBold");
}

// N2: ID4 drops trailing Regular.
#[test]
fn full_name_drops_trailing_regular() {
    let s = make(
        "Yrsa",
        "Regular",
        "Yrsa",
        RibbiSlot::Regular,
        Weight::REGULAR,
        Width::NORMAL,
    );
    assert_eq!(canonical_names(&s).full, "Yrsa");
}

// compose_subfamily covers the facet -> string rules.
#[test]
fn compose_subfamily_cases() {
    assert_eq!(
        compose_subfamily(Weight::REGULAR, Width::NORMAL, false, false),
        "Regular"
    );
    assert_eq!(
        compose_subfamily(Weight::BOLD, Width::NORMAL, false, false),
        "Bold"
    );
    assert_eq!(
        compose_subfamily(Weight::SEMI_BOLD, Width::NORMAL, true, false),
        "SemiBold Italic"
    );
    assert_eq!(
        compose_subfamily(Weight::REGULAR, Width(3), false, false),
        "Condensed"
    );
    assert_eq!(
        compose_subfamily(Weight::BLACK, Width::NORMAL, false, false),
        "Black"
    );
    assert_eq!(
        compose_subfamily(Weight::REGULAR, Width::NORMAL, true, true),
        "Oblique"
    );
}

#[test]
fn sanitize_postscript_strips_forbidden() {
    assert_eq!(sanitize_postscript("Foo (Bar)/Baz-Bold"), "FooBarBaz-Bold");
    let long: String = "A".repeat(100);
    assert_eq!(sanitize_postscript(&long).len(), 63);
}
