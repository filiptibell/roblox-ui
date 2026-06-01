use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result};
use async_fs as fs;

use super::modern::Modern;
use super::*;

/**
    The icons the user's Studio is configured to show: the custom pack set via Studio's
    `IconOverrideDir` setting if one is configured, otherwise the `Modern` default.
*/
pub struct Installed;

impl IconPackProvider for Installed {
    async fn get(&self) -> Result<IconPackContents> {
        match load_custom_override().await? {
            Some(contents) => Ok(contents),
            None => Modern.get().await,
        }
    }
}

/**
    Loads the custom icon pack configured in Studio (`IconOverrideDir` → a `RobloxCustom` theme),
    or `None` when no usable custom pack is set (so the caller falls back to `Modern`).
*/
async fn load_custom_override() -> Result<Option<IconPackContents>> {
    let Some(settings_path) = roblox_studio_utils::global_settings_path() else {
        return Ok(None);
    };
    let Ok(xml) = fs::read_to_string(&settings_path).await else {
        return Ok(None);
    };
    let Some(override_dir) = icon_override_dir(&xml) else {
        return Ok(None);
    };

    let custom_dirs = discover_roblox_custom_dirs(&override_dir)
        .await
        .unwrap_or_default();
    let Some(root) = custom_dirs.into_iter().next() else {
        return Ok(None);
    };

    let Ok(theme_str) = fs::read_to_string(root.join("index.theme")).await else {
        return Ok(None);
    };
    let theme = CustomThemeFile::from_str(&theme_str).context("invalid custom icon theme")?;
    let paths = theme
        .best_instances_paths(&root)
        .await
        .context("reading custom icon theme")?;
    if paths.is_empty() {
        return Ok(None);
    }

    let mut contents = IconPackContents::new();
    for path in paths {
        let Some(name) = path.file_name() else {
            continue;
        };
        let bytes = fs::read(&path)
            .await
            .with_context(|| format!("reading {}", path.display()))?;
        contents.insert_icon(PathBuf::from(name), bytes);
    }
    Ok(Some(contents))
}

/**
    Extracts the `IconOverrideDir` value from Studio's `GlobalSettings` XML
    (`<QDir name="IconOverrideDir">PATH</QDir>`).
*/
fn icon_override_dir(xml: &str) -> Option<PathBuf> {
    let marker = xml.find(r#"name="IconOverrideDir""#)?;
    let rest = &xml[marker..];
    let value_start = rest.find('>')? + 1;
    let value_end = rest[value_start..].find('<')?;
    let value = rest[value_start..value_start + value_end].trim();
    (!value.is_empty()).then(|| PathBuf::from(value))
}
