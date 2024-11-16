mod backup;
mod config;

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
}

#[derive(Subcommand)]
enum Commands {
  /// Show the current config or create an example
  ShowConfig,
  /// Write an example config to stdout or to the path if specified
  ExampleConfig {
    /// Path to write the example config
    #[arg(short, long)]
    path: Option<PathBuf>,
    /// Force format of example config
    #[arg(short, long, value_parser = ["json", "yaml", "yml"])]
    format: Option<String>,
  },
  /// Start the program
  Start {
    #[arg(short, long)]
    config: Option<PathBuf>,
  },
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

fn setup_tracing() {
  tracing_subscriber::fmt::fmt().without_time().init();
}

#[tokio::main]
async fn main() {
  setup_tracing();

  let cli = Cli::parse();

  match cli.command {
    Commands::ShowConfig => {
      println!("Config command");
    }
    Commands::ExampleConfig { path, format } => {
      write_example_config(path, format).expect("Failed to write example config");
    }
    Commands::Start { config } => {
      println!("Start command");
    }
  }
}
