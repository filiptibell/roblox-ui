use std::collections::BTreeMap;

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, Value as JsonValue};
use url::Url;

use super::{ApiDocLink, ApiDocNamedLink};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocItem {
    #[serde(
        default,
        alias = "documentation",
        deserialize_with = "deserialize_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        default,
        alias = "learn_more_link",
        deserialize_with = "deserialize_url",
        skip_serializing_if = "Option::is_none"
    )]
    pub learn_more_url: Option<Url>,
    #[serde(
        default,
        deserialize_with = "deserialize_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub code_sample: Option<String>,
    #[serde(default, alias = "keys", skip_serializing_if = "BTreeMap::is_empty")]
    pub linked_children: BTreeMap<String, ApiDocLink>,
    #[serde(default, alias = "params", skip_serializing_if = "Vec::is_empty")]
    pub linked_params: Vec<ApiDocNamedLink>,
    #[serde(default, alias = "returns", skip_serializing_if = "Vec::is_empty")]
    pub linked_returns: Vec<ApiDocLink>,
}

impl ApiDocItem {
    pub(super) fn parse(value: &JsonValue) -> Result<Self> {
        from_value(value.clone()).map_err(Error::from)
    }
}

fn deserialize_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    Ok(s.filter(|s| !s.trim().is_empty()))
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s.filter(|s| !s.trim().is_empty()) {
        Some(s) => Url::parse(&s).map_err(serde::de::Error::custom).map(Some),
        None => Ok(None),
    }
}
