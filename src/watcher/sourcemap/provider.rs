use anyhow::Result;

use crate::watcher::Settings;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourcemapProviderKind {
    #[default]
    None,
    File,
    Rojo,
}

#[derive(Debug, Default)]
pub enum SourcemapProvider {
    #[default]
    None,
    File(FileProvider),
    Rojo(RojoProvider),
}

impl SourcemapProvider {
    pub fn from_kind(kind: SourcemapProviderKind, settings: Settings) -> Self {
        match kind {
            SourcemapProviderKind::None => Self::None,
            SourcemapProviderKind::File => Self::File(FileProvider::new(settings)),
            SourcemapProviderKind::Rojo => Self::Rojo(RojoProvider::new(settings)),
        }
    }

    pub fn kind(&self) -> SourcemapProviderKind {
        match self {
            Self::None => SourcemapProviderKind::None,
            Self::File(_) => SourcemapProviderKind::File,
            Self::Rojo(_) => SourcemapProviderKind::Rojo,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::File(f) => f.start().await,
            Self::Rojo(r) => r.start().await,
        }
    }

    pub async fn update(&mut self) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::File(f) => f.update().await,
            Self::Rojo(r) => r.update().await,
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::File(f) => f.stop().await,
            Self::Rojo(r) => r.stop().await,
        }
    }
}
