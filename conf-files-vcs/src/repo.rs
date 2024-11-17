use core::str;
use std::path::Path;
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

struct FmtDelta<'a>(&'a Diff<'a>, &'a DiffDelta<'a>);
struct FmtDeltaStatus<'a>(&'a Delta);

impl<'a> std::fmt::Display for FmtDeltaStatus<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use owo_colors::OwoColorize;

    match self.0 {
      Delta::Added => write!(f, "{}", "added".green()),
      Delta::Deleted => write!(f, "{}", "deleted".red()),
      Delta::Modified => write!(f, "{}", "modified".yellow()),
      Delta::Unmodified => write!(f, "{}", "unmodified".dimmed()),
      Delta::Renamed => write!(f, "{}", "renamed".blue()),
      Delta::Copied => write!(f, "{}", "copied".blue()),
      Delta::Ignored => write!(f, "{}", "ignored".dimmed()),
      Delta::Unreadable => write!(f, "{}", "unreadable".dimmed()),
      Delta::Typechange => write!(f, "{}", "typechange".dimmed()),
      Delta::Untracked => write!(f, "{}", "untracked".dimmed()),
      Delta::Conflicted => write!(f, "{}", "conflicted".bright_red()),
    }
  }
}

impl<'a> std::fmt::Display for FmtDelta<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use owo_colors::OwoColorize;

    writeln!(f)?;
    writeln!(
      f,
      "File: {}\nStatus: {}",
      self.1.new_file().path().map(|p| p.display().to_string()).unwrap_or("<unknown>".to_string()).bold(),
      FmtDeltaStatus(&self.1.status()).bold()
    )?;

    let res = self.0.print(DiffFormat::Patch, |_, _, line| {
      use git2::DiffLineType::*;

      match line.origin_value() {
        Addition => write!(f, "{} {}", "+".bright_green(), str::from_utf8(line.content()).unwrap().green()),
        AddEOFNL => writeln!(f, "{} {}", "+".bright_green(), str::from_utf8(line.content()).unwrap().green()),
        Deletion => write!(f, "{} {}", "-".bright_red(), str::from_utf8(line.content()).unwrap().red()),
        DeleteEOFNL => writeln!(f, "{} {}", "-".bright_red(), str::from_utf8(line.content()).unwrap().red()),
        Context => write!(f, "  {}", str::from_utf8(line.content()).unwrap()),
        ContextEOFNL => writeln!(f, "  {}", str::from_utf8(line.content()).unwrap()),
        HunkHeader => writeln!(f, "{}", str::from_utf8(line.content()).unwrap().dimmed()),
        _ => Ok(()),
      }
      .unwrap();
      true
    });

    if let Err(e) = res {
      error!("failed to display diff: {}", e);
    }

    Ok(())
  }
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
    let index_tree = self.0.find_tree(index.write_tree()?)?;
    let commit = self.0.commit(Some("HEAD"), &sig, &sig, &message, &index_tree, &[&parent])?;

    let commit = self.0.find_commit(commit)?;

    if let Some(message) = commit.message() {
      info!("commit {} created:\ncommiter: {}\nmessage:\n{}", commit.id(), commit.committer(), message);
    } else {
      info!("commit {} created without message", commit.id());
    }

    Ok(())
  }

  pub fn show_diff(&self, path: &Path, id: Oid) -> anyhow::Result<()> {
    info!("showing diff head -> {} for path: {}", id, path.display());
    let path = dunce::canonicalize(path)?;
    let path = path.strip_prefix("/").unwrap_or(&path);
    let prev = self.0.find_commit(id)?.tree()?;
    let now = self.0.head()?.peel_to_tree()?;

    let mut diff_opts = DiffOptions::new();
    diff_opts.context_lines(u32::MAX).pathspec(path);
    let mut diff = self.0.diff_tree_to_tree(Some(&prev), Some(&now), Some(&mut diff_opts))?;

    let mut diff_find_opts = DiffFindOptions::new();
    diff_find_opts.renames(true);
    diff.find_similar(Some(&mut diff_find_opts))?;

    let delta = diff.deltas().next().context("no deltas found")?;
    println!("{}", FmtDelta(&diff, &delta));
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

    #[allow(clippy::unnecessary_cast)]
    let entry = IndexEntry {
      ctime: IndexTime::new(created as i32, 0),
      mtime: IndexTime::new(modified as i32, 0),
      dev: stat.dev() as u32,
      ino: stat.ino() as u32,
      mode: stat.mode() as u32,
      uid: stat.uid() as u32,
      gid: stat.gid() as u32,
      file_size: 0,
      id: Oid::zero(),
      flags: 0,
      flags_extended: 0,
      path: path.strip_prefix("/").unwrap_or(path).as_os_str().as_encoded_bytes().to_vec(),
    };

    index.add_frombuffer(&entry, &data)?;
    info!("added path to index: {}", path.display());
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
    let name = whoami::username();
    let devicename = whoami::fallible::hostname().unwrap_or("localhost".to_string());
    Ok(Signature::now(&name, &format!("{}@{}", name, devicename))?)
  }
}
