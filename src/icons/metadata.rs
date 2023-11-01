use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use once_cell::sync::Lazy;
use rbx_reflection::ReflectionDatabase;
use serde::Serialize;

static CLASS_DATABASE: Lazy<&ReflectionDatabase> = Lazy::new(rbx_reflection_database::get);
const CLASS_ICON_FALLBACKS: &[(&str, &[&str])] = &[("Package", &["PackageLink"])];

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
pub struct IconPackMetadata {
    class_count: usize,
    class_icons: BTreeMap<String, PathBuf>,
}

impl IconPackMetadata {
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

    pub fn generate_from(icon_pack_map: &BTreeMap<PathBuf, Bytes>) -> Result<Self> {
        let mut metadata = IconPackMetadata::default();

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

        Ok(metadata)
    }

    pub fn serialize_bytes(&self) -> Result<Bytes> {
        let bytes = serde_json::to_string(self)?;
        Ok(Bytes::from(bytes))
    }
}
