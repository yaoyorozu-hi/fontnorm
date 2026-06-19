use fontnorm::core::filename::{canonical_filename, canonical_stem, style_token};
use fontnorm::core::model::{ResolvedStyle, RibbiSlot, Weight, Width};

fn style(family: &str, weight: Weight, width: Width, italic: bool, oblique: bool) -> ResolvedStyle {
    let slot = RibbiSlot::from_bools(weight == Weight::BOLD, italic);
    ResolvedStyle {
        typographic_family: family.to_string(),
        typographic_subfamily: "x".to_string(),
        ribbi_family: family.to_string(),
        ribbi_slot: slot,
        weight,
        width,
        italic,
        oblique,
        monospace: false,
        monospace_authoritative: true,
        postscript_name: String::new(),
        use_typo_metrics: false,
        dup_suffix: None,
    }
}

#[test]
fn regular_upright() {
    let s = style("Yrsa", Weight::REGULAR, Width::NORMAL, false, false);
    assert_eq!(canonical_filename(&s, "ttf"), "Yrsa-Regular.ttf");
}

#[test]
fn bold_upright() {
    let s = style("Yrsa", Weight::BOLD, Width::NORMAL, false, false);
    assert_eq!(canonical_filename(&s, "ttf"), "Yrsa-Bold.ttf");
}

#[test]
fn bold_italic() {
    let s = style("Yrsa", Weight::BOLD, Width::NORMAL, true, false);
    assert_eq!(canonical_filename(&s, "ttf"), "Yrsa-BoldItalic.ttf");
}

#[test]
fn semibold_italic_otf() {
    let s = style(
        "Adobe Caslon Pro",
        Weight::SEMI_BOLD,
        Width::NORMAL,
        true,
        false,
    );
    assert_eq!(
        canonical_filename(&s, "OTF"),
        "AdobeCaslonPro-SemiBoldItalic.otf"
    );
}

#[test]
fn light_mono() {
    let mut s = style("IBM Plex Mono", Weight::LIGHT, Width::NORMAL, false, false);
    s.monospace = true;
    assert_eq!(canonical_filename(&s, "ttf"), "IBMPlexMono-Light.ttf");
}

#[test]
fn condensed_regular() {
    let s = style("Helvetica", Weight::REGULAR, Width(3), false, false);
    assert_eq!(canonical_stem(&s), "Helvetica-CondensedRegular");
}

#[test]
fn oblique_token() {
    let s = style("DejaVu Sans", Weight::REGULAR, Width::NORMAL, true, true);
    assert_eq!(style_token(&s), "Oblique");
}

#[test]
fn extension_lowercased() {
    let s = style("Foo", Weight::REGULAR, Width::NORMAL, false, false);
    assert_eq!(canonical_filename(&s, "TTF"), "Foo-Regular.ttf");
}
