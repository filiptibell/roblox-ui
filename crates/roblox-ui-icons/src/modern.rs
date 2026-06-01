use std::path::Path;

use anyhow::{Context, Result};
use async_fs as fs;
use futures_lite::StreamExt;
use roblox_studio_utils::RobloxStudioPaths;

use super::*;

/**
    The current Roblox Studio explorer icons, read from the installed Studio's themed
    `InsertableObjects` set. Errors if Studio is not installed.
*/
pub struct Modern;

impl IconPackProvider for Modern {
    async fn get(&self) -> Result<IconPackContents> {
        let paths = RobloxStudioPaths::new().context("Roblox Studio is not installed")?;
        let base = paths
            .content()
            .join("studio_svg_textures")
            .join("Shared")
            .join("InsertableObjects");

        let mut contents = IconPackContents::new();
        load_theme(&base.join("Light").join("Standard"), true, &mut contents).await?;
        load_theme(&base.join("Dark").join("Standard"), false, &mut contents).await?;
        Ok(contents)
    }
}

/**
    Reads the `<Class>@2x.png` icons from `dir` (the crisp 2x variant, re-keyed to `<Class>.png` so
    the metadata maps them by class) into the light or dark set.
*/
async fn load_theme(dir: &Path, light: bool, contents: &mut IconPackContents) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .await
        .with_context(|| format!("failed to read {}", dir.display()))?;
    while let Some(entry) = entries.next().await {
        let path = entry?.path();
        let Some(class) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix("@2x.png"))
        else {
            continue;
        };
        let bytes = fs::read(&path).await?;
        let key = format!("{class}.png");
        if light {
            contents.insert_icon_light(key, bytes);
        } else {
            contents.insert_icon_dark(key, bytes);
        }
    }
    Ok(())
}
