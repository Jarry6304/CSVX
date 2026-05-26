use crate::config::LogCfg;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_log(cfg: &LogCfg) -> anyhow::Result<WorkerGuard> {
    std::fs::create_dir_all(&cfg.dir)?;
    let appender = tracing_appender::rolling::daily(&cfg.dir, "import.log");
    let (file_writer, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(cfg.level.clone()));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_ansi(true))
        .with(fmt::layer().with_ansi(false).with_writer(file_writer))
        .init();

    Ok(guard)
}
