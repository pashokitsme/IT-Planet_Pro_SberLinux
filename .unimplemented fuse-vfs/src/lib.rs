mod fs;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicU64;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use fuser::MountOption;

pub type Inode = u64;
pub const BLOCK_SIZE: u64 = 512;

pub fn system_time_from_time(secs: i64, nsecs: u32) -> SystemTime {
  if secs >= 0 {
    UNIX_EPOCH + Duration::new(secs as u64, nsecs)
  } else {
    UNIX_EPOCH - Duration::new((-secs) as u64, nsecs)
  }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum FileKind {
  File,
  Directory,
}

impl From<FileKind> for fuser::FileType {
  fn from(kind: FileKind) -> Self {
    match kind {
      FileKind::File => fuser::FileType::RegularFile,
      FileKind::Directory => fuser::FileType::Directory,
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct InodeAttributes {
  pub inode: Inode,
  pub size: u64,
  pub kind: FileKind,
  // pub last_accessed: (i64, u32),
  // pub last_modified: (i64, u32),
  // pub last_metadata_changed: (i64, u32),
  // Permissions and special mode bits
  pub mode: u16,
  pub hardlinks: u32,
  pub uid: u32,
  pub gid: u32,
  // pub xattrs: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl From<InodeAttributes> for fuser::FileAttr {
  fn from(attrs: InodeAttributes) -> Self {
    let now = SystemTime::now();
    fuser::FileAttr {
      ino: attrs.inode,
      size: attrs.size,
      blocks: (attrs.size + BLOCK_SIZE - 1) / BLOCK_SIZE,
      atime: now, // system_time_from_time(attrs.last_accessed.0, attrs.last_accessed.1),
      mtime: now, //system_time_from_time(attrs.last_modified.0, attrs.last_modified.1),
      ctime: now, //system_time_from_time(attrs.last_metadata_changed.0, attrs.last_metadata_changed.1),
      crtime: SystemTime::UNIX_EPOCH,
      kind: attrs.kind.into(),
      perm: attrs.mode,
      nlink: attrs.hardlinks,
      uid: attrs.uid,
      gid: attrs.gid,
      rdev: 0,
      blksize: BLOCK_SIZE as u32,
      flags: 0,
    }
  }
}

impl InodeAttributes {
  pub fn new_file(inode: Inode, mode: u16) -> Self {
    Self { inode, size: 0, kind: FileKind::File, mode, hardlinks: 1, uid: 0, gid: 0 }
  }
}

pub struct Vfs {
  next_inode: AtomicU64,
  inodes: HashMap<Inode, InodeAttributes>,
}

impl Default for Vfs {
  fn default() -> Self {
    let inodes = HashMap::new();
    let mut vfs = Self { next_inode: AtomicU64::new(1), inodes };
    vfs.insert_inode(FileKind::Directory, 0o755);
    vfs
  }
}

impl Vfs {
  pub fn insert_inode(&mut self, kind: FileKind, mode: u16) -> InodeAttributes {
    let inode = self.next_inode.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let attrs = InodeAttributes { inode, size: 0, kind, mode, hardlinks: 1, uid: 0, gid: 0 };
    self.inodes.insert(inode, attrs);
    attrs
  }

  pub fn mount<P: AsRef<Path>>(mount_point: P) -> std::io::Result<()> {
    fuser::mount2(Vfs::default(), mount_point, &[MountOption::AutoUnmount, MountOption::AllowRoot])
  }
}
