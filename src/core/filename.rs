use super::model::ResolvedStyle;

/// Characters illegal in filenames on common platforms, plus whitespace, plus the
/// bracket/percent set the PostScript-name sanitizer forbids (L3, kept consistent).
fn sanitize_filename_part(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(
                    c,
                    '/' | '\\'
                        | ':'
                        | '*'
                        | '?'
                        | '"'
                        | '<'
                        | '>'
                        | '|'
                        | '\0'
                        | '['
                        | ']'
                        | '('
                        | ')'
                        | '{'
                        | '}'
                        | '%'
                )
        })
        .collect()
}

/// The style token used in the canonical filename: [Width][Weight][Italic], with
/// "Regular" always present for the plain face (unlike ID4).
pub fn style_token(style: &ResolvedStyle) -> String {
    let mut s = String::new();
    if let Some(wtok) = style.width.token() {
        s.push_str(wtok);
    }
    let slope_only = style.italic || style.oblique;
    // The weight token is always emitted, except when the face is a plain-Regular
    // upright with no width qualifier (then the bare "Regular" below covers it) or
    // a plain-Regular italic (then "Italic" alone reads cleanly, e.g. Yrsa-Italic).
    let emit_weight = !style.weight.is_regular() || (style.width.token().is_some() && !slope_only);
    if emit_weight {
        s.push_str(style.weight.token());
    }
    if style.oblique {
        s.push_str("Oblique");
    } else if style.italic {
        s.push_str("Italic");
    }
    if s.is_empty() {
        s.push_str("Regular");
    }
    s
}

/// Canonical output filename stem (without extension) derived purely from ResolvedStyle.
/// A dup-slot discriminator (N4) is appended so colliding faces get distinct, STABLE
/// filenames — the same physical font always produces the same name across runs.
pub fn canonical_stem(style: &ResolvedStyle) -> String {
    let family = sanitize_filename_part(&style.typographic_family);
    let token = style_token(style);
    let suffix = style
        .dup_suffix
        .map(|n| format!("-{n}"))
        .unwrap_or_default();
    format!("{family}-{token}{suffix}")
}

/// Full canonical filename with the original extension (lowercased, ".ttf"/".otf").
pub fn canonical_filename(style: &ResolvedStyle, ext: &str) -> String {
    format!("{}.{}", canonical_stem(style), ext.to_ascii_lowercase())
}
