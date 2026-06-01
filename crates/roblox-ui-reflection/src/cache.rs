use std::path::PathBuf;

use anyhow::{Context, Result};
use async_fs as fs;

use crate::constants::URL_VERSION;
use crate::{download_latest_studio, parse_reflection_metadata, Reflection};

fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("roblox-ui").join("reflection"))
}

/**
    Loads the cached reflection metadata, if any. Synchronous and network-free, for use at startup.
*/
pub fn load_cached() -> Option<Reflection> {
    let json = std::fs::read_to_string(cache_dir()?.join("reflection.json")).ok()?;
    serde_json::from_str(&json).ok()
}

async fn fetch_version() -> Result<String> {
    let bytes = roblox_ui_http::get_bytes(URL_VERSION).await?;
    Ok(String::from_utf8(bytes.to_vec())?.trim().to_string())
}

async fn fetch_and_cache(version: &str) -> Result<()> {
    let dir = cache_dir().context("no cache directory")?;
    let studio = download_latest_studio().await?;
    let xml = roblox_ui_util::zip::extract_file_from_zip(&studio, "ReflectionMetadata.xml")?;
    let reflection = parse_reflection_metadata(&xml)?;

    fs::create_dir_all(&dir).await?;
    fs::write(
        dir.join("reflection.json"),
        serde_json::to_string(&reflection)?,
    )
    .await?;
    fs::write(
        dir.join("meta.json"),
        serde_json::json!({ "studio_version": version }).to_string(),
    )
    .await?;
    Ok(())
}

/**
    Checks for a newer Studio version at most once a day, re-downloading the reflection metadata if
    it changed. Returns whether fresh data was fetched; it is visible to the next `load_cached`.
*/
pub async fn update_if_stale() -> Result<bool> {
    let dir = cache_dir().context("no cache directory")?;
    let meta: serde_json::Value = fs::read_to_string(dir.join("meta.json"))
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if meta.get("last_check").and_then(|v| v.as_str()) == Some(today.as_str()) {
        return Ok(false);
    }

    let version = fetch_version().await?;
    let stale = meta.get("studio_version").and_then(|v| v.as_str()) != Some(version.as_str())
        || fs::metadata(dir.join("reflection.json")).await.is_err();
    if stale {
        fetch_and_cache(&version).await?;
    }

    fs::create_dir_all(&dir).await?;
    let meta = serde_json::json!({ "last_check": today, "studio_version": version });
    fs::write(dir.join("meta.json"), meta.to_string()).await?;
    Ok(stale)
}

/**
    The cached reflection metadata, or a fresh fetch when nothing is cached yet.
*/
pub async fn cached_or_fetch() -> Result<Reflection> {
    let path = cache_dir()
        .context("no cache directory")?
        .join("reflection.json");
    let cached = fs::read_to_string(&path)
        .await
        .ok()
        .and_then(|j| serde_json::from_str(&j).ok());
    if let Some(reflection) = cached {
        return Ok(reflection);
    }
    fetch_and_cache(&fetch_version().await?).await?;
    Ok(serde_json::from_str(&fs::read_to_string(&path).await?)?)
}
