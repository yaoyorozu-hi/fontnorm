#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Weight(pub u16);

impl Weight {
    pub const THIN: Weight = Weight(100);
    pub const EXTRA_LIGHT: Weight = Weight(200);
    pub const LIGHT: Weight = Weight(300);
    pub const REGULAR: Weight = Weight(400);
    pub const MEDIUM: Weight = Weight(500);
    pub const SEMI_BOLD: Weight = Weight(600);
    pub const BOLD: Weight = Weight(700);
    pub const EXTRA_BOLD: Weight = Weight(800);
    pub const BLACK: Weight = Weight(900);

    const LADDER: [Weight; 9] = [
        Weight::THIN,
        Weight::EXTRA_LIGHT,
        Weight::LIGHT,
        Weight::REGULAR,
        Weight::MEDIUM,
        Weight::SEMI_BOLD,
        Weight::BOLD,
        Weight::EXTRA_BOLD,
        Weight::BLACK,
    ];

    /// Canonical display word, e.g. Weight(600) -> "SemiBold".
    pub fn name(self) -> &'static str {
        match self.0 {
            100 => "Thin",
            200 => "ExtraLight",
            300 => "Light",
            400 => "Regular",
            500 => "Medium",
            600 => "SemiBold",
            700 => "Bold",
            800 => "ExtraBold",
            900 => "Black",
            _ => "Regular",
        }
    }

    /// PostScript-style token (no space). Same as `name` for the standard ladder.
    pub fn token(self) -> &'static str {
        self.name()
    }

    /// Nearest standard rung for an arbitrary class value.
    /// Legacy 250/275 are mapped by `weight::normalize_legacy` before reaching here,
    /// but this is a robust fallback for any value.
    pub fn nearest(class: u16) -> Weight {
        let class = class.clamp(1, 1000);
        let mut best = Weight::REGULAR;
        let mut best_dist = u16::MAX;
        for w in Weight::LADDER {
            let dist = class.abs_diff(w.0);
            if dist < best_dist {
                best_dist = dist;
                best = w;
            }
        }
        best
    }

    pub fn is_regular(self) -> bool {
        self == Weight::REGULAR
    }

    /// True iff this weight maps to the RIBBI Bold style-link slot (== 700).
    pub fn is_bold_slot(self) -> bool {
        self == Weight::BOLD
    }

    /// PANOSE bWeight (byte 2) value corresponding to this weight.
    /// 5 = Book/Regular, 8 = Bold. Intermediate rungs mapped to the PANOSE 2..11 scale.
    pub fn panose_weight(self) -> u8 {
        match self.0 {
            100 => 2,
            200 => 3,
            300 => 4,
            400 => 5,
            500 => 6,
            600 => 7,
            700 => 8,
            800 => 9,
            900 => 10,
            _ => 5,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Width(pub u8);

impl Width {
    pub const NORMAL: Width = Width(5);

    pub fn from_class(class: u16) -> Width {
        Width(class.clamp(1, 9) as u8)
    }

    pub fn name(self) -> &'static str {
        match self.0 {
            1 => "UltraCondensed",
            2 => "ExtraCondensed",
            3 => "Condensed",
            4 => "SemiCondensed",
            5 => "Normal",
            6 => "SemiExpanded",
            7 => "Expanded",
            8 => "ExtraExpanded",
            9 => "UltraExpanded",
            _ => "Normal",
        }
    }

    /// Token used in subfamily / filename composition. None for Normal (omitted).
    pub fn token(self) -> Option<&'static str> {
        if self == Width::NORMAL {
            None
        } else {
            Some(self.name())
        }
    }

    pub fn is_normal(self) -> bool {
        self == Width::NORMAL
    }
}

/// One of the 4 RIBBI style-link slots. The (BOLD, ITALIC) pair that must be
/// unique within an ID1 group (invariant N4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RibbiSlot {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl RibbiSlot {
    pub fn from_bools(bold: bool, italic: bool) -> RibbiSlot {
        match (bold, italic) {
            (false, false) => RibbiSlot::Regular,
            (true, false) => RibbiSlot::Bold,
            (false, true) => RibbiSlot::Italic,
            (true, true) => RibbiSlot::BoldItalic,
        }
    }

    pub fn is_bold(self) -> bool {
        matches!(self, RibbiSlot::Bold | RibbiSlot::BoldItalic)
    }

    pub fn is_italic(self) -> bool {
        matches!(self, RibbiSlot::Italic | RibbiSlot::BoldItalic)
    }

    pub fn is_regular(self) -> bool {
        matches!(self, RibbiSlot::Regular)
    }

    /// The OpenType ID2 subfamily string for this slot.
    pub fn subfamily_name(self) -> &'static str {
        match self {
            RibbiSlot::Regular => "Regular",
            RibbiSlot::Bold => "Bold",
            RibbiSlot::Italic => "Italic",
            RibbiSlot::BoldItalic => "Bold Italic",
        }
    }
}

/// THE canonical identity. Output of resolution, input to writing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResolvedStyle {
    pub typographic_family: String,
    pub typographic_subfamily: String,
    pub ribbi_family: String,
    pub ribbi_slot: RibbiSlot,
    pub weight: Weight,
    pub width: Width,
    pub italic: bool,
    pub oblique: bool,
    pub monospace: bool,
    /// True when monospace was determined by measurement (Latin-primary). When false the
    /// `monospace` value only echoes the existing flag and must not drive metadata edits.
    pub monospace_authoritative: bool,
    pub postscript_name: String,
    pub use_typo_metrics: bool,
    /// Stable discriminator for a face that collides with a sibling in the same RIBBI slot
    /// (N4). `None` for a unique face; `Some(n)` (n >= 2) for the n-th colliding face by its
    /// intrinsic sort key. Applied identically to the PostScript name AND the output
    /// filename so both stay unique and idempotent across runs.
    pub dup_suffix: Option<u32>,
}
