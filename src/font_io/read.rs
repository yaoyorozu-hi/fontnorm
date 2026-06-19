use std::path::Path;

use read_fonts::tables::name::Name;
use read_fonts::types::{NameId, Tag};
use read_fonts::{FontRef, TableProvider};

use crate::core::signals::RawSignals;
use crate::error::FontError;

use super::monospace;

const WINDOWS: u16 = 3;
const WIN_ENC_BMP: u16 = 1;
const WIN_LANG_EN_US: u16 = 0x0409;
const MAC: u16 = 1;

/// Best decoded string for a name ID: prefer Windows 3/1/0x409, then any Windows,
/// then Mac, then any record.
fn best_name(name: &Name, id: NameId) -> Option<String> {
    let records = name.name_record();
    let data = name.string_data();

    let mut win_best: Option<String> = None;
    let mut win_any: Option<String> = None;
    let mut mac_any: Option<String> = None;
    let mut any: Option<String> = None;

    for r in records {
        if r.name_id() != id {
            continue;
        }
        let Ok(s) = r.string(data) else { continue };
        let s = s.to_string();
        if any.is_none() {
            any = Some(s.clone());
        }
        if r.platform_id() == WINDOWS {
            if r.encoding_id() == WIN_ENC_BMP && r.language_id() == WIN_LANG_EN_US {
                win_best.get_or_insert(s.clone());
            }
            win_any.get_or_insert(s.clone());
        } else if r.platform_id() == MAC {
            mac_any.get_or_insert(s.clone());
        }
    }
    win_best.or(win_any).or(mac_any).or(any)
}

fn has_mac_records(name: &Name) -> bool {
    name.name_record().iter().any(|r| r.platform_id() == MAC)
}

/// Read every relevant signal from a font into plain `RawSignals`.
/// Requires `name`, `OS/2`, and `head`; absence is a per-font error.
pub fn read_signals(font: &FontRef, path: &Path) -> Result<RawSignals, FontError> {
    let name = font.name().map_err(|_| FontError::MissingTable {
        path: path.to_path_buf(),
        table: "name",
    })?;
    let os2 = font.os2().map_err(|_| FontError::MissingTable {
        path: path.to_path_buf(),
        table: "OS/2",
    })?;
    let head = font.head().map_err(|_| FontError::MissingTable {
        path: path.to_path_buf(),
        table: "head",
    })?;

    let panose: [u8; 10] = os2.panose_10().try_into().unwrap_or([0; 10]);

    let (is_fixed_pitch, italic_angle) = match font.post() {
        Ok(post) => (
            Some(post.is_fixed_pitch()),
            Some(post.italic_angle().to_f32()),
        ),
        Err(_) => (None, None),
    };

    let filename_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let has_dsig = font.data_for_tag(Tag::new(b"DSIG")).is_some();
    let is_variable = font.data_for_tag(Tag::new(b"fvar")).is_some();

    Ok(RawSignals {
        name_family_id1: best_name(&name, NameId::FAMILY_NAME),
        name_subfamily_id2: best_name(&name, NameId::SUBFAMILY_NAME),
        name_full_id4: best_name(&name, NameId::FULL_NAME),
        name_postscript_id6: best_name(&name, NameId::POSTSCRIPT_NAME),
        name_typo_family_id16: best_name(&name, NameId::TYPOGRAPHIC_FAMILY_NAME),
        name_typo_subfamily_id17: best_name(&name, NameId::TYPOGRAPHIC_SUBFAMILY_NAME),

        us_weight_class: Some(os2.us_weight_class()),
        us_width_class: Some(os2.us_width_class()),
        fs_selection: Some(os2.fs_selection().bits()),
        panose: Some(panose),

        mac_style: Some(head.mac_style().bits()),

        is_fixed_pitch,
        italic_angle,

        monospace_measure: monospace::measure(font),

        filename_stem,

        has_mac_name_records: has_mac_records(&name),
        has_dsig,
        is_variable,
    })
}
