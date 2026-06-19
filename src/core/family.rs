use super::model::{RibbiSlot, Weight, Width};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FamilyMember {
    pub file_index: usize,
    /// A STABLE, intrinsic key for ordering colliding faces deterministically across runs
    /// (original PostScript name, else original filename stem). Never the processing order,
    /// which changes when files are renamed.
    pub sort_key: String,
    pub weight: Weight,
    pub italic: bool,
    pub width: Width,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FamilyGrouping {
    /// Canonical typographic family string for the whole group.
    pub typographic_family: String,
    pub members: Vec<FamilyMember>,
    /// True iff the family cannot fit in a single 4-slot RIBBI group:
    /// any member has a non-{400,700} weight, a non-normal width, or the
    /// (bold,italic) RIBBI slots are not unique.
    pub exceeds_ribbi: bool,
}

impl FamilyGrouping {
    pub fn new(typographic_family: String, members: Vec<FamilyMember>) -> Self {
        let exceeds = Self::compute_exceeds(&members);
        FamilyGrouping {
            typographic_family,
            members,
            exceeds_ribbi: exceeds,
        }
    }

    fn compute_exceeds(members: &[FamilyMember]) -> bool {
        let mut seen_slots: Vec<(bool, bool)> = Vec::new();
        for m in members {
            let non_ribbi_weight = !(m.weight == Weight::REGULAR || m.weight == Weight::BOLD);
            if non_ribbi_weight || !m.width.is_normal() {
                return true;
            }
            let slot = (m.weight == Weight::BOLD, m.italic);
            if seen_slots.contains(&slot) {
                return true;
            }
            seen_slots.push(slot);
        }
        false
    }

    /// Decide identity for a member, consistently with siblings.
    ///
    /// RIBBI weight (400/700) + normal width => lives in the base family, occupying
    /// the natural (bold, italic) slot. Anything else => its own ID1 family named
    /// "<Family> <Weight><Width>" with a Regular/Italic slot inside that sub-family.
    ///
    /// `dup_ordinal` is the rank of this member among siblings with an IDENTICAL
    /// (weight, italic, width) facet tuple, ordered by the STABLE `sort_key`. It is 0 for a
    /// unique face and 1, 2, ... for collisions (the Kobo dup-Regular bug at the identity
    /// level, N4). Because the ordering is intrinsic to each font (original PostScript name
    /// / filename), the same physical font gets the same ordinal on every run — the
    /// disambiguator is stable and idempotent.
    pub fn assign(&self, m: &FamilyMember) -> Assignment {
        let is_ribbi_weight = m.weight == Weight::REGULAR || m.weight == Weight::BOLD;
        let (ribbi_family, ribbi_slot) = if is_ribbi_weight && m.width.is_normal() {
            (
                self.typographic_family.clone(),
                RibbiSlot::from_bools(m.weight == Weight::BOLD, m.italic),
            )
        } else {
            (
                split_family_name(&self.typographic_family, m.weight, m.width),
                RibbiSlot::from_bools(false, m.italic),
            )
        };
        Assignment {
            ribbi_family,
            ribbi_slot,
            dup_ordinal: self.dup_ordinal(m),
        }
    }

    /// Count how many siblings with the same facet tuple sort STRICTLY before this member
    /// by `(sort_key, file_index)`. `file_index` only breaks ties when two fonts share a
    /// sort_key (degenerate; identical fonts), keeping the order total and deterministic.
    fn dup_ordinal(&self, m: &FamilyMember) -> u32 {
        self.members
            .iter()
            .filter(|o| {
                o.weight == m.weight
                    && o.italic == m.italic
                    && o.width == m.width
                    && (o.sort_key.as_str(), o.file_index) < (m.sort_key.as_str(), m.file_index)
            })
            .count() as u32
    }
}

/// The identity decision for one family member.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Assignment {
    pub ribbi_family: String,
    pub ribbi_slot: RibbiSlot,
    pub dup_ordinal: u32,
}

/// Name of the split ID1 sub-family for a non-RIBBI weight/width face.
pub fn split_family_name(base: &str, weight: Weight, width: Width) -> String {
    let mut s = base.to_string();
    if let Some(wtok) = width.token() {
        s.push(' ');
        s.push_str(wtok);
    }
    if !weight.is_regular() {
        s.push(' ');
        s.push_str(weight.token());
    }
    s
}

/// Normalize a family string into a grouping key: lowercase, style words already
/// stripped by the caller, collapse whitespace.
pub fn family_key(family: &str) -> String {
    family
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
