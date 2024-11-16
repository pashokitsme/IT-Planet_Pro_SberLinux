use std::path::PathBuf;

use conf_files_vcs::watch::Watchdog;
use tracing::*;

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
}

#[tokio::main]
async fn real_main() -> anyhow::Result<()> {
  let watcher = Watchdog {};
  let mut rx = watcher.watch().await?;

  while let Some(events) = rx.recv().await {
    info!("event: {:?}", events);
  }

  Ok(())
}

fn main() {
  setup_tracing();
  match real_main() {
    Ok(_) => (),
    Err(e) => {
      error!("Error: {}", e);
      std::process::exit(1);
    }
  }
}

fn setup_tracing() {
  tracing_subscriber::fmt::fmt().without_time().init();
}
