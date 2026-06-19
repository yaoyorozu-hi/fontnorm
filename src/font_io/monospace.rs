use std::collections::HashMap;

use read_fonts::{FontRef, TableProvider};

use crate::core::signals::MonospaceMeasurement;

/// Number of ASCII letters (A-Z + a-z).
const LATIN_LETTERS: u32 = 52;

/// Count how many codepoints in an inclusive range the cmap maps to a glyph.
fn coverage(cmap: &read_fonts::tables::cmap::Cmap, range: std::ops::RangeInclusive<u32>) -> u32 {
    range.filter(|cp| cmap.map_codepoint(*cp).is_some()).count() as u32
}

/// Measure advance widths to decide monospace, per fontbakery `glyph_metrics_stats`.
///
/// Monospace is only classified for **Latin-primary** fonts. A CJK/Kana/Hangul font whose
/// full-width Latin glyphs all share one advance must NOT be flagged monospace
/// (fontbakery #2202). We therefore (a) require the Latin alphabet present, (b) reject when
/// the font carries substantial CJK/Kana/Hangul coverage relative to its Latin coverage,
/// and (c) measure the width statistic over Latin letters specifically.
pub fn measure(font: &FontRef) -> MonospaceMeasurement {
    let (Ok(cmap), Ok(hmtx)) = (font.cmap(), font.hmtx()) else {
        return MonospaceMeasurement::default();
    };

    // Latin letters present (A-Z, a-z) and their advances.
    let mut advances: Vec<u16> = Vec::new();
    let mut latin_present = 0u32;
    for cp in (b'A' as u32..=b'Z' as u32).chain(b'a' as u32..=b'z' as u32) {
        if let Some(gid) = cmap.map_codepoint(cp) {
            latin_present += 1;
            if let Some(adv) = hmtx.advance(gid)
                && adv != 0
            {
                advances.push(adv);
            }
        }
    }

    // Require >80% of the Latin alphabet to even consider Latin-primary.
    let has_latin = (latin_present as f64) > 0.8 * (LATIN_LETTERS as f64);
    if !has_latin || advances.is_empty() {
        return MonospaceMeasurement {
            has_latin,
            seems_monospaced: false,
        };
    }

    // Reject CJK/Kana/Hangul-primary fonts: if non-Latin script coverage rivals or exceeds
    // the Latin coverage, the font is not Latin-primary and monospace does not apply.
    let cjk = coverage(&cmap, 0x4E00..=0x9FFF) // CJK Unified Ideographs
        + coverage(&cmap, 0x3040..=0x30FF) // Hiragana + Katakana
        + coverage(&cmap, 0xAC00..=0xD7AF); // Hangul Syllables
    if cjk > latin_present {
        return MonospaceMeasurement {
            has_latin: false,
            seems_monospaced: false,
        };
    }

    let mut counts: HashMap<u16, usize> = HashMap::new();
    for a in &advances {
        *counts.entry(*a).or_insert(0) += 1;
    }
    let most_common_count = counts.values().copied().max().unwrap_or(0);

    let seems_monospaced = (most_common_count as f64) >= (advances.len() as f64) * 0.8;

    MonospaceMeasurement {
        has_latin,
        seems_monospaced,
    }
}
