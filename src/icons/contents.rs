use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bytes::Bytes;
use futures::future::join_all;
use tokio::fs;

#[derive(Debug, Clone, Default)]
pub struct IconPackContents {
    light: HashMap<PathBuf, Bytes>,
    dark: HashMap<PathBuf, Bytes>,
}

impl IconPackContents {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len_light(&self) -> usize {
        self.light.len()
    }

    pub fn len_dark(&self) -> usize {
        self.dark.len()
    }

    pub fn len(&self) -> usize {
        self.len_light() + self.len_dark()
    }

    /**
        Inserts the given icon into the light icon set.
    */
    pub fn insert_icon_light<P, C>(&mut self, path: P, contents: C)
    where
        P: Into<PathBuf>,
        C: Into<Bytes>,
    {
        self.light.insert(path.into(), contents.into());
    }

    /**
        Inserts the given icon into the dark icon set.
    */
    pub fn insert_icon_dark<P, C>(&mut self, path: P, contents: C)
    where
        P: Into<PathBuf>,
        C: Into<Bytes>,
    {
        self.dark.insert(path.into(), contents.into());
    }

    /**
        Inserts the given icon into ***both*** the light and dark icon sets.
    */
    pub fn insert_icon<P, C>(&mut self, path: P, contents: C)
    where
        P: Into<PathBuf> + Clone,
        C: Into<Bytes> + Clone,
    {
        self.insert_icon_light(path.clone(), contents.clone());
        self.insert_icon_dark(path.clone(), contents.clone());
    }

    /**
        Writes all of the icon to the given directory.

        This will create subdirectories with their respective icon set contents:

        - `light`
        - `dark`
    */
    pub async fn write_to(&self, dir: impl AsRef<Path>) -> Result<()> {
        let dir = dir.as_ref();

        let dir_light = dir.join("light");
        let dir_dark = dir.join("dark");

        fs::remove_dir_all(&dir_light).await.ok();
        fs::remove_dir_all(&dir_dark).await.ok();

        fs::create_dir_all(&dir_light).await?;
        fs::create_dir_all(&dir_dark).await?;

        let mut all_futs = Vec::new();
        for (path, contents) in &self.light {
            all_futs.push(fs::write(dir_light.join(path), contents.as_ref()));
        }
        for (path, contents) in &self.dark {
            all_futs.push(fs::write(dir_dark.join(path), contents.as_ref()));
        }

        for result in join_all(all_futs).await {
            result.context("failed to write icon file")?;
        }

        Ok(())
    }
}
