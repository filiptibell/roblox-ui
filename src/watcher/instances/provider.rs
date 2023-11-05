use anyhow::Result;

use crate::watcher::Settings;

use super::{
    file_sourcemap::FileSourcemapProvider, none::NoneProvider,
    rojo_sourcemap::RojoSourcemapProvider, InstanceNode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstanceProviderKind {
    #[default]
    None,
    FileSourcemap,
    RojoSourcemap,
}

#[derive(Debug)]
pub enum InstanceProvider {
    None(NoneProvider),
    FileSourcemap(FileSourcemapProvider),
    RojoSourcemap(RojoSourcemapProvider),
}

impl InstanceProvider {
    pub fn from_kind(kind: InstanceProviderKind, settings: Settings) -> Self {
        match kind {
            InstanceProviderKind::None => Self::None(NoneProvider::new(settings)),
            InstanceProviderKind::FileSourcemap => {
                Self::FileSourcemap(FileSourcemapProvider::new(settings))
            }
            InstanceProviderKind::RojoSourcemap => {
                Self::RojoSourcemap(RojoSourcemapProvider::new(settings))
            }
        }
    }

    pub fn kind(&self) -> InstanceProviderKind {
        match self {
            Self::None(_) => InstanceProviderKind::None,
            Self::FileSourcemap(_) => InstanceProviderKind::FileSourcemap,
            Self::RojoSourcemap(_) => InstanceProviderKind::RojoSourcemap,
        }
    }

    pub async fn start(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        match self {
            Self::None(n) => n.start().await,
            Self::FileSourcemap(f) => f.start(smap).await,
            Self::RojoSourcemap(r) => r.start().await,
        }
    }

    pub async fn update(&mut self, smap: Option<&InstanceNode>) -> Result<()> {
        match self {
            Self::None(n) => n.update().await,
            Self::FileSourcemap(f) => f.update(smap).await,
            Self::RojoSourcemap(r) => r.update().await,
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        match self {
            Self::None(n) => n.stop().await,
            Self::FileSourcemap(f) => f.stop().await,
            Self::RojoSourcemap(r) => r.stop().await,
        }
    }
}
