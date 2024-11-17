use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::Error;
use notify_debouncer_full::DebouncedEvent;
use tracing::*;

use notify_debouncer_full::new_debouncer;

use crate::config::AppConfig;

pub struct Watchdog {
  config: AppConfig,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct Event {
  pub path: PathBuf,
  pub dir: Arc<PathBuf>,
}

struct EventInternal {
  ev: Result<Vec<DebouncedEvent>, Vec<Error>>,
  selected_patterns: Arc<Box<[String]>>,
  dir: Arc<PathBuf>,
}

impl Watchdog {
  pub fn new(config: AppConfig) -> Self {
    Self { config }
  }

  pub async fn watch_all(&self) -> anyhow::Result<tokio::sync::mpsc::UnboundedReceiver<Vec<Event>>> {
    let (ev_tx, ev_rx) = tokio::sync::mpsc::unbounded_channel();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let mut watchers = Vec::with_capacity(self.config.watch.len());

    for watch in self.config.watch.iter() {
      let tx = tx.clone();
      let selected_patterns = Arc::new(watch.patterns.clone().into_boxed_slice());
      let dir = Arc::new(dunce::canonicalize(&watch.dir).unwrap_or(watch.dir.to_path_buf()));
      let mut watcher = new_debouncer(Duration::from_secs(5), None, move |ev| {
        let selected_patterns = selected_patterns.clone();
        let dir = dir.clone();
        if let Err(err) = tx.send(EventInternal { ev, selected_patterns, dir }) {
          error!("failed to send event: {}", err);
        }
      })?;

      info!("watching: {:?}", watch.dir);
      watcher.watch(&watch.dir, notify::RecursiveMode::Recursive)?;
      watchers.push(watcher);
    }

    tokio::spawn(async move {
      while let Some(EventInternal { ev, selected_patterns, dir }) = rx.recv().await {
        let Ok(events) = ev else {
          error!("failed to receive event: {:?}", ev);
          continue;
        };

        debug!("received events: {:#?}", events);

        let mut unique_matched_paths = HashSet::new();

        for event in events {
          for path in event.paths.iter() {
            let canonicalized_path = dunce::canonicalize(path).unwrap_or(path.to_path_buf());
            let final_path = match canonicalized_path.strip_prefix(dir.as_ref()) {
              Ok(path) => path,
              Err(e) => {
                error!("failed to strip prefix of path: {}", e);
                continue;
              }
            };

            if !canonicalized_path.is_dir()
              && selected_patterns
                .iter()
                .any(|pat| glob_match::glob_match(pat, final_path.to_string_lossy().as_ref()))
            {
              unique_matched_paths.insert(Event { path: final_path.to_path_buf(), dir: dir.clone() });
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
      drop(watchers);
    });

    Ok(ev_rx)
  }
}
