use std::{
    collections::HashMap,
    env::current_dir,
    fmt, io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use path_clean::PathClean;
use tokio::fs;

#[derive(Debug, Clone, Default)]
pub struct AsyncFileCache {
    files: HashMap<PathBuf, String>,
}

impl AsyncFileCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }

    pub async fn update(&mut self, path: &Path) -> Result<Option<AsyncFileEvent>> {
        let path = match path.clean() {
            p if p.is_relative() => current_dir().expect("failed to get current dir").join(p),
            p => p,
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
