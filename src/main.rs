use clap::Parser;
use solar_monitoring_import as smi;
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = smi::cli::Cli::parse();
    let cfg = smi::config::load(&cli.config)?;
    let _log_guard = smi::log::init_log(&cfg.log)?;

    match cli.cmd {
        smi::cli::Cmd::Run {
            profile_dir,
            input_dir,
            dry_run,
        } => run_cmd(&cfg, profile_dir, input_dir, dry_run).await,
        smi::cli::Cmd::Learn {
            structure,
            sample,
            out,
            name,
        } => learn_cmd(structure, sample, out, name),
    }
}

async fn run_cmd(
    cfg: &smi::config::Config,
    profile_dir: Option<PathBuf>,
    input_dir: Option<PathBuf>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let pdir = profile_dir.as_deref().unwrap_or(&cfg.profile.dir);
    let registry = smi::profile::load_dir(pdir)?;
    if registry.is_empty() {
        tracing::warn!(dir = %pdir.display(), "no profiles loaded — nothing will match");
    }

    let dao: Box<dyn smi::dao::Dao> = if dry_run {
        Box::new(smi::dao::NoopDao)
    } else {
        smi::dao::make_dao(&cfg.database)?
    };
    let summary = smi::app::run(cfg, &registry, dao.as_ref(), input_dir.as_deref(), dry_run).await?;

    if summary.failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn learn_cmd(
    structure: PathBuf,
    sample: PathBuf,
    out: PathBuf,
    name: Option<String>,
) -> anyhow::Result<()> {
    let format = smi::reader::Format::from_path(&structure).ok_or_else(|| {
        anyhow::anyhow!("unsupported structure file extension: {}", structure.display())
    })?;
    let reader = smi::reader::make_reader(format);

    let read_one = |path: &std::path::Path| -> anyhow::Result<smi::grid::Grid> {
        let bytes = std::fs::read(path)?;
        let text = if format.needs_text_decode() {
            Some(smi::decoder::from_label("auto").decode(&bytes)?)
        } else {
            None
        };
        reader.read(path, text.as_deref(), None)
    };

    let structure_grid = read_one(&structure)?;
    let sample_grid = if sample == structure {
        None
    } else if sample.exists() {
        Some(read_one(&sample)?)
    } else {
        tracing::warn!(path = %sample.display(), "sample not found — using default value rules");
        None
    };

    let profile_name = name.unwrap_or_else(|| {
        structure
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });

    let profile = smi::profile::learn::learn(
        &structure_grid,
        sample_grid.as_ref(),
        &profile_name,
        format,
    );

    let json = serde_json::to_string_pretty(&profile)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, json)?;
    tracing::info!(out = %out.display(), profile = %profile.name, "profile written");
    Ok(())
}
