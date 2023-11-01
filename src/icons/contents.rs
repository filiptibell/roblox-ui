use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bytes::Bytes;
use futures::future::join_all;
use tokio::fs;

use super::*;

const METADATA_FILE_NAME: &str = "metadata.json";

type IconPackContentsMap = BTreeMap<PathBuf, Bytes>;

#[derive(Debug, Clone, Default)]
pub struct IconPackContents {
    light: IconPackContentsMap,
    dark: IconPackContentsMap,
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

        Each subdirectory will also contain a `metadata.json`
        file containing additional data about the icon pack.
    */
    pub async fn write_to(&self, dir: impl AsRef<Path>) -> Result<()> {
        let dir = dir.as_ref();

        let dir_light = dir.join("light");
        let dir_dark = dir.join("dark");

        fs::remove_dir_all(&dir_light).await.ok();
        fs::remove_dir_all(&dir_dark).await.ok();

        fs::create_dir_all(&dir_light).await?;
        fs::create_dir_all(&dir_dark).await?;

        let paths_light = self.light.keys().map(|p| p.deref()).collect::<Vec<_>>();
        let paths_dark = self.dark.keys().map(|p| p.deref()).collect::<Vec<_>>();

        let metadata_light = IconPackMetadata::from_paths(&paths_light)
            .context("failed to generate icon pack metadata (light)")?
            .serialize_bytes()
            .context("failed to serialize icon pack metadata (light)")?;
        let metadata_dark = IconPackMetadata::from_paths(&paths_dark)
            .context("failed to generate icon pack metadata (dark)")?
            .serialize_bytes()
            .context("failed to serialize icon pack metadata (dark)")?;

        let mut all_futs = vec![
            fs::write(dir_light.join(METADATA_FILE_NAME), metadata_light.as_ref()),
            fs::write(dir_dark.join(METADATA_FILE_NAME), metadata_dark.as_ref()),
        ];

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
