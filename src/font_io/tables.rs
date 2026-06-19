use read_fonts::tables::name::{MacRomanMapping, Name as ReadName};
use read_fonts::types::NameId;

use write_fonts::tables::name::{Name as WriteName, NameRecord};

use crate::core::names::CanonicalNames;

const WINDOWS: u16 = 3;
const WIN_ENC_BMP: u16 = 1;
const WIN_LANG_EN_US: u16 = 0x0409;
const MAC: u16 = 1;
const MAC_ENC_ROMAN: u16 = 0;
const MAC_LANG_EN: u16 = 0;

/// Whether a (platform, encoding) pair encodes name strings as MacRoman. write-fonts
/// PANICS encoding a non-MacRoman char into such a record, so we must never hand it one.
fn is_macroman(platform_id: u16, encoding_id: u16) -> bool {
    platform_id == MAC && encoding_id == MAC_ENC_ROMAN
}

/// Whether every char in `s` is representable in MacRoman.
fn macroman_encodable(s: &str) -> bool {
    s.chars().all(|c| MacRomanMapping.encode(c).is_some())
}

/// The identity name IDs this tool governs. All other records pass through verbatim.
fn managed_id(id: NameId) -> bool {
    matches!(
        id,
        NameId::FAMILY_NAME
            | NameId::SUBFAMILY_NAME
            | NameId::FULL_NAME
            | NameId::POSTSCRIPT_NAME
            | NameId::TYPOGRAPHIC_FAMILY_NAME
            | NameId::TYPOGRAPHIC_SUBFAMILY_NAME
    )
}

fn canonical_value(id: NameId, names: &CanonicalNames) -> Option<String> {
    match id {
        NameId::FAMILY_NAME => Some(names.family.clone()),
        NameId::SUBFAMILY_NAME => Some(names.subfamily.clone()),
        NameId::FULL_NAME => Some(names.full.clone()),
        NameId::POSTSCRIPT_NAME => Some(names.postscript.clone()),
        NameId::TYPOGRAPHIC_FAMILY_NAME => names.typo_family.clone(),
        NameId::TYPOGRAPHIC_SUBFAMILY_NAME => names.typo_subfamily.clone(),
        _ => None,
    }
}

/// Build the canonical owned `Name` table:
/// - All non-identity records (copyright, version, license, unique ID, ...) pass through.
/// - Managed identity records (1/2/4/6/16/17) are replaced with canonical values on the
///   platforms we manage: Windows 3/1/0x409 always; Mac 1/0/0 when the font already ships
///   Mac records.
/// - Records are sorted by (platformID, encodingID, languageID, nameID).
pub fn build_name(read_name: &ReadName, names: &CanonicalNames, has_mac: bool) -> WriteName {
    let data = read_name.string_data();
    let mut records: Vec<NameRecord> = Vec::new();

    // Pass-through every non-managed record verbatim. A Mac (MacRoman) record whose
    // decoded string is not MacRoman-encodable (mojibake / corruption) would panic
    // write-fonts on re-encode; drop such records rather than abort.
    for r in read_name.name_record() {
        if managed_id(r.name_id()) {
            continue;
        }
        let Ok(s) = r.string(data) else { continue };
        let s = s.to_string();
        if is_macroman(r.platform_id(), r.encoding_id()) && !macroman_encodable(&s) {
            continue;
        }
        records.push(NameRecord::new(
            r.platform_id(),
            r.encoding_id(),
            r.language_id(),
            r.name_id(),
            s.into(),
        ));
    }

    // Emit canonical identity records.
    let identity_ids = [
        NameId::FAMILY_NAME,
        NameId::SUBFAMILY_NAME,
        NameId::FULL_NAME,
        NameId::POSTSCRIPT_NAME,
        NameId::TYPOGRAPHIC_FAMILY_NAME,
        NameId::TYPOGRAPHIC_SUBFAMILY_NAME,
    ];
    for id in identity_ids {
        let Some(value) = canonical_value(id, names) else {
            continue; // ID16/17 omitted when None
        };
        records.push(NameRecord::new(
            WINDOWS,
            WIN_ENC_BMP,
            WIN_LANG_EN_US,
            id,
            value.clone().into(),
        ));
        // The Mac (1/0/0) identity record is legacy-compat. Emit it only when the
        // resolved value is MacRoman-encodable; otherwise the Windows 3/1/0x409 record
        // carries identity alone. Non-MacRoman family names (CJK, accents) must never
        // panic write-fonts' MacRoman encoder.
        if has_mac && macroman_encodable(&value) {
            records.push(NameRecord::new(
                MAC,
                MAC_ENC_ROMAN,
                MAC_LANG_EN,
                id,
                value.into(),
            ));
        }
    }

    records.sort_by_key(|r| {
        (
            r.platform_id,
            r.encoding_id,
            r.language_id,
            r.name_id.to_u16(),
        )
    });

    WriteName::new(records)
}
