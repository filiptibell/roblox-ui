use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tracing::error;

use notify_debouncer_full::{
    new_debouncer, notify::*, DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap,
};

fn is_matching_path(path: &Path, relevant_paths: &[PathBuf]) -> bool {
    let file_name = path.file_name().and_then(|f| f.to_str());
    let file_name = match file_name {
        None => return false,
        Some(f) => f,
    };
    relevant_paths.iter().any(|relevant_path| {
        let rfile_name = relevant_path.file_name().and_then(|f| f.to_str());
        if let Some(rfile_name) = rfile_name {
            rfile_name == file_name
        } else {
            false
        }
    })
}

fn matching_paths(event: &DebouncedEvent, relevant_paths: &[PathBuf]) -> Vec<PathBuf> {
    event
        .paths
        .iter()
        .filter_map(|p| {
            if is_matching_path(p, relevant_paths) {
                Some(p.to_path_buf())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

pub struct AsyncFileWatcher {
    // NOTE: We can't drop the debouncer since it would then stop watching,
    // so we keep it in the same struct that the consumer gets events from
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
    receiver: UnboundedReceiver<PathBuf>,
}

impl AsyncFileWatcher {
    pub fn new(relevant_paths: Vec<PathBuf>) -> Result<Self> {
        let (tx, rx) = unbounded_channel();

        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            None,
            move |result: DebounceEventResult| match result {
                Err(errors) => errors.iter().for_each(|e| error!("{e:?}")),
                Ok(events) => {
                    for event in events {
                        for path in matching_paths(&event, &relevant_paths) {
                            tx.send(path).unwrap()
                        }
                    }
                }
            },
        )?;

        debouncer
            .watcher()
            .watch(Path::new("."), RecursiveMode::Recursive)?;

        Ok(Self {
            _debouncer: debouncer,
            receiver: rx,
        })
    }

    pub async fn recv(&mut self) -> Option<PathBuf> {
        self.receiver.recv().await
    }
}
