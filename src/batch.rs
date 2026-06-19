use std::collections::HashMap;
use std::path::{Path, PathBuf};

use read_fonts::FontRef;
use walkdir::WalkDir;

use crate::config::Config;
use crate::core::family::{FamilyGrouping, FamilyMember, family_key};
use crate::core::filename::canonical_filename;
use crate::core::resolve::{ResolveContext, resolve};
use crate::core::signals::RawSignals;
use crate::core::style_words;
use crate::error::{FatalError, FontError};
use crate::font_io::{read, write};
use crate::report::{FontReport, FontStatus, RunReport};

/// A font file discovered in the input directory, paired with its read signals.
struct ScannedFont {
    path: PathBuf,
    ext: String,
    bytes: Vec<u8>,
    signals: RawSignals,
}

fn is_supported_ext(ext: &str) -> bool {
    matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf")
}

fn is_unsupported_container(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "woff" | "woff2" | "ttc" | "otc"
    )
}

/// Discover candidate font paths under the input directory.
fn discover_paths(cfg: &Config) -> Result<Vec<PathBuf>, FatalError> {
    let walker = if cfg.recursive {
        WalkDir::new(&cfg.input_dir)
    } else {
        WalkDir::new(&cfg.input_dir).max_depth(1)
    };

    let mut paths = Vec::new();
    for entry in walker {
        let entry = entry.map_err(|e| {
            FatalError::Scan(
                cfg.input_dir.clone(),
                e.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("walk error")),
            )
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        // Never descend into / pick up files already in the output dir.
        if path.starts_with(&cfg.output_dir) {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && (is_supported_ext(ext) || is_unsupported_container(ext))
        {
            paths.push(path.to_path_buf());
        }
    }
    paths.sort();
    Ok(paths)
}

/// Validate the output dir does not overlap input files, and create it.
fn prepare_output(cfg: &Config) -> Result<(), FatalError> {
    if !cfg.input_dir.is_dir() {
        return Err(FatalError::BadInputDir(cfg.input_dir.clone()));
    }
    // Output must not equal input, and input files must not live inside output.
    let in_canon = cfg.input_dir.canonicalize().ok();
    let out_canon = cfg.output_dir.canonicalize().ok();
    if let (Some(i), Some(o)) = (&in_canon, &out_canon)
        && i == o
    {
        return Err(FatalError::OutputOverlapsInput(cfg.output_dir.clone()));
    }
    if cfg.output_dir == cfg.input_dir {
        return Err(FatalError::OutputOverlapsInput(cfg.output_dir.clone()));
    }
    if !cfg.dry_run {
        std::fs::create_dir_all(&cfg.output_dir)
            .map_err(|e| FatalError::OutputDir(cfg.output_dir.clone(), e))?;
    }
    Ok(())
}

/// Read all discovered fonts, isolating per-font read failures.
fn scan(cfg: &Config) -> Result<(Vec<ScannedFont>, Vec<FontError>), FatalError> {
    let paths = discover_paths(cfg)?;
    let mut scanned = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        if is_unsupported_container(&ext) {
            failures.push(FontError::UnsupportedContainer(path));
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                failures.push(FontError::Io(path, e));
                continue;
            }
        };
        let font = match FontRef::new(&bytes) {
            Ok(f) => f,
            Err(e) => {
                failures.push(FontError::Parse(path, e.to_string()));
                continue;
            }
        };
        match read::read_signals(&font, &path) {
            Ok(signals) => scanned.push(ScannedFont {
                path,
                ext,
                bytes,
                signals,
            }),
            Err(e) => failures.push(e),
        }
    }
    Ok((scanned, failures))
}

/// Build family groupings from scanned signals (pass 1). Returns a map from each
/// scanned-font index to its grouping (shared via index lookup).
fn build_groupings(scanned: &[ScannedFont]) -> HashMap<usize, FamilyGrouping> {
    // Provisional per-file resolution (per-file mode) to learn family + facets.
    let mut groups: HashMap<String, (String, Vec<(usize, FamilyMember)>)> = HashMap::new();

    for (idx, sf) in scanned.iter().enumerate() {
        let res = resolve(&sf.signals, &ResolveContext::default());
        let fam = res.style.typographic_family.clone();
        let key = family_key(&strip_for_key(&fam));
        // Stable, intrinsic ordering key: original PostScript name, else filename stem.
        let sort_key = sf
            .signals
            .name_postscript_id6
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| sf.signals.filename_stem.clone());
        let member = FamilyMember {
            file_index: idx,
            sort_key,
            weight: res.style.weight,
            italic: res.style.italic,
            width: res.style.width,
        };
        groups
            .entry(key)
            .or_insert_with(|| (fam.clone(), Vec::new()))
            .1
            .push((idx, member));
    }

    let mut result = HashMap::new();
    for (_key, (family, members)) in groups {
        let member_list: Vec<FamilyMember> = members.iter().map(|(_, m)| m.clone()).collect();
        let grouping = FamilyGrouping::new(family, member_list);
        for (idx, _) in members {
            result.insert(idx, grouping.clone());
        }
    }
    result
}

fn strip_for_key(family: &str) -> String {
    style_words::strip_style_words(family)
}

/// Process the whole batch: scan, group, resolve+apply per font, write outputs.
pub fn run(cfg: &Config) -> Result<RunReport, FatalError> {
    prepare_output(cfg)?;

    let (scanned, mut failures) = scan(cfg)?;

    let groupings = if cfg.family_aware {
        build_groupings(&scanned)
    } else {
        HashMap::new()
    };

    let mut successes = Vec::new();
    let mut used_names: HashMap<PathBuf, usize> = HashMap::new();

    for (idx, sf) in scanned.iter().enumerate() {
        match process_one(cfg, sf, idx, &groupings, &mut used_names) {
            Ok(report) => successes.push(report),
            Err(e) => failures.push(e),
        }
    }

    Ok(RunReport {
        successes,
        failures,
        dry_run: cfg.dry_run,
    })
}

fn process_one(
    cfg: &Config,
    sf: &ScannedFont,
    idx: usize,
    groupings: &HashMap<usize, FamilyGrouping>,
    used_names: &mut HashMap<PathBuf, usize>,
) -> Result<FontReport, FontError> {
    let font =
        FontRef::new(&sf.bytes).map_err(|e| FontError::Parse(sf.path.clone(), e.to_string()))?;

    // Variable fonts are flattened to one static identity by this tool, which would
    // mislabel them; v1 copies them through unchanged and reports the skip (L2).
    if sf.signals.is_variable {
        return passthrough(cfg, sf, used_names, FontStatus::SkippedVariable);
    }

    let grouping = groupings.get(&idx);
    let ctx = ResolveContext {
        family: grouping,
        file_index: Some(idx),
        prefer: Default::default(),
        options: cfg.resolve_options,
    };
    let res = resolve(&sf.signals, &ctx);

    // Compute the output bytes. Unchanged fonts are copied through byte-for-byte (never
    // rebuilt — preserves DSIG, table order, checksums); changed fonts are rebuilt.
    let out_bytes = if res.changes.is_empty() {
        sf.bytes.clone()
    } else {
        write::apply(&font, &sf.signals, &res.style, &sf.path)?
    };

    // Determine output filename.
    let out_name = if cfg.rename {
        canonical_filename(&res.style, &sf.ext)
    } else {
        sf.path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("font")
            .to_string()
    };

    let (out_path, collided) = claim_out_path(cfg, &out_name, &sf.ext, &out_bytes, used_names);

    let renamed = cfg.rename
        && sf.path.file_name().and_then(|s| s.to_str())
            != out_path.file_name().and_then(|s| s.to_str());

    let status = if res.changes.is_empty() {
        FontStatus::AlreadyCanonical
    } else if renamed {
        FontStatus::Renamed
    } else {
        FontStatus::Normalized
    };

    let mut conflicts = res.conflicts;
    if collided {
        conflicts.push(crate::core::diff::Conflict::new(
            crate::core::diff::Field::Filename,
            format!(
                "output name collision; written as {}",
                out_path.file_name().and_then(|s| s.to_str()).unwrap_or("?")
            ),
        ));
    }

    if !cfg.dry_run {
        atomic_write(&out_path, &out_bytes)?;
    }

    let dsig_dropped = sf.signals.has_dsig && !res.changes.is_empty();
    Ok(FontReport {
        input: sf.path.clone(),
        output: out_path,
        status,
        changes: res.changes,
        conflicts,
        renamed,
        dsig_dropped,
    })
}

/// Resolve the final output path against (a) names already claimed this run and (b)
/// pre-existing files on disk. A foreign file in the output dir is never clobbered; a
/// byte-identical file we'd reproduce is left in place (keeps re-runs idempotent).
/// Returns the path and whether a dup-suffix was applied.
fn claim_out_path(
    cfg: &Config,
    out_name: &str,
    ext: &str,
    out_bytes: &[u8],
    used_names: &mut HashMap<PathBuf, usize>,
) -> (PathBuf, bool) {
    let stem = Path::new(out_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("font")
        .to_string();
    let ext = ext.to_ascii_lowercase();

    let mut out_path = cfg.output_dir.join(out_name);
    let mut dup = 1usize;
    loop {
        let claimed = used_names.contains_key(&out_path);
        let on_disk = out_path.exists();
        let disk_is_ours = on_disk
            && !claimed
            && std::fs::read(&out_path)
                .map(|d| d == out_bytes)
                .unwrap_or(false);
        if !claimed && (!on_disk || disk_is_ours) {
            break;
        }
        dup += 1;
        out_path = cfg.output_dir.join(format!("{stem}-dup{dup}.{ext}"));
    }
    used_names.insert(out_path.clone(), dup);
    (out_path, dup > 1)
}

/// Copy a font through unchanged (variable fonts), reporting the skip. Keeps the original
/// filename and collision-safe naming; never normalizes or renames.
fn passthrough(
    cfg: &Config,
    sf: &ScannedFont,
    used_names: &mut HashMap<PathBuf, usize>,
    status: FontStatus,
) -> Result<FontReport, FontError> {
    let out_name = sf
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("font")
        .to_string();
    let (out_path, _collided) = claim_out_path(cfg, &out_name, &sf.ext, &sf.bytes, used_names);
    if !cfg.dry_run {
        atomic_write(&out_path, &sf.bytes)?;
    }
    Ok(FontReport {
        input: sf.path.clone(),
        output: out_path,
        status,
        changes: Vec::new(),
        conflicts: Vec::new(),
        renamed: false,
        dsig_dropped: false,
    })
}

/// Write to a temp file in the same dir, then rename into place (atomic on same fs).
fn atomic_write(dest: &Path, bytes: &[u8]) -> Result<(), FontError> {
    let dir = dest.parent().unwrap_or_else(|| Path::new("."));
    let tmp_name = format!(
        ".{}.tmp",
        dest.file_name().and_then(|s| s.to_str()).unwrap_or("font")
    );
    let tmp = dir.join(tmp_name);
    std::fs::write(&tmp, bytes).map_err(|e| FontError::Io(tmp.clone(), e))?;
    std::fs::rename(&tmp, dest).map_err(|e| FontError::Io(dest.to_path_buf(), e))?;
    Ok(())
}
