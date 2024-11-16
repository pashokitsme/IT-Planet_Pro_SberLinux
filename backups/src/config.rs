use std::path::Path;
use std::path::PathBuf;

use tracing::*;

use serde_derive::Deserialize;
use serde_derive::Serialize;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
  pub(crate) journal_dir: PathBuf,
  pub(crate) backups: Vec<BackupConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackupConfig {
  pub(crate) source: PathBuf,
  pub(crate) destination: PathBuf,
  pub(crate) on: BackupTriggerConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackupTriggerConfig {
  pub(crate) trigger: BackupTrigger,
  pub(crate) strategy: BackupStrategy,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BackupTrigger {
  Change,
  Schedule,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BackupStrategy {
  Incremental,
  Differential,
  Full,
}

impl Config {
  pub fn example() -> Self {
    Config {
      journal_dir: PathBuf::from("/path/to/journal"),
      backups: vec![
        BackupConfig {
          source: PathBuf::from("/path/to/source"),
          destination: PathBuf::from("/path/to/destination"),
          on: BackupTriggerConfig { trigger: BackupTrigger::Change, strategy: BackupStrategy::Incremental },
        },
        BackupConfig {
          source: PathBuf::from("/path/to/source2"),
          destination: PathBuf::from("/path/to/destination2"),
          on: BackupTriggerConfig { trigger: BackupTrigger::Schedule, strategy: BackupStrategy::Full },
        },
        BackupConfig {
          source: PathBuf::from("/path/to/source3"),
          destination: PathBuf::from("/path/to/destination3"),
          on: BackupTriggerConfig {
            trigger: BackupTrigger::Schedule,
            strategy: BackupStrategy::Differential,
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
}
