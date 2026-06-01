use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use anyhow::Result;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

use super::classic::*;
use super::modern::*;
use super::vanilla2::*;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
pub enum IconPack {
    Classic,
    Vanilla2,
    Modern,
}

impl IconPack {
    pub fn all() -> &'static [Self] {
        &[Self::Classic, Self::Vanilla2, Self::Modern]
    }

    pub async fn get(self) -> Result<IconPackContents> {
        match self {
            Self::Classic => Classic.get().await,
            Self::Vanilla2 => Vanilla2.get().await,
            Self::Modern => Modern.get().await,
        }
    }
}

impl Display for IconPack {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let s = match self {
            Self::Classic => "Classic",
            Self::Vanilla2 => "Vanilla2",
            Self::Modern => "Modern",
        };
        s.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum IconPackParseError {
    #[error("unknown icon pack - must be one of 'Classic', 'Vanilla2', 'Modern'")]
    UnknownIconPack,
}

impl FromStr for IconPack {
    type Err = IconPackParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_ref() {
            "classic" => Ok(Self::Classic),
            "vanilla2" | "vanilla2.1" | "vanilla2_1" => Ok(Self::Vanilla2),
            "modern" => Ok(Self::Modern),
            _ => Err(IconPackParseError::UnknownIconPack),
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait IconPackProvider {
    async fn get(&self) -> Result<IconPackContents>;
}
