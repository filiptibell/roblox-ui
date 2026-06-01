use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_fs as fs;
use bytes::Bytes;
use futures::future::join_all;

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

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len_light() + self.len_dark()
    }

    /**
        The light-theme icon set (icon file path → bytes), for consumers that render icons directly
        from memory instead of writing the pack to disk first.
    */
    pub fn light(&self) -> &BTreeMap<PathBuf, Bytes> {
        &self.light
    }

    /**
        The dark-theme icon set (icon file path → bytes), for consumers that render icons directly
        from memory instead of writing the pack to disk first.
    */
    pub fn dark(&self) -> &BTreeMap<PathBuf, Bytes> {
        &self.dark
    }

    /**
        Resolves every Roblox class to its light-theme icon bytes, applying the **same subclass
        fallback chain** that [`write_to`](Self::write_to) bakes into `metadata.json` (via
        [`IconPackMetadata`]) — but entirely in memory. The returned map is keyed by class name, so
        e.g. `UICorner` resolves to the nearest ancestor that has an icon.
    */
    pub fn resolve_light(&self) -> Result<BTreeMap<String, Bytes>> {
        Self::resolve_map(&self.light)
    }

    /**
        Resolves every Roblox class to its dark-theme icon bytes, applying the **same subclass
        fallback chain** that [`write_to`](Self::write_to) bakes into `metadata.json` — in memory.
        See [`resolve_light`](Self::resolve_light).
    */
    pub fn resolve_dark(&self) -> Result<BTreeMap<String, Bytes>> {
        Self::resolve_map(&self.dark)
    }

    /**
        Build a `class name → icon bytes` map from a path→bytes set, expanding via the pack metadata
        so subclasses inherit an ancestor's icon (the in-memory equivalent of `write_to`'s metadata).
    */
    fn resolve_map(map: &IconPackContentsMap) -> Result<BTreeMap<String, Bytes>> {
        let paths = map.keys().map(|p| p.deref()).collect::<Vec<_>>();
        let metadata =
            IconPackMetadata::from_paths(&paths).context("failed to build icon pack metadata")?;

        let mut resolved = BTreeMap::new();
        for (class_name, icon_path) in metadata.class_icons {
            if let Some(bytes) = map.get(&icon_path) {
                resolved.insert(class_name, bytes.clone());
            }
        }

        Ok(resolved)
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

#[cfg(test)]
mod tests {
    use crate::IconPack;

    /**
        `resolve_dark` must apply the subclass fallback chain and pick the *nearest* iconed ancestor:
        `UICorner`/`UIScale`/`UIPadding`/`UIListLayout` have no own icon, so they must inherit
        `UIComponent`'s icon — NOT fall all the way back to the generic `Instance` icon.
    */
    #[test]
    fn resolve_dark_uses_nearest_iconed_ancestor() {
        let contents = futures_lite::future::block_on(IconPack::Vanilla2.get()).unwrap();
        let resolved = contents.resolve_dark().unwrap();

        let ui_component = resolved.get("UIComponent");
        let instance = resolved.get("Instance");
        assert!(ui_component.is_some(), "UIComponent ships its own icon");
        assert!(instance.is_some(), "Instance (root) has an icon");
        assert_ne!(
            ui_component, instance,
            "the UIComponent icon differs from the Instance icon"
        );

        for class in ["UICorner", "UIScale", "UIPadding", "UIListLayout"] {
            assert_eq!(
                resolved.get(class),
                ui_component,
                "{class} must inherit the nearest ancestor (UIComponent), not the Instance icon",
            );
        }
    }
}
