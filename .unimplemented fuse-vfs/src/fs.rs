use std::ffi::c_int;
use std::ffi::OsStr;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use fuser::FileAttr;
use fuser::FileType;
use fuser::ReplyAttr;
use fuser::ReplyData;
use fuser::ReplyDirectory;
use fuser::ReplyEntry;
use fuser::Request;
use tracing::*;

use fuser::Filesystem;

use crate::Vfs;

const ENOSYS: c_int = 38;
const ENOENT: c_int = 2;

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
  ino: 1,
  size: 0,
  blocks: 0,
  atime: UNIX_EPOCH, // 1970-01-01 00:00:00
  mtime: UNIX_EPOCH,
  ctime: UNIX_EPOCH,
  crtime: UNIX_EPOCH,
  kind: FileType::Directory,
  perm: 0o755,
  nlink: 2,
  uid: 501,
  gid: 20,
  rdev: 0,
  flags: 0,
  blksize: 512,
};

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
  ino: 2,
  size: 13,
  blocks: 1,
  atime: UNIX_EPOCH, // 1970-01-01 00:00:00
  mtime: UNIX_EPOCH,
  ctime: UNIX_EPOCH,
  crtime: UNIX_EPOCH,
  kind: FileType::RegularFile,
  perm: 0o644,
  nlink: 1,
  uid: 501,
  gid: 20,
  rdev: 0,
  flags: 0,
  blksize: 512,
};

impl Filesystem for Vfs {
  fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
    if parent == 1 && name.to_str() == Some("hello.txt") {
      reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
    } else {
      reply.error(ENOENT);
    }
  }

  fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
    match ino {
      1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
      2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
      _ => reply.error(ENOENT),
    }
  }

  fn read(
    &mut self,
    _req: &Request,
    ino: u64,
    _fh: u64,
    offset: i64,
    _size: u32,
    _flags: i32,
    _lock: Option<u64>,
    reply: ReplyData,
  ) {
    if ino == 2 {
      reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
    } else {
      reply.error(ENOENT);
    }
  }

  fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
    dbg!(ino);
    if ino != 1 {
      reply.error(ENOENT);
      return;
    }

    let entries = vec![
      (1, FileType::Directory, "."),
      (1, FileType::Directory, ".."),
      (2, FileType::RegularFile, "hello.txt"),
    ];

    for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
      // i + 1 means the index of the next entry
      if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
        break;
      }
    }
    reply.ok();
  }
}
