#![allow(dead_code)]

use std::{
    collections::HashMap,
    env::current_dir,
    fmt, io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use path_clean::PathClean;
use tokio::fs;

/**
    A simple file cache for async file watching.

    Performs cleanup and canonicalization of file paths and stores current contents of files.
*/
#[derive(Debug, Clone)]
pub struct AsyncFileCache {
    dir: PathBuf,
    files: HashMap<PathBuf, String>,
}

impl AsyncFileCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_file(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }

    pub async fn read_file_at(&mut self, path: &Path) -> Result<Option<AsyncFileEvent>> {
        let path = if path.is_relative() {
            self.dir.join(path).clean()
        } else {
            path.clean()
        };

        let prev = self.files.get(&path);
        let this = match fs::read_to_string(&path).await {
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => bail!("{e}"),
            Ok(v) => Some(v),
        };

        let event = AsyncFileEvent::new(prev.map(|s| s.as_str()), this.as_deref());

        match this {
            Some(v) => self.files.insert(path, v),
            None => self.files.remove(&path),
        };

        Ok(event)
    }
}

impl Default for AsyncFileCache {
    fn default() -> Self {
        Self {
            dir: current_dir().expect("failed to get current dir"),
            files: HashMap::new(),
        }
    }
}

/**
    A simplified event variant for file watching.

    Should be used exclusively by `AsyncFileCache`.
*/
#[derive(Debug, Clone, Copy)]
pub enum AsyncFileEvent {
    Created,
    Modified,
    Removed,
}

impl fmt::Display for AsyncFileEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Created => "Created",
            Self::Modified => "Modified",
            Self::Removed => "Removed",
        };
        s.fmt(f)
    }
}

impl AsyncFileEvent {
    pub const fn is_created(&self) -> bool {
        matches!(self, Self::Created)
    }

    pub const fn is_modified(&self) -> bool {
        matches!(self, Self::Modified)
    }

    pub const fn is_removed(&self) -> bool {
        matches!(self, Self::Removed)
    }

    fn new(prev: Option<&str>, this: Option<&str>) -> Option<Self> {
        match (prev, this) {
            (None, None) => None,
            (None, Some(_)) => Some(Self::Created),
            (Some(_), None) => Some(Self::Removed),
            (Some(p), Some(t)) => {
                if p != t {
                    Some(Self::Modified)
                } else {
                    None
                }
            }
        }
    }
}
