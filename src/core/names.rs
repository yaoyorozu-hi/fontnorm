use super::model::ResolvedStyle;

/// The canonical name strings computed from a ResolvedStyle.
/// ID16/17 are Some only when a typographic split is needed (N1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CanonicalNames {
    pub family: String,                 // ID1
    pub subfamily: String,              // ID2
    pub full: String,                   // ID4
    pub postscript: String,             // ID6
    pub typo_family: Option<String>,    // ID16
    pub typo_subfamily: Option<String>, // ID17
}

/// Sanitize a PostScript name per name ID6 rules: ASCII printable 33..=126 minus
/// the forbidden set `[](){}<>/%`, no spaces, <= 63 chars. The hyphen separator is
/// supplied by the caller; this preserves it.
pub fn sanitize_postscript(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let c = ch as u32;
        let forbidden = matches!(
            ch,
            '[' | ']' | '(' | ')' | '{' | '}' | '<' | '>' | '/' | '%'
        );
        if (33..=126).contains(&c) && !forbidden && ch != ' ' {
            out.push(ch);
        }
    }
    out.chars().take(63).collect()
}

fn no_spaces(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Drop a trailing " Regular" qualifier (ID4 rule N2).
fn drop_trailing_regular(full: &str) -> String {
    full.strip_suffix(" Regular")
        .map(str::to_string)
        .unwrap_or_else(|| full.to_string())
}

pub fn canonical_names(style: &ResolvedStyle) -> CanonicalNames {
    let family = style.ribbi_family.clone();
    let subfamily = style.ribbi_slot.subfamily_name().to_string();

    // ID16/17 emitted iff the face is split (ribbi_family differs from typographic_family)
    // or its typographic_subfamily is not a plain RIBBI word.
    let split = style.ribbi_family != style.typographic_family
        || !is_ribbi_word(&style.typographic_subfamily);

    let (typo_family, typo_subfamily) = if split {
        (
            Some(style.typographic_family.clone()),
            Some(style.typographic_subfamily.clone()),
        )
    } else {
        (None, None)
    };

    // ID4: prefer typographic family+subfamily when split, else RIBBI family+subfamily.
    let full = if split {
        drop_trailing_regular(&format!(
            "{} {}",
            style.typographic_family, style.typographic_subfamily
        ))
    } else {
        drop_trailing_regular(&format!("{family} {subfamily}"))
    };

    let postscript = if style.postscript_name.is_empty() {
        let raw = format!(
            "{}-{}",
            no_spaces(&style.typographic_family),
            no_spaces(&style.typographic_subfamily)
        );
        sanitize_postscript(&raw)
    } else {
        sanitize_postscript(&style.postscript_name)
    };

    CanonicalNames {
        family,
        subfamily,
        full,
        postscript,
        typo_family,
        typo_subfamily,
    }
}

fn is_ribbi_word(s: &str) -> bool {
    matches!(s, "Regular" | "Bold" | "Italic" | "Bold Italic")
}

/// Compose the canonical typographic subfamily string from facets:
/// [Width][Weight][Italic], with Normal width and (for upright Regular) collapsing
/// to "Regular". Spaces separate the human-readable form.
pub fn compose_subfamily(
    weight: super::model::Weight,
    width: super::model::Width,
    italic: bool,
    oblique: bool,
) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(wtok) = width.token() {
        parts.push(wtok);
    }
    if !weight.is_regular() {
        parts.push(weight.name());
    }
    let slope = if oblique {
        Some("Oblique")
    } else if italic {
        Some("Italic")
    } else {
        None
    };
    if let Some(s) = slope {
        parts.push(s);
    }
    if parts.is_empty() {
        "Regular".to_string()
    } else {
        parts.join(" ")
    }
}
