use super::model::Weight;

/// Normalize a raw usWeightClass into a value suitable for `Weight::nearest`.
/// Handles the legacy CFF values: 250 -> Thin(100), 275 -> ExtraLight(200).
/// Returns (normalized_class, was_legacy).
pub fn normalize_legacy(class: u16) -> (u16, bool) {
    match class {
        250 => (100, true),
        275 => (200, true),
        other => (other, false),
    }
}

/// Resolve a raw usWeightClass to a canonical `Weight` on the standard ladder.
/// Reports whether a legacy value was normalized.
pub fn from_class(class: u16) -> (Weight, bool) {
    let (norm, legacy) = normalize_legacy(class);
    (Weight::nearest(norm), legacy)
}

/// Parse a weight word from a free-text token (already lowercased, no spaces/hyphens).
/// Returns the matched weight. Longest-match-first to disambiguate compound words
/// like "extrabold" vs "bold" or "semibold" vs "bold".
pub fn from_word(word: &str) -> Option<Weight> {
    let w = word.to_ascii_lowercase();
    // Ordered longest / most-specific first.
    const TABLE: &[(&str, Weight)] = &[
        ("extrablack", Weight::BLACK),
        ("ultrablack", Weight::BLACK),
        ("extrabold", Weight::EXTRA_BOLD),
        ("ultrabold", Weight::EXTRA_BOLD),
        ("semibold", Weight::SEMI_BOLD),
        ("demibold", Weight::SEMI_BOLD),
        ("extralight", Weight::EXTRA_LIGHT),
        ("ultralight", Weight::EXTRA_LIGHT),
        ("hairline", Weight::THIN),
        ("regular", Weight::REGULAR),
        ("normal", Weight::REGULAR),
        ("medium", Weight::MEDIUM),
        ("black", Weight::BLACK),
        ("heavy", Weight::BLACK),
        ("bold", Weight::BOLD),
        ("light", Weight::LIGHT),
        ("thin", Weight::THIN),
        ("book", Weight::REGULAR),
        ("demi", Weight::SEMI_BOLD),
    ];
    TABLE
        .iter()
        .find(|(k, _)| w == *k)
        .map(|(_, weight)| *weight)
}
