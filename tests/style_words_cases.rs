use fontnorm::core::model::{Weight, Width};
use fontnorm::core::style_words::{
    Corroborated, parse, strip_corroborated_style_words, strip_style_words,
};

#[test]
fn corroborated_strip_keeps_uncorroborated_words() {
    // "Black" with no corroborated weight is kept.
    let corr = Corroborated::default();
    assert_eq!(
        strip_corroborated_style_words("Archivo Black", &corr),
        "Archivo Black"
    );
}

#[test]
fn corroborated_strip_removes_matching_weight() {
    let corr = Corroborated {
        weight: Some(Weight::BOLD),
        width: None,
        slope: false,
    };
    assert_eq!(
        strip_corroborated_style_words("Open Sans Bold", &corr),
        "Open Sans"
    );
}

#[test]
fn corroborated_strip_removes_matching_width() {
    let corr = Corroborated {
        weight: None,
        width: Some(Width(3)),
        slope: false,
    };
    assert_eq!(
        strip_corroborated_style_words("Barlow Condensed", &corr),
        "Barlow"
    );
}

#[test]
fn corroborated_strip_keeps_wrong_weight() {
    // Family says "Black" but corroborated weight is Bold -> keep (mismatch).
    let corr = Corroborated {
        weight: Some(Weight::BOLD),
        width: None,
        slope: false,
    };
    assert_eq!(
        strip_corroborated_style_words("Archivo Black", &corr),
        "Archivo Black"
    );
}

#[test]
fn semibold_italic_glued() {
    let p = parse("SemiBoldItalic");
    assert_eq!(p.weight, Some(Weight::SEMI_BOLD));
    assert!(p.italic);
    assert!(!p.oblique);
}

#[test]
fn cond_bold() {
    let p = parse("Cond Bold");
    assert_eq!(p.weight, Some(Weight::BOLD));
    assert_eq!(p.width, Some(Width(3)));
}

#[test]
fn black_oblique() {
    let p = parse("BlackOblique");
    assert_eq!(p.weight, Some(Weight::BLACK));
    assert!(p.oblique);
    assert!(p.italic);
}

#[test]
fn extra_light() {
    let p = parse("ExtraLight");
    assert_eq!(p.weight, Some(Weight::EXTRA_LIGHT));
}

#[test]
fn ultra_condensed_extrabold() {
    let p = parse("UltraCondensed ExtraBold");
    assert_eq!(p.width, Some(Width(1)));
    assert_eq!(p.weight, Some(Weight::EXTRA_BOLD));
}

#[test]
fn family_word_text_is_not_expanded() {
    // "Text" ends in "ext" but must not be parsed as Expanded width.
    let p = parse("Libre Caslon Text");
    assert_eq!(p.width, None);
    assert_eq!(p.weight, None);
}

#[test]
fn family_word_edit_is_not_italic() {
    let p = parse("Edit Sans");
    assert!(!p.italic);
}

#[test]
fn plain_italic() {
    let p = parse("Italic");
    assert!(p.italic);
    assert_eq!(p.weight, None);
}

#[test]
fn strip_recovers_family() {
    assert_eq!(strip_style_words("Yrsa Bold"), "Yrsa");
    assert_eq!(
        strip_style_words("Adobe Caslon Pro SemiBold Italic"),
        "Adobe Caslon Pro"
    );
    assert_eq!(strip_style_words("Roboto-BoldItalic"), "Roboto");
    assert_eq!(strip_style_words("Helvetica Condensed"), "Helvetica");
}

#[test]
fn strip_keeps_family_when_no_style() {
    assert_eq!(strip_style_words("Libre Caslon Text"), "Libre Caslon Text");
}

#[test]
fn strip_handles_all_style_words() {
    // Degenerate: everything is a style word -> fall back to original.
    let r = strip_style_words("Bold Italic");
    assert!(!r.is_empty());
}
