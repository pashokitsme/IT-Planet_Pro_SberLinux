use std::collections::HashSet;
use std::hash::Hasher;
use std::path::PathBuf;
use std::time::Duration;

use tracing::*;

use notify_debouncer_full::new_debouncer;

pub struct Watchdog {}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Event {
  path: PathBuf,
}

impl std::hash::Hash for Event {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.path.hash(state);
  }
}

impl Watchdog {
  pub async fn watch(&self) -> anyhow::Result<tokio::sync::mpsc::UnboundedReceiver<Vec<Event>>> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let (ev_tx, ev_rx) = tokio::sync::mpsc::unbounded_channel();

    let mut watcher = new_debouncer(Duration::from_secs(2), None, move |ev| {
      if let Err(err) = tx.send(ev) {
        error!("failed to send event: {}", err);
      }
    })?;

    let dir = "./watch";
    let pattern = "**/*";

    for path in [PathBuf::from(dir)]
      .into_iter()
      .chain(glob::glob(&format!("{}/{}", dir, pattern))?.filter_map(|p| p.ok()).filter(|p| p.is_dir()))
    {
      info!("watching: {:?}", path);
      watcher.watch(path, notify::RecursiveMode::Recursive)?;
    }

    tokio::spawn(async move {
      while let Some(ev) = rx.recv().await {
        let Ok(events) = ev else {
          error!("failed to receive event: {:?}", ev);
          continue;
        };

        debug!("received events: {:#?}", events);

        let mut unique_matched_paths = HashSet::new();

        for event in events {
          for path in event.paths.iter() {
            if !path.is_dir() && glob_match::glob_match(pattern, path.to_string_lossy().as_ref()) {
              unique_matched_paths.insert(Event { path: path.to_path_buf() });
            }
          }
        }

        if unique_matched_paths.is_empty() {
          info!("got an event, but no matched paths; skipping");
          continue;
        }

        match ev_tx.send(unique_matched_paths.into_iter().collect()) {
          Ok(_) => debug!("event sent successfully"),
          Err(e) => error!("failed to send event: {}", e),
        }
      }
      drop(watcher);
    });

    Ok(ev_rx)
  }
}
