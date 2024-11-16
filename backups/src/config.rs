use std::path::Path;
use std::path::PathBuf;

use tracing::*;

use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
  pub journal_dir: PathBuf,
  pub tasks: Vec<BackupTaskConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackupTaskConfig {
  pub source: PathBuf,
  pub destination: PathBuf,
  pub on: BackupTriggerConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackupTriggerConfig {
  pub trigger: BackupTrigger,
  pub strategy: BackupStrategyConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type")]
pub enum BackupTrigger {
  Change,
  Schedule {
    #[serde(default = "schedule_default_every")]
    every: Vec<String>,
    at: Option<String>,
  },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BackupStrategyConfig {
  Incremental,
  Differential,
}

impl Config {
  pub fn example() -> Self {
    Config {
      journal_dir: PathBuf::from("/path/to/journal"),
      tasks: vec![
        BackupTaskConfig {
          source: PathBuf::from("/path/to/source"),
          destination: PathBuf::from("/path/to/destination"),
          on: BackupTriggerConfig {
            trigger: BackupTrigger::Change,
            strategy: BackupStrategyConfig::Incremental,
          },
        },
        BackupTaskConfig {
          source: PathBuf::from("/path/to/source2"),
          destination: PathBuf::from("/path/to/destination2"),
          on: BackupTriggerConfig {
            trigger: BackupTrigger::Schedule {
              every: vec!["1 day".to_string()],
              at: Some("00:00:00".to_string()),
            },
            strategy: BackupStrategyConfig::Incremental,
          },
        },
        BackupTaskConfig {
          source: PathBuf::from("/path/to/source3"),
          destination: PathBuf::from("/path/to/destination3"),
          on: BackupTriggerConfig {
            trigger: BackupTrigger::Schedule {
              every: vec!["1 day".to_string()],
              at: Some("00:00:00".to_string()),
            },
            strategy: BackupStrategyConfig::Differential,
          },
        },
      ],
    }
  }

  pub fn from_file(path: PathBuf, format: Option<String>) -> anyhow::Result<Self> {
    let format = ConfigFormat::from_ext_or_format(Some(&path), format);
    debug!("resolved format: {}", format);

    let content = std::fs::read_to_string(path)?;

    let config: Self = match format {
      ConfigFormat::Json => serde_json::from_str(&content)?,
      ConfigFormat::Yaml => serde_yaml::from_str(&content)?,
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
