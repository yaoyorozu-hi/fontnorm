use fontnorm::core::signals::{
    FS_BOLD, FS_ITALIC, FS_OBLIQUE, FS_REGULAR, FS_USE_TYPO_METRICS, MAC_BOLD, MAC_ITALIC,
    MonospaceMeasurement, RawSignals,
};

/// Fluent builder for RawSignals with sensible defaults, for table-driven tests.
#[derive(Default)]
pub struct Sig {
    s: RawSignals,
}

#[allow(dead_code)]
impl Sig {
    pub fn new() -> Self {
        Sig::default()
    }

    pub fn family(mut self, v: &str) -> Self {
        self.s.name_family_id1 = Some(v.to_string());
        self
    }
    pub fn subfamily(mut self, v: &str) -> Self {
        self.s.name_subfamily_id2 = Some(v.to_string());
        self
    }
    pub fn full(mut self, v: &str) -> Self {
        self.s.name_full_id4 = Some(v.to_string());
        self
    }
    pub fn postscript(mut self, v: &str) -> Self {
        self.s.name_postscript_id6 = Some(v.to_string());
        self
    }
    pub fn typo_family(mut self, v: &str) -> Self {
        self.s.name_typo_family_id16 = Some(v.to_string());
        self
    }
    pub fn typo_subfamily(mut self, v: &str) -> Self {
        self.s.name_typo_subfamily_id17 = Some(v.to_string());
        self
    }
    pub fn weight_class(mut self, v: u16) -> Self {
        self.s.us_weight_class = Some(v);
        self
    }
    pub fn width_class(mut self, v: u16) -> Self {
        self.s.us_width_class = Some(v);
        self
    }
    pub fn filename(mut self, v: &str) -> Self {
        self.s.filename_stem = v.to_string();
        self
    }
    pub fn fs_bold(mut self) -> Self {
        self.s.fs_selection = Some(self.s.fs_selection.unwrap_or(0) | FS_BOLD);
        self
    }
    pub fn fs_italic(mut self) -> Self {
        self.s.fs_selection = Some(self.s.fs_selection.unwrap_or(0) | FS_ITALIC);
        self
    }
    pub fn fs_regular(mut self) -> Self {
        self.s.fs_selection = Some(self.s.fs_selection.unwrap_or(0) | FS_REGULAR);
        self
    }
    pub fn fs_oblique(mut self) -> Self {
        self.s.fs_selection = Some(self.s.fs_selection.unwrap_or(0) | FS_OBLIQUE);
        self
    }
    pub fn fs_use_typo_metrics(mut self) -> Self {
        self.s.fs_selection = Some(self.s.fs_selection.unwrap_or(0) | FS_USE_TYPO_METRICS);
        self
    }
    pub fn fs_raw(mut self, v: u16) -> Self {
        self.s.fs_selection = Some(v);
        self
    }
    pub fn mac_bold(mut self) -> Self {
        self.s.mac_style = Some(self.s.mac_style.unwrap_or(0) | MAC_BOLD);
        self
    }
    pub fn mac_italic(mut self) -> Self {
        self.s.mac_style = Some(self.s.mac_style.unwrap_or(0) | MAC_ITALIC);
        self
    }
    pub fn mac_raw(mut self, v: u16) -> Self {
        self.s.mac_style = Some(v);
        self
    }
    pub fn panose(mut self, p: [u8; 10]) -> Self {
        self.s.panose = Some(p);
        self
    }
    pub fn panose_weight(mut self, w: u8) -> Self {
        let mut p = self.s.panose.unwrap_or([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        p[0] = 2;
        p[2] = w;
        self.s.panose = Some(p);
        self
    }
    pub fn panose_proportion(mut self, v: u8) -> Self {
        let mut p = self.s.panose.unwrap_or([2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        p[0] = 2;
        p[3] = v;
        self.s.panose = Some(p);
        self
    }
    pub fn fixed_pitch(mut self, v: u32) -> Self {
        self.s.is_fixed_pitch = Some(v);
        self
    }
    pub fn monospace_measured(mut self, has_latin: bool, seems: bool) -> Self {
        self.s.monospace_measure = MonospaceMeasurement {
            has_latin,
            seems_monospaced: seems,
        };
        self
    }
    pub fn build(self) -> RawSignals {
        self.s
    }
}
