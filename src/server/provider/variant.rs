use anyhow::Result;

use crate::server::Config;

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

/**
    A container enum for different instance providers.

    Should be constructed using [`InstanceProvider::from_kind`], with three main methods:

    - [`InstanceProvider::start`] to start the provider
    - [`InstanceProvider::update`] to update the provider with new optional instance root node
    - [`InstanceProvider::stop`] to stop the provider

    The `start` and `stop` methods may only be called once, and the
    `update` method must be called *after* `start`, but *before* `stop`.
*/
#[derive(Debug)]
pub enum InstanceProviderVariant {
    None(NoneProvider),
    FileSourcemap(FileSourcemapProvider),
    RojoSourcemap(RojoSourcemapProvider),
}

impl InstanceProviderVariant {
    pub fn from_kind(kind: InstanceProviderKind, config: Config) -> Self {
        match kind {
            InstanceProviderKind::None => Self::None(NoneProvider::new(config)),
            InstanceProviderKind::FileSourcemap => {
                Self::FileSourcemap(FileSourcemapProvider::new(config))
            }
            InstanceProviderKind::RojoSourcemap => {
                Self::RojoSourcemap(RojoSourcemapProvider::new(config))
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

impl Default for InstanceProviderVariant {
    fn default() -> Self {
        Self::from_kind(InstanceProviderKind::default(), Config::default())
    }
}
