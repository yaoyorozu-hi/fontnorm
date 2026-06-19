# fontnorm

A pure-Rust CLI that normalizes and corrects font metadata. It reads a directory of
`.ttf`/`.otf` fonts, resolves each font's conflicting identity/style signals into one
coherent style, rewrites the metadata tables to agree, renames the file to a canonical
form, and writes the result to an output subfolder. **Input files are never modified.**

It exists to fix the class of bug where a renderer mis-renders a font because its
metadata tells contradictory stories — e.g. a Bold face whose PANOSE weight says "Book",
so an e-reader renders body text with the bold file. The renderer trusts metadata, not
the design; fontnorm makes every identity field tell the same story.

## What it does

For each font, fontnorm reads the `name`, `OS/2`, `head`, and `post` tables plus measured
glyph advance widths, then:

- Resolves the intended **weight**, **slope (italic/oblique)**, **width**, **monospace**,
  and **family/subfamily** from all available signals, in a fixed priority order
  (typographic name → usWeightClass → style bits → filename → PANOSE).
- Rewrites the metadata so every field is internally consistent (see invariants below).
- Generates canonical `name` IDs 1/2/4/6/16/17 (Windows 3/1/0x409, plus a consistent Mac
  1/0/0 record when the font already ships one). All other name records — copyright,
  version, license, unique ID — pass through untouched.
- Renames the output file to `<Family>-<StyleToken>.<ext>` (e.g. `Yrsa-BoldItalic.ttf`).
- Drops any `DSIG` (a metadata edit invalidates a signature) and recomputes checksums.
- Leaves all outline data (`glyf`/`loca`/`CFF `) **byte-identical** — outlines are never
  parsed or re-serialized.

Outline preservation is structural: only the four metadata tables are recompiled; every
other table is copied through as raw bytes. This is what makes the tool safe on CFF/`.otf`
fonts, which have no pure-Rust outline writer.

## Usage

```
fontnorm <INPUT_DIR> [OPTIONS]

Arguments:
  <INPUT_DIR>              Directory containing fonts (non-recursive by default)

Options:
  -o, --output <SUBDIR>    Output subfolder under INPUT_DIR        [default: normalized]
      --out-dir <PATH>     Explicit output directory (may be outside INPUT_DIR)
  -n, --dry-run            Resolve and report changes, write nothing
  -r, --recursive          Recurse into subdirectories
      --no-rename          Keep original filenames; fix embedded metadata only
      --no-family          Disable whole-family analysis; per-file resolution only
      --no-monospace       Skip monospace measurement/correction
      --use-typo-metrics   Force fsSelection USE_TYPO_METRICS (bit 7) on (never clears it)
  -v, --verbose...         Increase verbosity (-v shows already-canonical fonts)
  -q, --quiet              Errors only
```

Examples:

```sh
# Preview what would change, without writing anything:
fontnorm ~/fonts --dry-run

# Normalize into ~/fonts/normalized/ with canonical filenames:
fontnorm ~/fonts

# Fix metadata but keep the original filenames, write elsewhere:
fontnorm ~/fonts --no-rename --out-dir ~/fonts-fixed
```

Example output:

```
  fix   KoboBroken-Bold.ttf  ->  LiberationSans-Bold.ttf  (1 change)
          OS/2.panose.bWeight : 5 -> 8
          conflict[OS/2.panose.bWeight]: panose.bWeight=5 corroborates differently than resolved Bold
  fix   Edmondsans-Regular.otf  (3 changes)
          name.ID1 (Family) : Edmondsans Regular -> Edmondsans
          name.ID4 (Full) : Edmondsans-Regular -> Edmondsans
          name.ID16 (Typo Family) : Edmondsans -> (removed)

normalized 2 font(s), 0 already canonical, 0 skipped.
```

Exit codes: `0` success (including dry-run and runs where some fonts were skipped),
`1` fatal setup error (bad input dir, unwritable/overlapping output), `2` usage error.

## Safety guarantees

- **Inputs are read-only.** All output goes to the output directory; the tool refuses to
  run if the output directory equals or overlaps the input.
- **Per-font failure isolation.** A corrupt or unsupported font is skipped and reported;
  it never aborts the batch.
- **Idempotent.** Running fontnorm a second time on its own output reports zero changes
  and produces byte-identical files. A font already in canonical form is copied through
  byte-for-byte rather than rebuilt.
- **Atomic writes.** Each output is written to a temp file and renamed into place.

## Normalization invariants enforced

| | Rule |
|---|---|
| I1 | `fsSelection.ITALIC` ⇔ `macStyle.Italic` ⇔ name says "Italic" |
| I2 | Oblique sets `fsSelection.OBLIQUE`; oblique implies italic style-linking |
| W1 | `fsSelection.BOLD` ⇔ `macStyle.Bold` |
| W2 | Bold ⇔ subfamily "Bold"/"Bold Italic" and usWeightClass 700 |
| W3 | usWeightClass ⇔ weight word in the typographic name (100…900 ladder) |
| W4 | Legacy usWeightClass 250→Thin(100), 275→ExtraLight(200) |
| R1 | `fsSelection.REGULAR` set ⇒ ITALIC and BOLD both clear |
| R2 | A face occupies exactly one RIBBI slot (Regular/Bold/Italic/Bold Italic) |
| N1 | name ID2 ∈ {Regular, Bold, Italic, Bold Italic}; non-RIBBI weights split into their own ID1 family with ID16/17 carrying the true family/subfamily |
| N2 | ID4 (full name) drops a trailing "Regular" |
| N3 | ID6 (PostScript) sanitized: ASCII, ≤63 chars, no `[](){}<>/%` or spaces |
| N4 | Within an ID1 RIBBI group, the (Bold, Italic) slot is unique per face |
| M1 | Measured-monospaced ⇔ `post.isFixedPitch` ≠ 0 ⇔ `panose.bProportion` = 9 |

Monospace is determined by measuring glyph advance widths (fontbakery's 80%-of-printable-
ASCII-share-one-width rule), gated on Latin presence so CJK fonts are never misclassified.

PANOSE `bWeight` is treated as corroborating, not authoritative: it is corrected only when
it genuinely misleads — unset, off by two or more rungs, or on the wrong side of the
Bold boundary (the exact Kobo bug). Harmless adjacent-rung values (e.g. Book vs Medium) are
left untouched. Disagreements are always reported even when not corrected.

## Resolution priority

When signals conflict, each style facet is resolved independently by walking this order
until a confident value is found:

1. Typographic name (ID17 else ID2) — human-authored intent
2. `OS/2.usWeightClass`
3. `fsSelection` / `macStyle` style bits
4. Filename heuristics (`*-BoldItalic`, `*-SemiBold`)
5. PANOSE (corroborating only)

Monospace is the exception: the *measured* advance widths are authoritative over metadata —
but only for **Latin-primary** fonts. A CJK/Kana/Hangul font whose full-width Latin glyphs
all share one advance is gated out of the monospace decision entirely (its `isFixedPitch`
and PANOSE proportion are preserved, never changed).

**Family renames require positive evidence.** A trailing style word is only stripped from a
family name when independent metadata confirms it: a weight word must match the resolved
`usWeightClass`/BOLD bit, a width word must match `usWidthClass`, a slope word must match the
italic bits. So "Open Sans Bold" with `usWeightClass=700` becomes family "Open Sans", but
"Archivo Black" with `usWeightClass=400` keeps its family — the "Black" is part of the real
name, not a redundant descriptor.

fontnorm is whole-family aware. It scans the directory first to group families, because the
RIBBI-vs-typographic split (whether a SemiBold becomes its own ID1 family) cannot be decided
from a single file. Family context is advisory and degrades gracefully to per-file
resolution; `--no-family` forces pure per-file mode. Two faces that resolve to the same RIBBI
slot within a family are given distinct PostScript names (ID6 must be unique) and the
collision is reported.

## v1 scope and limitations

**In scope:** raw SFNT `.ttf` (glyf) and `.otf` (CFF) fonts; name IDs 1/2/4/6/16/17;
usWeightClass (incl. legacy 250/275), usWidthClass, fsSelection, PANOSE bWeight/bProportion,
macStyle, post.isFixedPitch; cross-field invariant enforcement; monospace measurement;
filename normalization; DSIG drop; whole-family-aware resolution; per-font error isolation.

**Deferred (not in v1):**

- **WOFF / WOFF2** — only raw `.ttf`/`.otf`. WOFF2 needs the glyf-transform reversal, the
  largest residual risk; deferred to a later milestone.
- **`.ttc` / `.otc`** font collections — skipped and reported.
- **Variable fonts** (`fvar` present) — copied through unchanged and reported as skipped,
  because flattening a variable font to one static identity would mislabel it.
- **Full family-graph analysis** — WWS axis detection, name IDs 21/22, optical-size and
  mixed width+weight+optical families. v1 handles ≤4-RIBBI families and clean per-weight
  splits; pathological families resolve per-file with a warning.
- **Vertical metrics** (ascender/descender/lineGap, `USE_TYPO_METRICS` is preserved but
  the metric values are not adjusted).
- **Config files** and a `--prefer` priority-reorder flag (the machinery exists; v1 ships
  the default order only).

**Known limitations:**

- **Reserved `fsSelection` bits 10–15** are normalized to 0. The underlying `read-fonts`
  flag type truncates unknown bits before they reach the tool, so they cannot be preserved.
  Setting them to 0 is spec-conformant (the spec requires reserved bits to be 0).
- **Mac (1/0/0) name records** are emitted only for MacRoman-encodable identity strings.
  A non-MacRoman family name (CJK, some accents) ships its identity in the Windows
  3/1/0x409 record only; the legacy Mac record is dropped for that name ID.

## Library stack

Pure Rust, no C bindings, no FFI, no `-sys` crates:

- [`read-fonts`](https://crates.io/crates/read-fonts) (0.40) — zero-copy parser
- [`write-fonts`](https://crates.io/crates/write-fonts) (0.49) — owned tables + `FontBuilder`
- [`font-types`](https://crates.io/crates/font-types) (0.12)
- `clap`, `walkdir`, `thiserror`, `log`, `env_logger`

All metadata-only edits go through `write-fonts`' `FontBuilder`, which recompiles only the
changed tables and copies everything else through as raw bytes.

## License

MIT OR Apache-2.0.
```

Test fixtures under `tests/fixtures/` are third-party fonts used only for testing:
Liberation Sans (OFL), DejaVu (permissive), Edmondsans (losttype). They are not part of
the distributed tool.
```
