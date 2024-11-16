use std::path::PathBuf;
use std::thread;

use tracing::*;

use clap::*;
use clap_derive::*;

use backups::config;
use backups::scheduler;

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
  /// Start the program
  Start,
}

fn write_example_config(path: Option<PathBuf>, format: Option<String>) -> anyhow::Result<()> {
  let config = config::Config::example();
  let format = config::ConfigFormat::from_ext_or_format(path.as_ref(), format);
  debug!("resolved format: {}", format);

  let data = match format {
    config::ConfigFormat::Json => serde_json::to_string_pretty(&config)?,
    config::ConfigFormat::Yaml => serde_yaml::to_string(&config)?,
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
  scheduler::run_backup_tasks(config).await?;
  Ok(())
}

#[tokio::main]
async fn real_main() -> anyhow::Result<()> {
  let cli = Cli::parse();

  let Cli { config, format, .. } = cli;

  match cli.command {
    Commands::ShowConfig { example } => {
      if example {
        write_example_config(config, format)?;
      } else {
        let config = config::Config::resolve(config, format)?;
        println!("{:#?}", config);
      }
    }
    Commands::Start => {
      start(config, format).await?;
    }
  }

  loop {
    thread::park();
  }
}

fn main() {
  color_eyre::install().expect("failed to install color_eyre");
  setup_tracing();

  match real_main() {
    Ok(_) => (),
    Err(e) => {
      error!("failed to run program: {}", e);
      std::process::exit(1);
    }
  }
}

fn setup_tracing() {
  tracing_subscriber::fmt::fmt().pretty().init();
}
