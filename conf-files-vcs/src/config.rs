use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use tracing::*;

use serde_derive::Deserialize;
use serde_derive::Serialize;

pub type AppConfig = Arc<Config>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
  pub(crate) repo: PathBuf,
  pub(crate) watch: Vec<WatchPath>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WatchPath {
  pub(crate) dir: PathBuf,
  pub(crate) patterns: Vec<String>,
}

impl Config {
  pub fn repo(&self) -> &Path {
    &self.repo
  }

  pub fn example() -> Self {
    Self {
      repo: PathBuf::from("./configs.git"),
      watch: vec![WatchPath { dir: PathBuf::from("./watch"), patterns: vec!["**/*".to_string()] }],
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

  pub fn resolve(config_path: Option<&Path>, format: Option<&str>) -> anyhow::Result<AppConfig> {
    const DEFAULT_CONFIG_FILENAMES: &[&str] = &["config.yaml", "config.yml", "config.json"];

    let config_path = match config_path {
      Some(path) if path.exists() => path.to_path_buf(),
      _ => match DEFAULT_CONFIG_FILENAMES.iter().find(|f| Path::new(f).exists()) {
        Some(path) => PathBuf::from(path),
        None => return Err(anyhow::anyhow!("no config file specified and no default config file found")),
      },
    };

    Self::from_file(config_path, format.map(|s| s.to_string())).map(Arc::new)
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
