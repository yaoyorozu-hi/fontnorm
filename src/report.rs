use std::path::PathBuf;

use crate::core::diff::{Conflict, FieldChange};
use crate::error::FontError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontStatus {
    Normalized,
    AlreadyCanonical,
    Renamed,
    /// Variable font: copied through unchanged, not normalized (v1 deferral).
    SkippedVariable,
}

#[derive(Debug)]
pub struct FontReport {
    pub input: PathBuf,
    pub output: PathBuf,
    pub status: FontStatus,
    pub changes: Vec<FieldChange>,
    pub conflicts: Vec<Conflict>,
    pub renamed: bool,
    /// A DSIG was present and was dropped because metadata changed (risk register R1).
    pub dsig_dropped: bool,
}

#[derive(Debug, Default)]
pub struct RunReport {
    pub successes: Vec<FontReport>,
    pub failures: Vec<FontError>,
    pub dry_run: bool,
}

impl RunReport {
    pub fn normalized_count(&self) -> usize {
        self.successes
            .iter()
            .filter(|r| r.status == FontStatus::Normalized || r.status == FontStatus::Renamed)
            .count()
    }
    pub fn unchanged_count(&self) -> usize {
        self.successes
            .iter()
            .filter(|r| r.status == FontStatus::AlreadyCanonical)
            .count()
    }
    pub fn skipped_variable_count(&self) -> usize {
        self.successes
            .iter()
            .filter(|r| r.status == FontStatus::SkippedVariable)
            .count()
    }

    /// Render a human-readable report to a string.
    pub fn render(&self, verbosity: u8) -> String {
        let mut out = String::new();
        for r in &self.successes {
            let in_name = r.input.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            let out_name = r.output.file_name().and_then(|s| s.to_str()).unwrap_or("?");

            match r.status {
                FontStatus::AlreadyCanonical => {
                    if verbosity >= 1 {
                        out.push_str(&format!("  ok    {in_name}  (already canonical)\n"));
                    }
                    continue;
                }
                FontStatus::SkippedVariable => {
                    out.push_str(&format!(
                        "  skip  {in_name}  (variable font, copied through unchanged)\n"
                    ));
                    continue;
                }
                FontStatus::Normalized | FontStatus::Renamed => {
                    let rename_note = if r.renamed && in_name != out_name {
                        format!("  ->  {out_name}")
                    } else {
                        String::new()
                    };
                    out.push_str(&format!(
                        "  fix   {in_name}{rename_note}  ({} change{})\n",
                        r.changes.len(),
                        if r.changes.len() == 1 { "" } else { "s" }
                    ));
                }
            }

            for c in &r.changes {
                out.push_str(&format!(
                    "          {} : {} -> {}\n",
                    c.field.label(),
                    c.before,
                    c.after
                ));
            }
            for cf in &r.conflicts {
                out.push_str(&format!(
                    "          conflict[{}]: {}\n",
                    cf.field.label(),
                    cf.detail
                ));
            }
            if r.dsig_dropped {
                out.push_str("          DSIG removed (metadata edited)\n");
            }
        }

        for f in &self.failures {
            let name = f.path().file_name().and_then(|s| s.to_str()).unwrap_or("?");
            out.push_str(&format!("  skip  {name}  ({f})\n"));
        }

        let verb = if self.dry_run {
            "would normalize"
        } else {
            "normalized"
        };
        let skipped = self.failures.len() + self.skipped_variable_count();
        out.push_str(&format!(
            "\n{} {} font(s), {} already canonical, {} skipped.\n",
            verb,
            self.normalized_count(),
            self.unchanged_count(),
            skipped
        ));
        out
    }
}
