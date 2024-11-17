use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use anyhow::Context;

use git2::*;
use tracing::*;

use crate::watch::Event;

pub struct Repo(git2::Repository);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Status {
  Created,
  Modified,
  Deleted,
  Unmodified,
}

impl Repo {
  pub fn open_or_create<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let repo = if !path.as_ref().exists() {
      let mut opts = RepositoryInitOptions::new();
      opts.bare(true).mkdir(true).initial_head("master");
      info!("init new bare repo at {}", path.as_ref().display());
      Self(git2::Repository::init_opts(&path, &opts)?)
    } else {
      info!("open existing repo at {}", path.as_ref().display());
      Self(git2::Repository::open(path)?)
    };

    repo.ensure_head_exists()?;
    Ok(repo)
  }

  pub fn autosave(&self, paths: &[Event]) -> anyhow::Result<()> {
    const DIFF_ERROR_MESSAGE: &str = "failed to diff tree -> index";
    let tree = self.0.head()?.peel_to_tree()?;

    let mut index = self.0.index()?;
    for path in paths {
      self.add_from_path(&mut index, &path.dir.join(&path.path))?;
    }

    let diff = self.0.diff_tree_to_index(Some(&tree), Some(&index), None).context(DIFF_ERROR_MESSAGE)?;
    let delta_paths = diff
      .deltas()
      .filter(|d| d.status() != Delta::Unmodified)
      .map(|d| d.new_file().path().unwrap())
      .collect::<Vec<_>>();

    if delta_paths.is_empty() {
      info!("no changes to commit; aborting autosave");
      return Ok(());
    }

    info!("resolved deltas at paths: {:?}", delta_paths);

    let sig = self.creds()?;
    let parent = self.0.head()?.peel_to_commit().context("parent commit not found")?;
    let message = self.create_commit_message(&delta_paths);
    let commit = self.0.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent])?;

    let commit = self.0.find_commit(commit)?;

    if let Some(message) = commit.message() {
      info!("commit {} created with message:\n{}", commit.id(), message);
    } else {
      info!("commit {} created without message", commit.id());
    }

    Ok(())
  }

  fn add_from_path(&self, index: &mut Index, path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::MetadataExt;

    if !path.exists() {
      index.remove_path(path)?;
      info!("removed path from index: {}", path.display());
      return Ok(());
    }

    let stat = path.metadata()?;

    if stat.is_dir() {
      warn!("provided path is a directory: {}; only files are supported", path.display());
      return Ok(());
    }

    let created = stat.created()?.duration_since(UNIX_EPOCH)?.as_secs();
    let modified = stat.modified()?.duration_since(UNIX_EPOCH)?.as_secs();

    let data = std::fs::read(path)?;
    let blob_oid = self.0.blob(&data)?;

    #[allow(clippy::unnecessary_cast)]
    let entry = IndexEntry {
      ctime: IndexTime::new(created as i32, 0),
      mtime: IndexTime::new(modified as i32, 0),
      dev: stat.dev() as u32,
      ino: stat.ino() as u32,
      mode: stat.mode() as u32,
      uid: stat.uid() as u32,
      gid: stat.gid() as u32,
      file_size: stat.len() as u32,
      id: blob_oid,
      flags: 0,
      flags_extended: 0,
      path: path.strip_prefix("/").unwrap_or(path).as_os_str().as_encoded_bytes().to_vec(),
    };

    index.add_frombuffer(&entry, &data)?;
    info!("added path to index: {}; oid: {}", path.display(), blob_oid);
    Ok(())
  }

  fn create_commit_message(&self, paths: &[&Path]) -> String {
    let mut message = format!("Autosaving: {} files\n\nSaved files:\n", paths.len());
    paths.iter().for_each(|path| message.push_str(&format!("\t- {}\n", path.display())));
    message
  }

  fn ensure_head_exists(&self) -> anyhow::Result<()> {
    let Err(err) = self.0.head() else {
      return Ok(());
    };

    if err.code() == ErrorCode::UnbornBranch {
      warn!("branch is unborn; creating it");
      let sig = self.creds()?;
      let tree = self.0.treebuilder(None)?.write()?;
      self.0.commit(Some("HEAD"), &sig, &sig, "init", &self.0.find_tree(tree)?, &[])?;
      return Ok(());
    }

    Err(err.into())
  }

  fn creds(&self) -> anyhow::Result<Signature> {
    Ok(Signature::now("1", "1@email.com")?)
  }
}
