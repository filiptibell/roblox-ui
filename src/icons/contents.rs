use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use futures::future::join_all;
use once_cell::sync::Lazy;
use rbx_reflection::ReflectionDatabase;
use serde::Serialize;
use tokio::fs;

static CLASS_DATABASE: Lazy<&ReflectionDatabase> = Lazy::new(rbx_reflection_database::get);
const CLASS_ICON_FALLBACKS: &[(&str, &[&str])] = &[("Package", &["PackageLink"])];
const METADATA_FILE_NAME: &str = "metadata.json";

type IconPackContentsMap = BTreeMap<PathBuf, Bytes>;

fn class_name_from_path(path: &Path) -> Result<&str> {
    let file_name = path
        .file_name()
        .context("missing file name")?
        .to_str()
        .context("non-utf8 file name")?;

    Ok(match path.extension().and_then(|e| e.to_str()) {
        Some("png") => file_name.trim_end_matches(".png"),
        Some("svg") => file_name.trim_end_matches(".svg"),
        Some(ext) => bail!("unknown file extension '{ext}'"),
        None => bail!("missing file extension"),
    })
}

fn class_is_a(instance_class: impl AsRef<str>, class_name: impl AsRef<str>) -> Option<bool> {
    let mut instance_class = instance_class.as_ref();
    let class_name = class_name.as_ref();

    if class_name == "Instance" || instance_class == class_name {
        Some(true)
    } else {
        let db = rbx_reflection_database::get();

        while instance_class != class_name {
            let class_descriptor = db.classes.get(instance_class)?;
            if let Some(sup) = &class_descriptor.superclass {
                instance_class = sup.borrow();
            } else {
                return Some(false);
            }
        }

        Some(true)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct IconPackContentsMetadata {
    class_count: usize,
    class_icons: BTreeMap<String, PathBuf>,
}

impl IconPackContentsMetadata {
    fn has_icon(&self, class_name: impl AsRef<str>) -> bool {
        let class_name = class_name.as_ref();
        self.class_icons.contains_key(class_name)
    }

    fn add_icon(
        &mut self,
        class_name: impl AsRef<str>,
        icon_path: impl Into<PathBuf>,
        allow_class_not_in_database: bool,
    ) {
        let class_name = class_name.as_ref();
        let icon_path = icon_path.into();

        let subclasses = CLASS_DATABASE
            .classes
            .values()
            .filter_map(|descriptor| {
                let subclass = descriptor.name.as_ref();
                if subclass != class_name && class_is_a(subclass, class_name).unwrap_or_default() {
                    Some(subclass)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for subclass in subclasses {
            if !self.class_icons.contains_key(subclass) {
                self.class_icons
                    .insert(subclass.to_string(), icon_path.to_path_buf());
            }
        }

        if allow_class_not_in_database || CLASS_DATABASE.classes.contains_key(class_name) {
            self.class_icons
                .insert(class_name.to_string(), icon_path.to_path_buf());
        }
    }

    fn generate(icon_pack_map: &IconPackContentsMap) -> Result<Bytes> {
        let mut metadata = IconPackContentsMetadata::default();

        for path in icon_pack_map.keys() {
            metadata.add_icon(class_name_from_path(path)?, path, false);
        }

        for (class_name, fallbacks) in CLASS_ICON_FALLBACKS {
            if metadata.has_icon(class_name) {
                continue;
            }
            for fallback in fallbacks.iter() {
                if let Some(path) = icon_pack_map.keys().find(|path| {
                    matches!(
                        class_name_from_path(path),
                        Ok(path_class) if &path_class == fallback
                    )
                }) {
                    metadata.add_icon(class_name, path, true);
                    break;
                }
            }
        }

        metadata.class_count = metadata.class_icons.len();

        let bytes = serde_json::to_string(&metadata)
            .context("failed to serialize metadata")?
            .as_bytes()
            .to_vec();

        Ok(Bytes::from(bytes))
    }
}

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

        let metadata_light = IconPackContentsMetadata::generate(&self.light)
            .context("failed to generate icon pack metadata (light)")?;
        let metadata_dark = IconPackContentsMetadata::generate(&self.dark)
            .context("failed to generate icon pack metadata (dark)")?;

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
