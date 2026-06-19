use super::model::{Weight, Width};
use super::weight;

/// Style facets parsed out of a descriptor string (name ID2/ID17 or a filename stem).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ParsedStyle {
    pub weight: Option<Weight>,
    pub width: Option<Width>,
    pub italic: bool,
    pub oblique: bool,
}

/// All recognized style tokens, longest-first, used both to classify a token and to
/// strip style words from a family string. Each maps to its facet effect.
#[derive(Clone, Copy)]
enum Token {
    Weight(Weight),
    Width(Width),
    Italic,
    Oblique,
}

/// Recognized style words, longest-first so compound words match before their substrings.
const STYLE_TOKENS: &[(&str, Token)] = &[
    // widths (compound first)
    ("ultracondensed", Token::Width(Width(1))),
    ("extracondensed", Token::Width(Width(2))),
    ("semicondensed", Token::Width(Width(4))),
    ("ultraexpanded", Token::Width(Width(9))),
    ("extraexpanded", Token::Width(Width(8))),
    ("semiexpanded", Token::Width(Width(6))),
    ("condensed", Token::Width(Width(3))),
    ("expanded", Token::Width(Width(7))),
    ("narrow", Token::Width(Width(3))),
    ("cond", Token::Width(Width(3))),
    ("cnd", Token::Width(Width(3))),
    ("ext", Token::Width(Width(7))),
    // weights (compound first)
    ("extrablack", Token::Weight(Weight::BLACK)),
    ("ultrablack", Token::Weight(Weight::BLACK)),
    ("extrabold", Token::Weight(Weight::EXTRA_BOLD)),
    ("ultrabold", Token::Weight(Weight::EXTRA_BOLD)),
    ("semibold", Token::Weight(Weight::SEMI_BOLD)),
    ("demibold", Token::Weight(Weight::SEMI_BOLD)),
    ("extralight", Token::Weight(Weight::EXTRA_LIGHT)),
    ("ultralight", Token::Weight(Weight::EXTRA_LIGHT)),
    ("hairline", Token::Weight(Weight::THIN)),
    ("regular", Token::Weight(Weight::REGULAR)),
    ("normal", Token::Weight(Weight::REGULAR)),
    ("medium", Token::Weight(Weight::MEDIUM)),
    ("heavy", Token::Weight(Weight::BLACK)),
    ("black", Token::Weight(Weight::BLACK)),
    ("light", Token::Weight(Weight::LIGHT)),
    ("book", Token::Weight(Weight::REGULAR)),
    ("demi", Token::Weight(Weight::SEMI_BOLD)),
    ("bold", Token::Weight(Weight::BOLD)),
    ("thin", Token::Weight(Weight::THIN)),
    // slope
    ("oblique", Token::Oblique),
    ("italic", Token::Italic),
    ("ital", Token::Italic),
    ("obl", Token::Oblique),
    ("it", Token::Italic),
];

/// Greedily consume style tokens from the front of a lowercase chunk, returning the
/// facet effects found and any leading non-style remainder.
/// e.g. "semibolditalic" -> [SemiBold, Italic], remainder "".
///      "caslonbold"     -> remainder "caslon", [Bold]  (only trailing style words strip)
fn parse_chunk(chunk: &str, acc: &mut ParsedStyle) -> bool {
    // Try to match the whole chunk as a sequence of style tokens from the end inward.
    // Strategy: repeatedly strip a recognized token from the END of the chunk.
    let mut rest = chunk;
    let mut matched_any = false;
    'outer: loop {
        for (word, tok) in STYLE_TOKENS {
            if let Some(prefix) = rest.strip_suffix(word) {
                apply_token(*tok, acc);
                matched_any = true;
                rest = prefix;
                if rest.is_empty() {
                    break 'outer;
                }
                continue 'outer;
            }
        }
        break;
    }
    rest.is_empty() && matched_any
}

fn apply_token(tok: Token, acc: &mut ParsedStyle) {
    match tok {
        Token::Weight(w) => {
            if acc.weight.is_none() {
                acc.weight = Some(w);
            }
        }
        Token::Width(w) => {
            if acc.width.is_none() {
                acc.width = Some(w);
            }
        }
        Token::Italic => acc.italic = true,
        Token::Oblique => {
            acc.oblique = true;
            acc.italic = true;
        }
    }
}

/// Parse style facets from a descriptor (ID2/ID17 or filename stem).
///
/// Tokens are split on separators only (space/hyphen/underscore). Each token is then
/// decomposed by greedy suffix-stripping against the longest-first STYLE_TOKENS table,
/// which correctly handles separator-less compounds ("SemiBoldItalic" -> SemiBold +
/// Italic) without falsely splitting "SemiBold" into "Semi"+"Bold". Non-style leading
/// remainder (the family portion of a filename like "RobotoBold") is ignored, but any
/// trailing style facets it carries are still captured.
pub fn parse(descriptor: &str) -> ParsedStyle {
    let mut acc = ParsedStyle::default();
    for token in descriptor.split([' ', '-', '_']) {
        if token.is_empty() {
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if let Some(w) = weight::from_word(&lower) {
            if acc.weight.is_none() {
                acc.weight = Some(w);
            }
            continue;
        }
        // Only capture facets when the separator-token decomposes ENTIRELY into style
        // words. This prevents family tokens like "Text" (ends in "ext"=Expanded) or
        // "Edit" (ends in "it"=Italic) from injecting phantom facets.
        let mut local = ParsedStyle::default();
        if parse_chunk(&lower, &mut local) {
            merge(&mut acc, &local);
        }
    }
    acc
}

fn merge(acc: &mut ParsedStyle, other: &ParsedStyle) {
    if acc.weight.is_none() {
        acc.weight = other.weight;
    }
    if acc.width.is_none() {
        acc.width = other.width;
    }
    acc.italic |= other.italic;
    acc.oblique |= other.oblique;
}

/// Which style facets carry independent metadata corroboration and may therefore be
/// stripped from a family-name string as redundant descriptors. A facet is only listed
/// here when other metadata (usWeightClass, usWidthClass, style bits, or ID2/ID17)
/// confirms it — this is the positive-evidence rule that prevents destroying real family
/// names like "Archivo Black" (whose "Black" is uncorroborated when usWeightClass=400).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Corroborated {
    /// The resolved weight, present iff weight is corroborated and non-Regular.
    pub weight: Option<Weight>,
    /// The resolved width, present iff width is corroborated and non-Normal.
    pub width: Option<Width>,
    /// Italic/oblique corroborated.
    pub slope: bool,
}

/// Strip trailing style words from a family string, but ONLY words whose facet is
/// corroborated by `corr`. Tokens are removed from the trailing end while each is a pure
/// style word describing a corroborated facet. Uncorroborated style-looking words
/// (e.g. "Black" on a 400-weight face) are kept — they are part of the real family name.
pub fn strip_corroborated_style_words(s: &str, corr: &Corroborated) -> String {
    let sep_split: Vec<&str> = s.split([' ', '-', '_']).filter(|t| !t.is_empty()).collect();

    let mut keep_end = sep_split.len();
    while keep_end > 0 {
        let lower = sep_split[keep_end - 1].to_ascii_lowercase();
        if token_is_corroborated_style(&lower, corr) {
            keep_end -= 1;
        } else {
            break;
        }
    }

    let kept = &sep_split[..keep_end];
    if kept.is_empty() {
        // Everything looked like (corroborated) style words; keep the original so we
        // never produce an empty family.
        return s.trim().to_string();
    }
    kept.join(" ").trim().to_string()
}

/// True iff a trailing token is a pure style word AND every facet it describes is
/// corroborated. A weight word must match the corroborated weight; a width word the
/// corroborated width; a slope word requires corroborated slope.
fn token_is_corroborated_style(token: &str, corr: &Corroborated) -> bool {
    if token.is_empty() {
        return false;
    }
    let mut facets = ParsedStyle::default();
    let pure = if let Some(w) = weight::from_word(token) {
        facets.weight = Some(w);
        true
    } else {
        parse_chunk(token, &mut facets)
    };
    if !pure {
        return false;
    }
    if let Some(w) = facets.weight
        && corr.weight != Some(w)
    {
        return false;
    }
    if let Some(w) = facets.width
        && corr.width != Some(w)
    {
        return false;
    }
    if (facets.italic || facets.oblique) && !corr.slope {
        return false;
    }
    true
}

/// Strip ALL trailing pure style words unconditionally. Used only for family-grouping
/// key derivation, where over-stripping merely groups more aggressively (a clustering
/// hint, not an identity decision). NEVER used to write a family name.
pub fn strip_style_words(s: &str) -> String {
    let sep_split: Vec<&str> = s.split([' ', '-', '_']).filter(|t| !t.is_empty()).collect();
    let mut keep_end = sep_split.len();
    while keep_end > 0 {
        let lower = sep_split[keep_end - 1].to_ascii_lowercase();
        if is_pure_style(&lower) {
            keep_end -= 1;
        } else {
            break;
        }
    }
    let kept = &sep_split[..keep_end];
    if kept.is_empty() {
        return s.trim().to_string();
    }
    kept.join(" ").trim().to_string()
}

/// True iff the whole lowercase token decomposes entirely into recognized style words.
fn is_pure_style(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    if weight::from_word(token).is_some() {
        return true;
    }
    let mut acc = ParsedStyle::default();
    parse_chunk(token, &mut acc)
}
