/// Every metadata signal lifted out of a font, as plain data. Produced by
/// `font_io::read`, consumed by `core::resolve`. No font-crate types here.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct RawSignals {
    // name table (Windows 3/1/0x409 preferred, Mac fallback), decoded to String
    pub name_family_id1: Option<String>,
    pub name_subfamily_id2: Option<String>,
    pub name_full_id4: Option<String>,
    pub name_postscript_id6: Option<String>,
    pub name_typo_family_id16: Option<String>,
    pub name_typo_subfamily_id17: Option<String>,

    // OS/2
    pub us_weight_class: Option<u16>,
    pub us_width_class: Option<u16>,
    pub fs_selection: Option<u16>,
    pub panose: Option<[u8; 10]>,

    // head
    pub mac_style: Option<u16>,

    // post
    pub is_fixed_pitch: Option<u32>,
    pub italic_angle: Option<f32>,

    // measured ground truth
    pub monospace_measure: MonospaceMeasurement,

    // weak hint
    pub filename_stem: String,

    // provenance
    pub has_mac_name_records: bool,
    pub has_dsig: bool,
    /// True iff the font has an `fvar` table (variable font). v1 skips these.
    pub is_variable: bool,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct MonospaceMeasurement {
    /// Font is Latin-primary (Latin alphabet present, not CJK/Kana/Hangul-dominated).
    pub has_latin: bool,
    /// fontbakery 80%-share-one-width result, measured over Latin letters.
    pub seems_monospaced: bool,
}

// fsSelection bit positions
pub const FS_ITALIC: u16 = 0x0001;
pub const FS_BOLD: u16 = 0x0020;
pub const FS_REGULAR: u16 = 0x0040;
pub const FS_USE_TYPO_METRICS: u16 = 0x0080;
pub const FS_OBLIQUE: u16 = 0x0200;

// macStyle bit positions
pub const MAC_BOLD: u16 = 0x0001;
pub const MAC_ITALIC: u16 = 0x0002;

impl RawSignals {
    pub fn fs_italic(&self) -> bool {
        self.fs_selection.is_some_and(|b| b & FS_ITALIC != 0)
    }
    pub fn fs_bold(&self) -> bool {
        self.fs_selection.is_some_and(|b| b & FS_BOLD != 0)
    }
    pub fn fs_oblique(&self) -> bool {
        self.fs_selection.is_some_and(|b| b & FS_OBLIQUE != 0)
    }
    pub fn fs_use_typo_metrics(&self) -> bool {
        self.fs_selection
            .is_some_and(|b| b & FS_USE_TYPO_METRICS != 0)
    }
    pub fn mac_bold(&self) -> bool {
        self.mac_style.is_some_and(|b| b & MAC_BOLD != 0)
    }
    pub fn mac_italic(&self) -> bool {
        self.mac_style.is_some_and(|b| b & MAC_ITALIC != 0)
    }
    pub fn panose_weight(&self) -> Option<u8> {
        self.panose.map(|p| p[2])
    }
    pub fn panose_proportion(&self) -> Option<u8> {
        self.panose.map(|p| p[3])
    }
}
