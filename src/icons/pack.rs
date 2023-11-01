use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use anyhow::Result;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use thiserror::Error;

use super::classic::*;
use super::vanilla2::*;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, SerializeDisplay, DeserializeFromStr)]
pub enum IconPack {
    Classic,
    Vanilla2,
}

impl IconPack {
    pub fn all() -> &'static [Self] {
        &[Self::Classic, Self::Vanilla2]
    }

    pub fn provider(&self) -> &'static dyn IconPackProvider {
        match self {
            Self::Classic => &Classic,
            Self::Vanilla2 => &Vanilla2,
        }
    }
}

impl Display for IconPack {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let s = match self {
            Self::Classic => "Classic",
            Self::Vanilla2 => "Vanilla2",
        };
        s.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum IconPackParseError {
    #[error("unknown icon pack - must be one of 'Classic', 'Vanilla2'")]
    UnknownIconPack,
}

impl FromStr for IconPack {
    type Err = IconPackParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_ref() {
            "classic" => Ok(Self::Classic),
            "vanilla2" | "vanilla2.1" | "vanilla2_1" => Ok(Self::Vanilla2),
            _ => Err(IconPackParseError::UnknownIconPack),
        }
    }
}

#[async_trait::async_trait]
pub trait IconPackProvider {
    async fn download(&self) -> Result<IconPackContents>;
}
