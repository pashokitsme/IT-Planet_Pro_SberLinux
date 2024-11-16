mod backup;
mod config;

use std::path::Path;
use std::path::PathBuf;

use tracing::*;

use clap::*;
use clap_derive::*;

use config::ConfigFormat;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,

  /// Path to config
  #[arg(short, long)]
  config: Option<PathBuf>,

  /// Force format of example config
  #[arg(short, long, value_parser = ["json", "yaml", "yml"])]
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
  /// Start the program
  Start,
}

fn write_example_config(path: Option<PathBuf>, format: Option<String>) -> anyhow::Result<()> {
  let config = config::Config::example();

  let format = ConfigFormat::from_ext_or_format(path.as_ref(), format);

  debug!("resolved format: {}", format);

  let data = match format {
    ConfigFormat::Json => serde_json::to_string_pretty(&config)?,
    ConfigFormat::Yaml => serde_yaml::to_string(&config)?,
  };

  if let Some(path) = path {
    info!("writing example config to {}; format: {}", path.display(), format);
    std::fs::write(path, data)?;
  } else {
    info!("writing example config to stdout; output format: {}", format);
    println!("{}", data);
  }

  Ok(())
}

async fn start(config_path: Option<PathBuf>, format: Option<String>) -> anyhow::Result<()> {
  let config = config::Config::resolve(config_path, format)?;

  Ok(())
}

#[tokio::main]
async fn main() {
  color_eyre::install().expect("failed to install color_eyre");
  setup_tracing();

  let cli = Cli::parse();

  let Cli { config, format, .. } = cli;

  match cli.command {
    Commands::ShowConfig { example } => {
      if example {
        write_example_config(config, format).expect("Failed to create example config");
      } else {
        let config = config::Config::resolve(config, format).expect("Failed to resolve config");
        println!("{:#?}", config);
      }
    }
    Commands::Start => {
      start(config, format).await.expect("Failed to run application");
    }
  }
}

fn setup_tracing() {
  tracing_subscriber::fmt::fmt().without_time().init();
}
