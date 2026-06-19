use super::diff::{Conflict, Field, FieldChange};
use super::family::{FamilyGrouping, FamilyMember};
use super::model::{ResolvedStyle, RibbiSlot, Weight, Width};
use super::names::{self, canonical_names};
use super::signals::RawSignals;
use super::style_words::{self, ParsedStyle};
use super::weight;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SignalSource {
    TypographicName,
    WeightClass,
    StyleBits,
    Filename,
    Panose,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResolutionOrder(pub Vec<SignalSource>);

impl Default for ResolutionOrder {
    fn default() -> Self {
        ResolutionOrder(vec![
            SignalSource::TypographicName,
            SignalSource::WeightClass,
            SignalSource::StyleBits,
            SignalSource::Filename,
            SignalSource::Panose,
        ])
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResolveOptions {
    pub monospace_enabled: bool,
    /// Force USE_TYPO_METRICS on (never clears it).
    pub force_use_typo_metrics: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        ResolveOptions {
            monospace_enabled: true,
            force_use_typo_metrics: false,
        }
    }
}

#[derive(Default)]
pub struct ResolveContext<'a> {
    pub family: Option<&'a FamilyGrouping>,
    /// This font's index in the batch (matches `FamilyMember.file_index`), used to find
    /// its member within `family` for dup-slot disambiguation. `None` in per-file mode.
    pub file_index: Option<usize>,
    pub prefer: ResolutionOrder,
    pub options: ResolveOptions,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Resolution {
    pub style: ResolvedStyle,
    pub changes: Vec<FieldChange>,
    pub conflicts: Vec<Conflict>,
}

/// Parse style facets from the highest-priority name descriptor available.
fn parse_name_style(sig: &RawSignals) -> ParsedStyle {
    if let Some(s) = &sig.name_typo_subfamily_id17 {
        return style_words::parse(s);
    }
    if let Some(s) = &sig.name_subfamily_id2 {
        return style_words::parse(s);
    }
    ParsedStyle::default()
}

fn resolve_weight(
    sig: &RawSignals,
    ctx: &ResolveContext,
    conflicts: &mut Vec<Conflict>,
) -> (Weight, bool) {
    let name_style = parse_name_style(sig);
    let file_style = style_words::parse(&sig.filename_stem);
    let mut legacy_normalized = false;
    let mut chosen: Option<Weight> = None;

    for src in &ctx.prefer.0 {
        let cand = match src {
            SignalSource::TypographicName => name_style.weight,
            SignalSource::WeightClass => sig.us_weight_class.map(|c| {
                let (w, legacy) = weight::from_class(c);
                if legacy {
                    legacy_normalized = true;
                }
                w
            }),
            SignalSource::StyleBits => {
                if sig.fs_bold() || sig.mac_bold() {
                    Some(Weight::BOLD)
                } else {
                    None
                }
            }
            SignalSource::Filename => file_style.weight,
            SignalSource::Panose => None, // corroborating only
        };
        if let Some(c) = cand {
            match chosen {
                None => chosen = Some(c),
                Some(prev) if prev != c => {
                    conflicts.push(Conflict::new(
                        Field::UsWeightClass,
                        format!("{src:?} disagrees: {} vs chosen {}", c.name(), prev.name()),
                    ));
                }
                _ => {}
            }
        }
    }

    // PANOSE corroboration (never decisive): report disagreement only.
    if let (Some(w), Some(pw)) = (chosen, sig.panose_weight())
        && pw != 0
        && pw != w.panose_weight()
    {
        conflicts.push(Conflict::new(
            Field::PanoseWeight,
            format!(
                "panose.bWeight={} corroborates differently than resolved {}",
                pw,
                w.name()
            ),
        ));
    }

    (chosen.unwrap_or(Weight::REGULAR), legacy_normalized)
}

fn resolve_italic(
    sig: &RawSignals,
    ctx: &ResolveContext,
    conflicts: &mut Vec<Conflict>,
) -> (bool, bool) {
    let name_style = parse_name_style(sig);
    let file_style = style_words::parse(&sig.filename_stem);

    let mut italic: Option<bool> = None;
    let oblique = name_style.oblique || file_style.oblique || sig.fs_oblique();

    for src in &ctx.prefer.0 {
        let cand = match src {
            SignalSource::TypographicName => Some(name_style.italic),
            SignalSource::StyleBits => Some(sig.fs_italic() || sig.mac_italic()),
            SignalSource::Filename => Some(file_style.italic),
            _ => None,
        };
        if let Some(c) = cand {
            if italic.is_none() {
                italic = Some(c);
            } else if italic != Some(c) {
                conflicts.push(Conflict::new(
                    Field::FsSelection,
                    format!(
                        "italic disagreement from {src:?}: {c} vs chosen {}",
                        italic.unwrap()
                    ),
                ));
                // Any positive outranking a negative wins true.
                if c {
                    italic = Some(true);
                }
            }
        }
    }

    // OBLIQUE implies italic style-linking (research I2): the face slants, so it occupies
    // the Italic RIBBI slot.
    let italic = italic.unwrap_or(false) || oblique;
    (italic, oblique)
}

fn resolve_width(sig: &RawSignals, ctx: &ResolveContext) -> Width {
    let name_style = parse_name_style(sig);
    let file_style = style_words::parse(&sig.filename_stem);
    // A width word may live only in the family string (ID16/ID1), e.g. "Barlow Condensed"
    // or "GothamCondensed", with usWidthClass left at 5. Parse those too so width survives.
    let family_width = sig
        .name_typo_family_id16
        .as_deref()
        .or(sig.name_family_id1.as_deref())
        .and_then(|f| style_words::parse(f).width);

    for src in &ctx.prefer.0 {
        let cand = match src {
            SignalSource::TypographicName => name_style.width.or(family_width),
            SignalSource::WeightClass => sig
                .us_width_class
                .filter(|c| *c != 5)
                .map(Width::from_class),
            SignalSource::Filename => file_style.width,
            _ => None,
        };
        if let Some(c) = cand {
            return c;
        }
    }
    // Fall back to an explicit usWidthClass even if it is 5 (Normal).
    sig.us_width_class
        .map(Width::from_class)
        .unwrap_or(Width::NORMAL)
}

/// Resolve monospace. Returns `(monospace, authoritative)`. `authoritative` is true only
/// when the advance-width measurement applies (Latin-primary font); otherwise the value
/// merely echoes the existing flag and the writer must NOT use it to change isFixedPitch
/// or panose.bProportion (this prevents marking CJK fonts monospace, H1).
fn resolve_monospace(
    sig: &RawSignals,
    ctx: &ResolveContext,
    conflicts: &mut Vec<Conflict>,
) -> (bool, bool) {
    if !ctx.options.monospace_enabled {
        return (sig.is_fixed_pitch.is_some_and(|v| v != 0), false);
    }
    let m = &sig.monospace_measure;
    if m.has_latin {
        let measured = m.seems_monospaced;
        let flag = sig.is_fixed_pitch.is_some_and(|v| v != 0);
        let panose_mono = sig.panose_proportion() == Some(9);
        if measured != flag {
            conflicts.push(Conflict::new(
                Field::PostIsFixedPitch,
                format!("measured monospace={measured} but post.isFixedPitch flag={flag}"),
            ));
        }
        if measured != panose_mono && sig.panose_proportion().is_some() {
            conflicts.push(Conflict::new(
                Field::PanoseProportion,
                format!("measured monospace={measured} but panose.bProportion mono={panose_mono}"),
            ));
        }
        (measured, true)
    } else {
        // No Latin to measure (CJK etc.): preserve the existing state, do not decide.
        (sig.is_fixed_pitch.is_some_and(|v| v != 0), false)
    }
}

/// Treat empty / whitespace-only strings as absent so the fallback chain fires (M4).
fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|t| !t.is_empty())
}

/// Resolve the canonical typographic family.
///
/// ID16 and ID1 are run through the SAME corroboration-aware strip so the result
/// converges (M1/M2): only style words whose facet is independently corroborated are
/// removed. Family names like "Archivo Black" (uncorroborated weight) are preserved.
fn resolve_family(
    sig: &RawSignals,
    ctx: &ResolveContext,
    corr: &style_words::Corroborated,
) -> String {
    if let Some(g) = ctx.family {
        return g.typographic_family.clone();
    }
    let base = non_empty(&sig.name_typo_family_id16)
        .map(|f| style_words::strip_corroborated_style_words(f, corr))
        .or_else(|| {
            non_empty(&sig.name_family_id1)
                .map(|f| style_words::strip_corroborated_style_words(f, corr))
        })
        .or_else(|| {
            // Last resort: no name table family at all. The filename is a descriptor by
            // convention, so its trailing style words are safe to strip unconditionally.
            let stem = style_words::strip_style_words(&sig.filename_stem);
            if stem.trim().is_empty() {
                None
            } else {
                Some(stem)
            }
        })
        .filter(|b| !b.trim().is_empty())
        .unwrap_or_else(|| "Unknown".to_string());
    collapse_ws(&base)
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Determine which trailing style words in a family name are redundant style descriptors
/// and so may be stripped. A word is corroborated iff it matches a RESOLVED facet — the
/// facet the resolver already concluded from the totality of signals (D1 + H2).
///
/// This is the positive-evidence rule expressed against the resolution output rather than
/// a hand-picked subset of raw signals:
/// - "Archivo Black", usWeightClass=400 -> resolves to weight=Regular, so corr.weight=None;
///   the family word "Black" (=900) doesn't match -> KEEP "Archivo Black".
/// - "Open Sans Bold", usWeightClass=700 -> resolves to weight=Bold -> "Bold" matches -> STRIP.
/// - "Gotham Condensed", width recovered to Condensed from the family name -> "Condensed"
///   matches the resolved width -> STRIP exactly once (it is re-added by the split family /
///   subfamily token and preserved in usWidthClass, so no information is lost).
fn compute_corroboration(weight: Weight, width: Width, italic: bool) -> style_words::Corroborated {
    style_words::Corroborated {
        weight: (!weight.is_regular()).then_some(weight),
        width: (!width.is_normal()).then_some(width),
        slope: italic,
    }
}

pub fn resolve(sig: &RawSignals, ctx: &ResolveContext) -> Resolution {
    let mut conflicts = Vec::new();

    let (weight, legacy_weight) = resolve_weight(sig, ctx, &mut conflicts);
    let (italic, oblique) = resolve_italic(sig, ctx, &mut conflicts);
    let width = resolve_width(sig, ctx);
    let (monospace, monospace_authoritative) = resolve_monospace(sig, ctx, &mut conflicts);
    let corr = compute_corroboration(weight, width, italic);
    let typographic_family = resolve_family(sig, ctx, &corr);

    let typographic_subfamily = names::compose_subfamily(weight, width, italic, oblique);

    // RIBBI family + slot: use family grouping if available, else per-file logic.
    // Look up THIS font's member in the grouping (by stable file_index) so its intrinsic
    // sort_key drives dup-slot disambiguation deterministically across runs.
    let (ribbi_family, ribbi_slot, dup_ordinal) = if let Some(g) = ctx.family {
        let member = ctx
            .file_index
            .and_then(|idx| g.members.iter().find(|m| m.file_index == idx).cloned())
            .unwrap_or(FamilyMember {
                file_index: ctx.file_index.unwrap_or(0),
                sort_key: String::new(),
                weight,
                italic,
                width,
            });
        let a = g.assign(&member);
        (a.ribbi_family, a.ribbi_slot, a.dup_ordinal)
    } else {
        let (f, s) = per_file_ribbi(&typographic_family, weight, width, italic);
        (f, s, 0)
    };
    if dup_ordinal > 0 {
        conflicts.push(Conflict::new(
            Field::NamePostscriptId6,
            format!(
                "duplicate RIBBI slot within family '{ribbi_family}' \
                 ({ribbi_slot:?}); PostScript name disambiguated"
            ),
        ));
    }

    // USE_TYPO_METRICS: preserve original, force on if requested. Never clear.
    let use_typo_metrics = sig.fs_use_typo_metrics() || ctx.options.force_use_typo_metrics;

    // A dup-slot collision yields a stable Some(n>=2) discriminator (N4).
    let dup_suffix = (dup_ordinal > 0).then_some(dup_ordinal + 1);

    let mut style = ResolvedStyle {
        typographic_family,
        typographic_subfamily,
        ribbi_family,
        ribbi_slot,
        weight,
        width,
        italic,
        oblique,
        monospace,
        monospace_authoritative,
        postscript_name: String::new(),
        use_typo_metrics,
        dup_suffix,
    };

    // PostScript name from the canonical family + subfamily (always keep style token).
    // The dup discriminator keeps ID6 unique for colliding faces (N4/N3).
    let ps_sub = if style.typographic_subfamily == "Regular" {
        "Regular".to_string()
    } else {
        style.typographic_subfamily.replace(' ', "")
    };
    let suffix = style
        .dup_suffix
        .map(|n| format!("-{n}"))
        .unwrap_or_default();
    style.postscript_name = names::sanitize_postscript(&format!(
        "{}-{}{}",
        style.typographic_family.replace(' ', ""),
        ps_sub,
        suffix
    ));

    let changes = compute_changes(sig, &style, legacy_weight);

    Resolution {
        style,
        changes,
        conflicts,
    }
}

fn per_file_ribbi(family: &str, weight: Weight, width: Width, italic: bool) -> (String, RibbiSlot) {
    let is_ribbi_weight = weight == Weight::REGULAR || weight == Weight::BOLD;
    if is_ribbi_weight && width.is_normal() {
        (
            family.to_string(),
            RibbiSlot::from_bools(weight == Weight::BOLD, italic),
        )
    } else {
        (
            super::family::split_family_name(family, weight, width),
            RibbiSlot::from_bools(false, italic),
        )
    }
}

/// Diff the canonical style against what the font currently encodes.
fn compute_changes(
    sig: &RawSignals,
    style: &ResolvedStyle,
    legacy_weight: bool,
) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    let names = canonical_names(style);

    push_name_change(
        &mut changes,
        Field::NameFamilyId1,
        &sig.name_family_id1,
        Some(&names.family),
    );
    push_name_change(
        &mut changes,
        Field::NameSubfamilyId2,
        &sig.name_subfamily_id2,
        Some(&names.subfamily),
    );
    push_name_change(
        &mut changes,
        Field::NameFullId4,
        &sig.name_full_id4,
        Some(&names.full),
    );
    push_name_change(
        &mut changes,
        Field::NamePostscriptId6,
        &sig.name_postscript_id6,
        Some(&names.postscript),
    );
    push_name_change(
        &mut changes,
        Field::NameTypoFamilyId16,
        &sig.name_typo_family_id16,
        names.typo_family.as_deref(),
    );
    push_name_change(
        &mut changes,
        Field::NameTypoSubfamilyId17,
        &sig.name_typo_subfamily_id17,
        names.typo_subfamily.as_deref(),
    );

    // usWeightClass
    let target_weight = style.weight.0;
    if let Some(c) = sig.us_weight_class {
        if c != target_weight {
            let note = if legacy_weight { " (legacy)" } else { "" };
            changes.push(FieldChange::new(
                Field::UsWeightClass,
                format!("{c}{note}"),
                target_weight.to_string(),
            ));
        }
    } else {
        changes.push(FieldChange::new(
            Field::UsWeightClass,
            "(none)",
            target_weight.to_string(),
        ));
    }

    // usWidthClass
    let target_width = style.width.0 as u16;
    if let Some(c) = sig.us_width_class {
        if c != target_width {
            changes.push(FieldChange::new(
                Field::UsWidthClass,
                c.to_string(),
                target_width.to_string(),
            ));
        }
    } else if !style.width.is_normal() {
        changes.push(FieldChange::new(
            Field::UsWidthClass,
            "(none)",
            target_width.to_string(),
        ));
    }

    // fsSelection
    let target_fs = target_fs_selection(sig, style);
    if let Some(cur) = sig.fs_selection {
        if cur != target_fs {
            changes.push(FieldChange::new(
                Field::FsSelection,
                format!("{cur:#06x}"),
                format!("{target_fs:#06x}"),
            ));
        }
    } else {
        changes.push(FieldChange::new(
            Field::FsSelection,
            "(none)",
            format!("{target_fs:#06x}"),
        ));
    }

    // macStyle
    let target_mac = target_mac_style(sig, style);
    if let Some(cur) = sig.mac_style
        && cur != target_mac
    {
        changes.push(FieldChange::new(
            Field::MacStyle,
            format!("{cur:#06x}"),
            format!("{target_mac:#06x}"),
        ));
    }

    // panose bWeight + bProportion
    if let Some(p) = sig.panose {
        let tw = style.weight.panose_weight();
        if p[0] == 2 && panose_weight_needs_fix(p[2], tw) {
            changes.push(FieldChange::new(
                Field::PanoseWeight,
                p[2].to_string(),
                tw.to_string(),
            ));
        }
        // bProportion: only adjust when the monospace decision is authoritative
        // (measured). Set 9 for monospace; clear a stale 9 to proportional (4) when not.
        // For non-Latin fonts (non-authoritative) leave it untouched.
        let target_prop = if !style.monospace_authoritative {
            p[3]
        } else if style.monospace {
            9u8
        } else if p[3] == 9 {
            4
        } else {
            p[3]
        };
        if p[0] == 2 && p[3] != target_prop {
            changes.push(FieldChange::new(
                Field::PanoseProportion,
                p[3].to_string(),
                target_prop.to_string(),
            ));
        }
    }

    // post.isFixedPitch — only change when the monospace decision is authoritative.
    let target_fixed = u32::from(style.monospace);
    if style.monospace_authoritative
        && let Some(cur) = sig.is_fixed_pitch
    {
        let cur_bool = cur != 0;
        if cur_bool != style.monospace {
            changes.push(FieldChange::new(
                Field::PostIsFixedPitch,
                cur.to_string(),
                target_fixed.to_string(),
            ));
        }
    }

    changes
}

fn push_name_change(
    changes: &mut Vec<FieldChange>,
    field: Field,
    current: &Option<String>,
    target: Option<&str>,
) {
    match (current.as_deref(), target) {
        (Some(c), Some(t)) if c != t => changes.push(FieldChange::new(field, c, t)),
        (None, Some(t)) => changes.push(FieldChange::new(field, "(none)", t)),
        (Some(c), None) => changes.push(FieldChange::new(field, c, "(removed)")),
        _ => {}
    }
}

/// Compute the canonical fsSelection value from the resolved style, preserving
/// reserved bits and USE_TYPO_METRICS from the original.
pub fn target_fs_selection(sig: &RawSignals, style: &ResolvedStyle) -> u16 {
    use super::signals::{FS_BOLD, FS_ITALIC, FS_OBLIQUE, FS_REGULAR, FS_USE_TYPO_METRICS};
    // Start from the original, but clear all managed bits.
    let managed = FS_ITALIC | FS_BOLD | FS_REGULAR | FS_OBLIQUE | FS_USE_TYPO_METRICS;
    let mut fs = sig.fs_selection.unwrap_or(0) & !managed;

    if style.ribbi_slot.is_italic() {
        fs |= FS_ITALIC;
    }
    if style.ribbi_slot.is_bold() {
        fs |= FS_BOLD;
    }
    if style.ribbi_slot.is_regular() {
        fs |= FS_REGULAR;
    }
    if style.oblique {
        fs |= FS_OBLIQUE;
    }
    if style.use_typo_metrics {
        fs |= FS_USE_TYPO_METRICS;
    }
    fs
}

/// Compute the canonical macStyle, preserving reserved/other bits from the original.
pub fn target_mac_style(sig: &RawSignals, style: &ResolvedStyle) -> u16 {
    use super::signals::{MAC_BOLD, MAC_ITALIC};
    let managed = MAC_BOLD | MAC_ITALIC;
    let mut mac = sig.mac_style.unwrap_or(0) & !managed;
    if style.ribbi_slot.is_bold() {
        mac |= MAC_BOLD;
    }
    if style.ribbi_slot.is_italic() {
        mac |= MAC_ITALIC;
    }
    mac
}

/// Decide whether PANOSE bWeight should be corrected toward `target`.
///
/// PANOSE bWeight (2..=11) is corroborating, not authoritative, and real fonts ship
/// adjacent-rung values that are harmless. We only correct a genuinely misleading value:
/// unset (0/1), a value that crosses the Bold boundary relative to the resolved weight
/// (the exact Kobo `Yrsa-Bold` bug: Bold face with bWeight=Book), or one off by >=2 rungs.
/// A 1-rung difference (e.g. Book=5 vs Medium=6) is left untouched.
pub fn panose_weight_needs_fix(current: u8, target: u8) -> bool {
    const BOLD_THRESHOLD: u8 = 8;
    if current < 2 {
        return true; // unset / no-fit
    }
    let crosses_bold = (current >= BOLD_THRESHOLD) != (target >= BOLD_THRESHOLD);
    let far = current.abs_diff(target) >= 2;
    crosses_bold || far
}
