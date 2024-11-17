use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use conf_files_vcs::config::*;
use conf_files_vcs::repo::Repo;
use conf_files_vcs::watch::Watchdog;

use tracing::*;

use clap::*;
use clap_derive::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,

  /// Path to config
  #[arg(short, long, global = true)]
  config: Option<PathBuf>,

  /// Force format of example config
  #[arg(short, long, value_parser = ["json", "yaml", "yml"], global = true)]
  format: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
  /// Show the current config or create an example one
  ShowConfig {
    /// Create an example config; Provide a path to write it to
    #[arg(long)]
    example: bool,
  },
  /// Watch for changes
  Watch,
  /// See the difference between files
  Diff {
    /// Path to the file to diff assuming the file was already autosaved
    #[arg()]
    path: PathBuf,
    /// Commit id to diff file with
    #[arg(long)]
    id: String,
  },
  /// Show the commit logs
  Log {
    /// Path to the file to log
    #[arg()]
    path: Option<PathBuf>,
  },
  /// Reset file to previous state
  Reset {
    /// Path to the file to reset
    #[arg()]
    path: PathBuf,
    /// Commit id to reset file to
    #[arg(long)]
    id: String,
  },
}

impl Commands {
  async fn run(&self, cli: &Cli) -> anyhow::Result<()> {
    match self {
      Self::ShowConfig { example, .. } if *example => self.write_example_config(cli),
      Self::ShowConfig { .. } => self.show_config(cli),
      Self::Watch => self.watch(cli).await,
      Self::Diff { path, id } => self.diff(cli, path, id),
      Self::Log { path } => self.log(cli, path.as_deref()),
      Self::Reset { path, id } => self.reset(cli, path, id),
    }
  }

  fn diff(&self, cli: &Cli, path: &Path, id: &str) -> anyhow::Result<()> {
    let (_, repo) = self.open_repo(cli)?;
    let oid = git2::Oid::from_str(id).context("provided an invalid commit id")?;
    repo.show_diff(path, oid)
  }

  fn log(&self, cli: &Cli, path: Option<&Path>) -> anyhow::Result<()> {
    let (_, repo) = self.open_repo(cli)?;
    repo.log(path)
  }

  fn reset(&self, cli: &Cli, path: &Path, id: &str) -> anyhow::Result<()> {
    let (_, repo) = self.open_repo(cli)?;
    let oid = git2::Oid::from_str(id).context("provided an invalid commit id")?;
    repo.reset(path, oid)
  }

  fn show_config(&self, cli: &Cli) -> anyhow::Result<()> {
    let config = Config::resolve(cli.config.as_deref(), cli.format.as_deref())?;
    println!("{:#?}", config);
    Ok(())
  }

  fn write_example_config(&self, cli: &Cli) -> anyhow::Result<()> {
    let config = Config::example();
    let format = ConfigFormat::from_ext_or_format(cli.config.as_ref(), cli.format.clone());
    debug!("resolved format: {}", format);

    let data = match format {
      ConfigFormat::Json => serde_json::to_string_pretty(&config)?,
      ConfigFormat::Yaml => serde_yml::to_string(&config)?,
    };

    if let Some(path) = cli.config.as_ref() {
      info!("writing example config to {}; format: {}", path.display(), format);
      std::fs::write(path, data)?;
    } else {
      info!("writing example config to stdout; output format: {}", format);
      println!("{}", data);
    }

    Ok(())
  }

  async fn watch(&self, cli: &Cli) -> anyhow::Result<()> {
    let (config, repo) = self.open_repo(cli)?;
    let watcher = Watchdog::new(config);
    let mut rx = watcher.watch_all().await?;
    while let Some(events) = rx.recv().await {
      info!("event: {:?}", events);
      match repo.autosave(&events) {
        Ok(_) => info!("autosaved"),
        Err(e) => error!("failed to autosave: {}", e),
      }
    }
    Ok(())
  }

  fn open_repo(&self, cli: &Cli) -> anyhow::Result<(AppConfig, Repo)> {
    const REPO_INIT_ERROR: &str = "failed to open or create repo; perhaps you need to clone again or delete it by yourself and let the program to reinit it?";
    let config = Config::resolve(cli.config.as_deref(), cli.format.as_deref())?;
    let repo = Repo::open_or_create(config.repo()).context(REPO_INIT_ERROR)?;
    Ok((config, repo))
  }
}

#[tokio::main]
async fn real_main() -> anyhow::Result<()> {
  let cli = Cli::parse();
  cli.command.run(&cli).await
}

fn main() {
  setup_tracing();
  match real_main() {
    Ok(_) => (),
    Err(e) => {
      if let Some(source) = e.source() {
        error!("{}", source);
      }
      error!("{}", e);
      std::process::exit(1);
    }
  }
}

fn setup_tracing() {
  tracing_subscriber::fmt::fmt().without_time().init();
}
