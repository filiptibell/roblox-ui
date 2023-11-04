use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RojoProjectFile {
    name: String,
    tree: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourcemapNode {
    name: String,
    class_name: String,
    #[serde(default)]
    file_paths: Vec<PathBuf>,
    #[serde(default)]
    children: Vec<SourcemapNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourcemapProviderMode {
    #[default]
    None,
    Sourcemap,
    RojoProject,
}

impl SourcemapProviderMode {
    pub const fn is_sourcemap(&self) -> bool {
        matches!(self, Self::Sourcemap)
    }

    pub const fn is_rojo_project(&self) -> bool {
        matches!(self, Self::RojoProject)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SourcemapProvider {
    current_mode: Arc<Mutex<SourcemapProviderMode>>,
    last_sourcemap: Arc<Mutex<Option<SourcemapNode>>>,
    last_project: Arc<Mutex<Option<RojoProjectFile>>>,
    // TODO: Store some kind of dyn handler here, we need a
    // shared trait that gets implemented for the sourcemap file
    // handler as well as the rojo sourcemap watch handler
}

impl SourcemapProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(&self) -> SourcemapProviderMode {
        *self
            .current_mode
            .try_lock()
            .expect("sourcemap mode lock is being held")
    }

    pub fn set_mode(&self, new_mode: SourcemapProviderMode) {
        let mut guard = self
            .current_mode
            .try_lock()
            .expect("sourcemap mode lock is being held");
        *guard = new_mode
    }

    fn transition_to_mode(&self, mode: SourcemapProviderMode) -> Result<()> {
        if self.mode() != mode {
            self.set_mode(mode);
            info!("new provider mode: {mode:#?}");
            if mode.is_sourcemap() {
                // TODO: emit initial sourcemap
            } else if mode.is_rojo_project() {
                // TODO: check and spawn rojo
            }
        }

        Ok(())
    }

    pub fn update_sourcemap(&self, contents: Option<&str>) -> Result<()> {
        let smap = contents
            .and_then(|c| if c.is_empty() { None } else { Some(c) })
            .map(serde_json::from_str::<SourcemapNode>)
            .transpose()
            .context("failed to deserialize sourcemap")?;

        let mut sourcemap_guard = self
            .last_sourcemap
            .try_lock()
            .expect("sourcemap lock is being held");

        if let Some(smap) = smap {
            // TODO: Give new sourcemap
            *sourcemap_guard = Some(smap);
            // We should not overwrite the current sourcemap
            // if it is being emitted using the rojo executable
            if !self.mode().is_rojo_project() {
                self.transition_to_mode(SourcemapProviderMode::Sourcemap)?;
            }
        } else if self.mode().is_sourcemap() {
            *sourcemap_guard = None;
            // Same as above
            if !self.mode().is_rojo_project() {
                self.transition_to_mode(SourcemapProviderMode::None)?;
            }
        }

        Ok(())
    }

    pub fn update_project(&self, contents: Option<&str>) -> Result<()> {
        let proj = contents
            .and_then(|c| if c.is_empty() { None } else { Some(c) })
            .map(serde_json::from_str::<RojoProjectFile>)
            .transpose()
            .context("failed to deserialize rojo project")?;

        let mut project_guard = self
            .last_project
            .try_lock()
            .expect("rojo project lock is being held");

        if *project_guard != proj {
            *project_guard = proj;

            // Reset any spawned process
            self.transition_to_mode(SourcemapProviderMode::None)?;

            if project_guard.is_some() {
                // Try to spawn a new one
                self.transition_to_mode(SourcemapProviderMode::RojoProject)?;
            } else {
                // Go back to either using manual sourcemaps or nothing
                let sourcemap_exists = self
                    .last_sourcemap
                    .try_lock()
                    .expect("sourcemap lock is being held")
                    .is_some();
                if sourcemap_exists {
                    self.transition_to_mode(SourcemapProviderMode::Sourcemap)?;
                } else {
                    self.transition_to_mode(SourcemapProviderMode::None)?;
                }
            }
        }

        Ok(())
    }
}
