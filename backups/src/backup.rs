use color_eyre::eyre::ContextCompat;
use color_eyre::eyre::OptionExt;
use tracing::*;

use crate::config::*;

pub fn make_backup(config: &BackupTaskConfig) -> anyhow::Result<()> {
  match config.on.strategy {
    BackupStrategyConfig::Incremental => make_incremental_backup(config),
    BackupStrategyConfig::Differential => make_differential_backup(config),
  }
}

fn make_incremental_backup(config: &BackupTaskConfig) -> anyhow::Result<()> {
  Ok(())
}

fn make_differential_backup(config: &BackupTaskConfig) -> anyhow::Result<()> {
  std::fs::create_dir_all(&config.destination)?;
  let temp_dir = tempfile::tempdir_in(
    config
      .destination
      .parent()
      .ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "destination has no parent"))?,
  )?;
  let temp_bak_dir = temp_dir.path();
  std::fs::create_dir_all(temp_bak_dir)?;

  let span = info_span!("tempdir", path = temp_bak_dir.display().to_string());
  let _guard = span.enter();
  info!("temp dir path: {}", temp_bak_dir.display());
  copy_recursively(&config.source, temp_bak_dir)?;
  drop(_guard);
  let span = info_span!(
    "mv",
    src = temp_bak_dir.display().to_string(),
    dst = config.destination.display().to_string()
  );
  let _guard = span.enter();
  info!("remove old backup");
  std::fs::remove_dir_all(&config.destination)?;
  info!("moving temp dir to destination");
  std::fs::rename(temp_bak_dir, &config.destination)?;
  drop(_guard);

  Ok(())
}

fn copy_recursively(source: &std::path::Path, dest: &std::path::Path) -> anyhow::Result<()> {
  if source.is_dir() {
    std::fs::create_dir_all(dest)?;

    for entry in std::fs::read_dir(source)? {
      let entry = entry?;
      let path = entry.path();
      let dest_path = dest.join(path.file_name().unwrap());

      if path.is_dir() {
        copy_recursively(&path, &dest_path)?;
      } else {
        std::fs::copy(&path, &dest_path)?;
      }
    }
  } else {
    std::fs::copy(source, dest)?;
  }

  Ok(())
}
