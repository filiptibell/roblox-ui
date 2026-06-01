/*!
    An async, recursive, debounced filesystem watcher over one or more roots,
    built on `notify-debouncer-full`. Emits every changed path; filtering is left
    to the sync engine.
*/

use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use async_channel::{unbounded, Receiver};
use notify_debouncer_full::{
    new_debouncer, notify::*, DebounceEventResult, Debouncer, RecommendedCache,
};
use tracing::error;

/**
    An async, recursive file watcher.

    Watches one or more source roots recursively and emits every changed path
    (created / modified / removed), debounced. Filtering of irrelevant paths
    (ignore globs, etc.) is left to the consumer / sync engine.

    Starts watching when constructed and stops when dropped.
*/
pub struct AsyncFileWatcher {
    // The debouncer must be kept alive to keep watching.
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    receiver: Receiver<PathBuf>,
}

impl AsyncFileWatcher {
    pub fn new(roots: Vec<PathBuf>) -> Result<Self> {
        let (tx, rx) = unbounded();

        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            None,
            move |result: DebounceEventResult| match result {
                Err(errors) => errors.iter().for_each(|e| error!("{e:?}")),
                Ok(events) => {
                    for event in events {
                        for path in &event.paths {
                            tx.try_send(path.clone()).ok();
                        }
                    }
                }
            },
        )?;

        for root in &roots {
            if let Err(e) = debouncer.watch(root, RecursiveMode::Recursive) {
                error!("failed to watch {}: {e:?}", root.display());
            }
        }

        Ok(Self {
            _debouncer: debouncer,
            receiver: rx,
        })
    }

    pub async fn recv(&mut self) -> Option<PathBuf> {
        self.receiver.recv().await.ok()
    }
}
