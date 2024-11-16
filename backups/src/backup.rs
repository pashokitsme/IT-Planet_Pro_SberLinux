use crate::config::*;

pub fn make_backup(config: &BackupTaskConfig) -> anyhow::Result<()> {
  match config.on.strategy {
    BackupStrategyConfig::Incremental => incremental::make_incremental_backup(config),
    BackupStrategyConfig::Differential => differential::make_differential_backup(config),
  }
}

mod incremental {
  use std::path::Path;

  use super::BackupTaskConfig;
  use tracing::*;

  pub fn make_incremental_backup(config: &BackupTaskConfig) -> anyhow::Result<()> {
    if !config.src.exists() {
      anyhow::bail!("src directory does not exist: {}", config.src.display());
    }

    std::fs::create_dir_all(&config.dst)?;
    let span =
      info_span!("rm", src = config.src.display().to_string(), dst = config.dst.display().to_string());
    let _guard = span.enter();
    remove_unwanted_files_from_dst(&config.src, &config.dst)?;
    drop(_guard);
    let span =
      info_span!("cp", src = config.src.display().to_string(), dst = config.dst.display().to_string());
    let _guard = span.enter();
    copy_incremental_all(&config.src, &config.dst)?;
    drop(_guard);
    Ok(())
  }

  pub fn copy_incremental_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    let mut copied_count = 0;

    if src.is_dir() {
      std::fs::create_dir_all(dst)?;

      for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dst_path = dst.join(path.file_name().unwrap());
        let src_path = src.join(path.file_name().unwrap());

        if path.is_dir() {
          copy_incremental_all(&path, &dst_path)?;
        } else if !dst_path.exists() || dst_path.metadata()?.modified()? < src_path.metadata()?.modified()? {
          info!("copying {} to {}", path.display(), dst_path.display());
          std::fs::copy(&path, &dst_path)?;
          copied_count += 1;
        }
      }
    } else if !dst.exists() || dst.metadata()?.modified()? < src.metadata()?.modified()? {
      info!("copying {} to {}", src.display(), dst.display());
      std::fs::copy(src, dst)?;
      copied_count += 1;
    }

    if copied_count > 0 {
      info!("copied {} files", copied_count);
    } else {
      info!("no files copied, everything is up-to-date");
    }

    Ok(())
  }

  pub fn remove_unwanted_files_from_dst(src: &Path, dst: &Path) -> anyhow::Result<()> {
    let mut removed_count = 0;
    if src.is_dir() {
      std::fs::create_dir_all(dst)?;

      for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dst_path = dst.join(path.file_name().unwrap());
        let src_path = src.join(path.file_name().unwrap());

        if path.is_dir() {
          if dst_path.exists() && !src_path.exists() {
            std::fs::remove_dir_all(&dst_path)?;
          } else {
            remove_unwanted_files_from_dst(&path, &dst_path)?;
          }
        } else if dst_path.exists() && !src_path.exists() {
          std::fs::remove_file(&dst_path)?;
          removed_count += 1;
        }
      }
    } else if dst.exists() && !src.exists() {
      std::fs::remove_dir_all(dst)?;
      removed_count += 1;
    }

    if removed_count > 0 {
      info!("removed {} files", removed_count);
    } else {
      info!("no files removed, everything is up-to-date");
    }

    Ok(())
  }
}

mod differential {
  use std::path::Path;

  use super::BackupTaskConfig;
  use tracing::*;

  pub fn make_differential_backup(config: &BackupTaskConfig) -> anyhow::Result<()> {
    std::fs::create_dir_all(&config.dst)?;
    let temp_dir = tempfile::tempdir_in(
      config
        .dst
        .parent()
        .ok_or(std::io::Error::new(std::io::ErrorKind::InvalidInput, "dst has no parent"))?,
    )?;
    let temp_bak_dir = temp_dir.path();
    std::fs::create_dir_all(temp_bak_dir)?;

    let span = info_span!("tmp", path = temp_bak_dir.display().to_string());
    let _guard = span.enter();
    info!("temp dir path: {}", temp_bak_dir.display());
    copy_all(&config.src, temp_bak_dir)?;
    drop(_guard);
    let span =
      info_span!("mv", src = temp_bak_dir.display().to_string(), dst = config.dst.display().to_string());
    let _guard = span.enter();
    info!("remove old backup");
    std::fs::remove_dir_all(&config.dst)?;
    info!("moving temp dir to dst");
    std::fs::rename(temp_bak_dir, &config.dst)?;
    drop(_guard);

    Ok(())
  }

  fn copy_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    let mut copied_count = 0;
    if src.is_dir() {
      std::fs::create_dir_all(dst)?;

      for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dst_path = dst.join(path.file_name().unwrap());

        if path.is_dir() {
          copy_all(&path, &dst_path)?;
        } else {
          std::fs::copy(&path, &dst_path)?;
          copied_count += 1;
        }
      }
    } else {
      std::fs::copy(src, dst)?;
      copied_count += 1;
    }

    info!("copied {} files", copied_count);

    Ok(())
  }
}
