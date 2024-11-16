use std::path::Path;
use std::process::exit;

use fuse_vfs::Vfs;

fn setup_tracing() {
  tracing_subscriber::fmt::fmt().without_time().init();
}

fn main() {
  setup_tracing();

  let Some(path) = std::env::args().nth(1) else {
    tracing::error!(
      "Usage: {} <vfs path>",
      Path::new(&std::env::args().next().unwrap()).file_name().unwrap().to_str().unwrap()
    );
    exit(1)
  };

  let path = Path::new(&path).canonicalize().unwrap();

  if !path.exists() {
    tracing::error!("Path does not exist: {:?}", path);
    exit(2);
  }

  Vfs::mount(path).unwrap();
}
