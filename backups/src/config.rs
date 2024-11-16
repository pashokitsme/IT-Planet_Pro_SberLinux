use std::path::Path;
use std::path::PathBuf;

use color_eyre::owo_colors::OwoColorize;
use tracing::*;

use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
  pub tasks: Vec<BackupTaskConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackupTaskConfig {
  pub src: PathBuf,
  pub dst: PathBuf,
  pub on: BackupTriggerConfig,
}

impl std::fmt::Display for BackupTaskConfig {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "\ttask `{}` -> `{}`; on: {}", self.src.display().bold(), self.dst.display().bold(), self.on)
  }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackupTriggerConfig {
  pub trigger: BackupTrigger,
  pub strategy: BackupStrategyConfig,
}

impl std::fmt::Display for BackupTriggerConfig {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "strategy: {}; trigger: {}", self.strategy, self.trigger)
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type")]
pub enum BackupTrigger {
  // Change,
  Schedule {
    #[serde(default = "schedule_default_every")]
    every: Vec<String>,
    at: Option<String>,
  },
}

impl std::fmt::Display for BackupTrigger {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BackupTrigger::Schedule { every, at } => {
        write!(
          f,
          "every: {}; at: {}",
          every.join(", ").bold(),
          at.as_deref().unwrap_or("<not specified>").bold()
        )
      }
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BackupStrategyConfig {
  Incremental,
  Differential,
}

impl std::fmt::Display for BackupStrategyConfig {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BackupStrategyConfig::Incremental => write!(f, "inc"),
      BackupStrategyConfig::Differential => write!(f, "diff"),
    }
  }
}

impl std::fmt::Display for Config {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for task in &self.tasks {
      write!(f, "{}", task)?;
    }
    Ok(())
  }
}

impl Config {
  pub fn example() -> Self {
    Config {
      tasks: vec![BackupTaskConfig {
        src: PathBuf::from("/src"),
        dst: PathBuf::from("/dst"),
        on: BackupTriggerConfig {
          trigger: BackupTrigger::Schedule { every: vec!["10 seconds".to_string()], at: None },
          strategy: BackupStrategyConfig::Incremental,
        },
      }],
    }
  }

  pub fn from_file(path: PathBuf, format: Option<String>) -> anyhow::Result<Self> {
    let format = ConfigFormat::from_ext_or_format(Some(&path), format);
    debug!("resolved format: {}", format);

    let content = std::fs::read_to_string(path)?;

    let config: Self = match format {
      ConfigFormat::Json => serde_json::from_str(&content)?,
      ConfigFormat::Yaml => serde_yml::from_str(&content)?,
    };

    debug!("config: {:#?}", config);

    Ok(config)
  }

  pub fn resolve(config_path: Option<PathBuf>, format: Option<String>) -> anyhow::Result<Self> {
    const DEFAULT_CONFIG_FILENAMES: &[&str] = &["config.yaml", "config.yml", "config.json"];

    let config_path = match config_path {
      Some(path) if path.exists() => path,
      _ => match DEFAULT_CONFIG_FILENAMES.iter().find(|f| Path::new(f).exists()) {
        Some(path) => PathBuf::from(path),
        None => return Err(anyhow::anyhow!("no config file specified and no default config file found")),
      },
    };

    Self::from_file(config_path, format)
  }
}

#[derive(Default, Clone, Debug)]
pub enum ConfigFormat {
  Json,
  #[default]
  Yaml,
}

impl std::fmt::Display for ConfigFormat {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ConfigFormat::Json => write!(f, "json"),
      ConfigFormat::Yaml => write!(f, "yaml"),
    }
  }
}

impl From<Option<String>> for ConfigFormat {
  fn from(s: Option<String>) -> Self {
    s.and_then(ConfigFormat::from_ext).unwrap_or_default()
  }
}

impl From<String> for ConfigFormat {
  fn from(s: String) -> Self {
    ConfigFormat::from_ext(&s).unwrap_or_default()
  }
}

impl ConfigFormat {
  pub fn from_ext<P: AsRef<Path>>(s: P) -> Option<Self> {
    match s.as_ref().extension().and_then(std::ffi::OsStr::to_str) {
      Some("json") => Some(ConfigFormat::Json),
      Some("yaml") | Some("yml") => Some(ConfigFormat::Yaml),
      _ => None,
    }
  }

  pub fn from_ext_or_format<P: AsRef<Path>>(s: Option<P>, or: Option<String>) -> Self {
    s.and_then(Self::from_ext).unwrap_or(Self::from(or.unwrap_or_default()))
  }
}

fn schedule_default_every() -> Vec<String> {
  vec!["1 day".to_string()]
}
