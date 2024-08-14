use std::fmt;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIs, EnumString};
use ustr::Ustr;

const MAIN_SEPARATOR: char = '/';
const PATH_SEPARATOR: char = '.';

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumString, EnumIs, Display)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase", prefix = "@")]
pub enum ApiDocKeyScope {
    Roblox,
    Luau,
}

impl ApiDocKeyScope {
    pub(super) fn parse(s: &str) -> Option<Self> {
        s.trim().trim_start_matches('@').parse().ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumString, EnumIs, Display)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum ApiDocKeySubscope {
    Global,
    GlobalType,
    Enum,
}

impl ApiDocKeySubscope {
    pub(super) fn parse(s: &str) -> Option<Self> {
        s.trim().parse().ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIs)]
pub enum ApiDocKeyPathExtra {
    Param(usize),
    Return(usize),
    Overload(String),
}

impl ApiDocKeyPathExtra {
    pub(super) fn parse(s: &str) -> Option<Self> {
        match s.split_once(MAIN_SEPARATOR) {
            Some(("param", rest)) => rest.parse().ok().map(Self::Param),
            Some(("return", rest)) => rest.parse().ok().map(Self::Return),
            Some(("overload", rest)) => Some(Self::Overload(rest.to_string())),
            _ => None,
        }
    }
}

impl fmt::Display for ApiDocKeyPathExtra {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Param(i) => write!(f, "param{MAIN_SEPARATOR}{i}"),
            Self::Return(i) => write!(f, "return{MAIN_SEPARATOR}{i}"),
            Self::Overload(s) => write!(f, "overload{MAIN_SEPARATOR}{s}"),
        }
    }
}

/**
    A key for an API documentation item.

    This is a combination of the scope, subscope, path, and
    extra data, and typically looks something like this:

    - `@luau/global/string.split`
    - `@luau/global/string.split/param/0`
    - `@luau/global/string.split/param/1`
    - `@luau/global/string.split/return/0`
    - `@roblox/global/Vector3`
    - `@roblox/global/Vector3.new`
    - `@roblox/globaltype/Vector3/X`
    - `@roblox/globaltype/Vector3/Y`
    - `@roblox/enum/EnumName`
    - `@roblox/enum/EnumName.EnumItemName`
*/
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiDocKey {
    pub scope: ApiDocKeyScope,
    pub subscope: ApiDocKeySubscope,
    pub path: Vec<Ustr>,
    pub extra: Option<ApiDocKeyPathExtra>,
}

impl ApiDocKey {
    pub(super) fn parse(s: &str) -> Option<Self> {
        let (scope_str, rest) = s.trim().split_once(MAIN_SEPARATOR)?;
        let (subscope_str, rest) = rest.split_once(MAIN_SEPARATOR)?;

        let scope = ApiDocKeyScope::parse(scope_str)?;
        let subscope = ApiDocKeySubscope::parse(subscope_str)?;
        let (path, extra) = match rest.split_once(MAIN_SEPARATOR) {
            Some((path, rest)) => (path, ApiDocKeyPathExtra::parse(rest)),
            None => (rest, None),
        };

        let path = path.split(PATH_SEPARATOR).map(Ustr::from).collect();

        Some(Self {
            scope,
            subscope,
            path,
            extra,
        })
    }
}

impl fmt::Display for ApiDocKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.scope)?;
        write!(f, "{MAIN_SEPARATOR}")?;
        write!(f, "{}", self.subscope)?;
        write!(f, "{MAIN_SEPARATOR}")?;

        let num_parts = self.path.len();
        for (i, part) in self.path.iter().enumerate() {
            write!(f, "{}", part)?;
            if i < num_parts - 1 {
                write!(f, "{PATH_SEPARATOR}")?;
            }
        }

        if let Some(extra) = self.extra.as_ref() {
            write!(f, "{MAIN_SEPARATOR}")?;
            write!(f, "{extra}")?;
        }

        Ok(())
    }
}

impl Serialize for ApiDocKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ApiDocKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ApiDocKey::parse(&s).ok_or_else(|| serde::de::Error::custom("invalid api docs key"))
    }
}
