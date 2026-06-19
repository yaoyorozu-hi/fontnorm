/// Identifies a specific metadata field for change/conflict reporting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    NameFamilyId1,
    NameSubfamilyId2,
    NameFullId4,
    NamePostscriptId6,
    NameTypoFamilyId16,
    NameTypoSubfamilyId17,
    UsWeightClass,
    UsWidthClass,
    FsSelection,
    PanoseWeight,
    PanoseProportion,
    MacStyle,
    PostIsFixedPitch,
    Filename,
}

impl Field {
    pub fn label(self) -> &'static str {
        match self {
            Field::NameFamilyId1 => "name.ID1 (Family)",
            Field::NameSubfamilyId2 => "name.ID2 (Subfamily)",
            Field::NameFullId4 => "name.ID4 (Full)",
            Field::NamePostscriptId6 => "name.ID6 (PostScript)",
            Field::NameTypoFamilyId16 => "name.ID16 (Typo Family)",
            Field::NameTypoSubfamilyId17 => "name.ID17 (Typo Subfamily)",
            Field::UsWeightClass => "OS/2.usWeightClass",
            Field::UsWidthClass => "OS/2.usWidthClass",
            Field::FsSelection => "OS/2.fsSelection",
            Field::PanoseWeight => "OS/2.panose.bWeight",
            Field::PanoseProportion => "OS/2.panose.bProportion",
            Field::MacStyle => "head.macStyle",
            Field::PostIsFixedPitch => "post.isFixedPitch",
            Field::Filename => "filename",
        }
    }
}

/// A single before -> after change produced by normalization.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldChange {
    pub field: Field,
    pub before: String,
    pub after: String,
}

impl FieldChange {
    pub fn new(field: Field, before: impl Into<String>, after: impl Into<String>) -> Self {
        FieldChange {
            field,
            before: before.into(),
            after: after.into(),
        }
    }
}

/// A disagreement between signal sources, reported even when auto-resolved.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Conflict {
    pub field: Field,
    pub detail: String,
}

impl Conflict {
    pub fn new(field: Field, detail: impl Into<String>) -> Self {
        Conflict {
            field,
            detail: detail.into(),
        }
    }
}
