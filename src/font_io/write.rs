use std::path::Path;

use read_fonts::tables::head::MacStyle;
use read_fonts::tables::os2::SelectionFlags;
use read_fonts::types::Tag;
use read_fonts::{FontRef, TableProvider};

use write_fonts::FontBuilder;
use write_fonts::from_obj::ToOwnedTable;
use write_fonts::tables::head::Head;
use write_fonts::tables::os2::Os2;
use write_fonts::tables::post::Post;

use crate::core::model::ResolvedStyle;
use crate::core::names::canonical_names;
use crate::core::resolve::{panose_weight_needs_fix, target_fs_selection, target_mac_style};
use crate::core::signals::RawSignals;
use crate::error::FontError;

const DSIG: Tag = Tag::new(b"DSIG");

/// Apply a `ResolvedStyle` to a font, producing new sfnt bytes.
///
/// Mutates only the four metadata tables (name, OS/2, head, post). Every other table —
/// including glyf/loca/CFF outlines — passes through `copy_missing_tables` as raw bytes,
/// byte-identical. DSIG is dropped (any metadata edit invalidates a signature).
pub fn apply(
    font: &FontRef,
    sig: &RawSignals,
    style: &ResolvedStyle,
    path: &Path,
) -> Result<Vec<u8>, FontError> {
    let names = canonical_names(style);

    let read_name = font.name().map_err(|e| werr(path, e))?;
    let name = super::tables::build_name(&read_name, &names, sig.has_mac_name_records);

    // OS/2
    let mut os2: Os2 = font.os2().map_err(|e| werr(path, e))?.to_owned_table();
    os2.us_weight_class = style.weight.0;
    os2.us_width_class = style.width.0 as u16;
    os2.fs_selection = SelectionFlags::from_bits_truncate(target_fs_selection(sig, style));
    if os2.panose_10[0] == 2 {
        let target = style.weight.panose_weight();
        if panose_weight_needs_fix(os2.panose_10[2], target) {
            os2.panose_10[2] = target;
        }
        // bProportion: 9 = Monospaced. Only touch it when the monospace decision is
        // authoritative (measured); leave non-Latin fonts (CJK) untouched (H1). Set 9 for
        // monospace; clear a stale 9 to proportional (4) otherwise (M3).
        if style.monospace_authoritative {
            if style.monospace {
                os2.panose_10[3] = 9;
            } else if os2.panose_10[3] == 9 {
                os2.panose_10[3] = 4;
            }
        }
    }

    // head
    let mut head: Head = font.head().map_err(|e| werr(path, e))?.to_owned_table();
    head.mac_style = MacStyle::from_bits_truncate(target_mac_style(sig, style));

    let mut builder = FontBuilder::new();
    builder.add_table(&name).map_err(|e| werr(path, e))?;
    builder.add_table(&os2).map_err(|e| werr(path, e))?;
    builder.add_table(&head).map_err(|e| werr(path, e))?;

    // post: rewrite isFixedPitch only when the monospace decision is authoritative;
    // otherwise preserve the font's existing value (do not mark CJK fonts, H1).
    if let Ok(read_post) = font.post() {
        let mut post: Post = read_post.to_owned_table();
        if style.monospace_authoritative {
            post.is_fixed_pitch = u32::from(style.monospace);
        }
        builder.add_table(&post).map_err(|e| werr(path, e))?;
    }

    // Pass through every other source table as raw bytes, dropping DSIG (a metadata
    // edit invalidates any embedded signature). This is `copy_missing_tables` minus DSIG.
    for record in font.table_directory().table_records() {
        let tag = record.tag();
        if tag == DSIG || builder.contains(tag) {
            continue;
        }
        if let Some(data) = font.data_for_tag(tag) {
            builder.add_raw(tag, data.as_bytes());
        }
    }

    let bytes = builder.build();

    // Sanity: re-parse the output so we never write a font we can't read back.
    FontRef::new(&bytes).map_err(|e| werr(path, e))?;
    Ok(bytes)
}

fn werr(path: &Path, e: impl std::fmt::Display) -> FontError {
    FontError::Write(path.to_path_buf(), e.to_string())
}
