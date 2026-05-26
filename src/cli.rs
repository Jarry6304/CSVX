use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "import", version, about = "Solar monitoring monthly importer")]
pub struct Cli {
    #[arg(long, default_value = "config.toml", global = true)]
    pub config: PathBuf,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Process files in input_dir using profiles in profile_dir.
    Run {
        #[arg(long)]
        profile_dir: Option<PathBuf>,

        #[arg(long)]
        input_dir: Option<PathBuf>,

        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Generate a profile.json from a template + sample.
    Learn {
        #[arg(long)]
        structure: PathBuf,

        #[arg(long)]
        sample: PathBuf,

        #[arg(long)]
        out: PathBuf,

        #[arg(long)]
        name: Option<String>,
    },
}
