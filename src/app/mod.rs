pub mod pipeline;
pub mod scan;

use crate::config::Config;
use crate::dao::Dao;
use crate::profile::Registry;
use std::path::Path;

#[derive(Debug, Default)]
pub struct RunSummary {
    pub processed: u64,
    pub rejected_no_match: u64,
    pub failed: u64,
    pub rows_total: u64,
}

pub async fn run(
    cfg: &Config,
    registry: &Registry,
    dao: &dyn Dao,
    input_override: Option<&Path>,
    dry_run: bool,
) -> anyhow::Result<RunSummary> {
    let input_dir = input_override.unwrap_or(&cfg.source.input_dir);
    tracing::info!(
        input = %input_dir.display(),
        backup = %cfg.source.backup_dir.display(),
        error = %cfg.source.error_dir.display(),
        profiles = registry.len(),
        dry_run,
        "run starting"
    );

    let files = scan::list_input_files(input_dir)?;
    if files.is_empty() {
        tracing::info!("no input files found");
        return Ok(RunSummary::default());
    }

    let mut summary = RunSummary::default();
    for path in &files {
        let span = tracing::info_span!("file", path = %path.display());
        let _g = span.enter();
        match pipeline::process_file(path, registry, dao, dry_run).await {
            Ok(out) => {
                summary.processed += 1;
                summary.rows_total += out.rows;
                tracing::info!(
                    profile = %out.profile_name,
                    year_month = %out.year_month,
                    rows = out.rows,
                    drift = out.signature_drift,
                    "file done"
                );
                if !dry_run {
                    match pipeline::move_to_backup(path, &cfg.source.backup_dir) {
                        Ok(dest) => tracing::info!(dest = %dest.display(), "moved to backup"),
                        Err(e) => tracing::error!(error = %e, "backup move failed"),
                    }
                }
            }
            Err(e) => {
                if pipeline::is_no_match(&e) {
                    summary.rejected_no_match += 1;
                    tracing::warn!(error = %e, "no profile matched — rejecting");
                } else {
                    summary.failed += 1;
                    tracing::error!(error = %e, "processing failed");
                }
                if !dry_run {
                    if let Err(move_err) = pipeline::move_to_error(path, &cfg.source.error_dir, &e) {
                        tracing::error!(error = %move_err, "error move failed");
                    }
                }
            }
        }
    }

    tracing::info!(
        processed = summary.processed,
        rejected = summary.rejected_no_match,
        failed = summary.failed,
        rows_total = summary.rows_total,
        "run done"
    );
    Ok(summary)
}
