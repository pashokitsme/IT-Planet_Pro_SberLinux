use backups::backup::*;
use backups::config::*;

#[test]
fn differential_backup() {
  let temp_dir = tempfile::tempdir().unwrap();

  let src = temp_dir.path().join("src");
  let dst = temp_dir.path().join("dst");

  std::fs::create_dir_all(&src).unwrap();
  std::fs::create_dir_all(&dst).unwrap();

  let config = BackupTaskConfig {
    source: src.clone(),
    destination: dst.clone(),
    on: BackupTriggerConfig {
      trigger: BackupTrigger::Schedule {
        every: vec!["1 second".to_string()],
        at: Some("00:00:00".to_string()),
      },
      strategy: BackupStrategyConfig::Differential,
    },
  };

  std::fs::write(src.join("file1"), "content1").unwrap();
  std::fs::write(src.join("file2"), "content2").unwrap();
  std::fs::create_dir(src.join("dir1")).unwrap();
  std::fs::write(src.join("dir1/file3"), "content3").unwrap();

  make_backup(&config).unwrap();

  assert_eq!(std::fs::read_to_string(dst.join("file1")).unwrap(), "content1");
  assert_eq!(std::fs::read_to_string(dst.join("file2")).unwrap(), "content2");
  assert_eq!(std::fs::read_to_string(dst.join("dir1/file3")).unwrap(), "content3");

  std::fs::write(src.join("file2"), "content2_modified").unwrap();
  std::fs::remove_file(src.join("dir1/file3")).unwrap();

  make_backup(&config).unwrap();

  assert_eq!(std::fs::read_to_string(dst.join("file1")).unwrap(), "content1");
  assert_eq!(std::fs::read_to_string(dst.join("file2")).unwrap(), "content2_modified");
  assert!(dst.join("dir1").exists());
  assert!(!dst.join("dir1/file3").exists());
}
