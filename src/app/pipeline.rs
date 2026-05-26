use crate::dao::{row::DailyKwhRow, Dao};
use crate::decoder;
use crate::error::ProfileError;
use crate::profile::{first_row, Profile, Registry};
use crate::reader::{self, Format};
use crate::reshape;
use anyhow::Context;
use chrono::Local;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ProcessOutcome {
    pub rows: u64,
    pub profile_name: String,
    pub year_month: String,
    pub signature_drift: bool,
}

/// Decode → read → route → apply → dao(or dry-run).
pub async fn process_file(
    path: &Path,
    registry: &Registry,
    dao: &dyn Dao,
    dry_run: bool,
) -> anyhow::Result<ProcessOutcome> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("read {}", path.display()))?;

    let format = Format::from_path(path)
        .ok_or_else(|| anyhow::anyhow!("unsupported file: {}", path.display()))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();

    // Phase 1: peek header for routing (auto-encoding for safety on csv).
    let peek_decoder = decoder::from_label("auto");
    let peek_text = if format.needs_text_decode() {
        Some(
            peek_decoder
                .decode(&bytes)
                .with_context(|| "decode for routing peek")?,
        )
    } else {
        None
    };
    let reader = reader::make_reader(format);
    let peek_grid = reader
        .read(path, peek_text.as_deref(), None)
        .with_context(|| "read for routing peek")?;
    let header_row = first_row(&peek_grid);

    let m = registry.route(&filename, &header_row)?;
    let profile: Profile = m.profile.clone();
    let signature_drift = !m.signature_ok;
    if signature_drift {
        tracing::warn!(
            profile = %profile.name,
            expected = ?profile.match_rules.header_signature,
            actual = %m.actual_signature,
            "header signature drift — proceeding anyway"
        );
    }

    // Phase 2: full read with profile-specified encoding & sheet.
    let final_decoder = decoder::from_label(&profile.encoding);
    let final_text = if format.needs_text_decode() {
        Some(
            final_decoder
                .decode(&bytes)
                .with_context(|| format!("decode with {}", profile.encoding))?,
        )
    } else {
        None
    };
    let grid = reader
        .read(path, final_text.as_deref(), profile.sheet)
        .with_context(|| "read with profile encoding/sheet")?;

    let year_month = profile.extract_year_month(path)?;
    let rows = reshape::apply(&grid, &profile, &year_month)?;
    let row_count = rows.len() as u64;

    if dry_run {
        tracing::info!(
            file = %filename,
            profile = %profile.name,
            year_month = %year_month,
            rows = row_count,
            "dry-run preview"
        );
        log_preview(&rows);
        return Ok(ProcessOutcome {
            rows: row_count,
            profile_name: profile.name,
            year_month,
            signature_drift,
        });
    }

    let inserted = dao.upsert_month(&year_month, &rows).await?;
    Ok(ProcessOutcome {
        rows: inserted,
        profile_name: profile.name,
        year_month,
        signature_drift,
    })
}

fn log_preview(rows: &[DailyKwhRow]) {
    for (i, r) in rows.iter().take(5).enumerate() {
        tracing::info!(
            i,
            caseid = %r.caseid,
            plant_name = ?r.plant_name,
            shift_hour = %r.shift_hour,
            daily_kwh = ?r.daily_kwh,
            "preview row"
        );
    }
}

/// Move a successfully-imported file to backup_dir, suffixing the stem with today's date.
pub fn move_to_backup(src: &Path, backup_dir: &Path) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(backup_dir)?;
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("bad filename: {}", src.display()))?;
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("");
    let date = Local::now().format("%Y%m%d");
    let new_name = if ext.is_empty() {
        format!("{}_{}", stem, date)
    } else {
        format!("{}_{}.{}", stem, date, ext)
    };
    let dest = backup_dir.join(new_name);
    std::fs::rename(src, &dest).with_context(|| format!("rename {} -> {}", src.display(), dest.display()))?;
    Ok(dest)
}

/// Move a failing file to error_dir and drop an `<file>.err.txt` with the error chain.
pub fn move_to_error(src: &Path, error_dir: &Path, err: &anyhow::Error) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(error_dir)?;
    let filename = src
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("bad filename: {}", src.display()))?;
    let dest = error_dir.join(filename);
    std::fs::rename(src, &dest)
        .with_context(|| format!("rename {} -> {}", src.display(), dest.display()))?;
    let err_path = dest.with_extension(format!(
        "{}.err.txt",
        src.extension().and_then(|s| s.to_str()).unwrap_or("")
    ));
    let body = render_error_chain(err);
    std::fs::write(&err_path, body)?;
    Ok(dest)
}

fn render_error_chain(err: &anyhow::Error) -> String {
    let mut s = format!("{err}\n");
    let mut cur = err.source();
    let mut depth = 1usize;
    while let Some(e) = cur {
        use std::fmt::Write as _;
        let _ = writeln!(s, "  caused by [{depth}]: {e}");
        cur = e.source();
        depth += 1;
    }
    s
}

/// Classify routing errors so the caller can decide what to log.
pub fn is_no_match(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<ProfileError>(),
        Some(ProfileError::NoMatch { .. })
    )
}
