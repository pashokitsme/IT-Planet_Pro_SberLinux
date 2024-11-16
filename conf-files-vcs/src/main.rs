use std::path::PathBuf;

use conf_files_vcs::config::*;
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
}

impl Commands {
  async fn run(&self, cli: &Cli) -> anyhow::Result<()> {
    match self {
      Self::ShowConfig { example, .. } if *example => self.write_example_config(cli),
      Self::ShowConfig { .. } => self.show_config(cli),
      Self::Watch => self.watch(cli).await,
    }
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
    let config = Config::resolve(cli.config.as_deref(), cli.format.as_deref())?;
    let watcher = Watchdog::new(config);
    let mut rx = watcher.watch_all().await?;
    while let Some(events) = rx.recv().await {
      info!("event: {:?}", events);
    }
    Ok(())
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
      error!("Error: {}", e);
      std::process::exit(1);
    }
  }
}

fn setup_tracing() {
  tracing_subscriber::fmt::fmt().without_time().init();
}
