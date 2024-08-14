use serde::{Deserialize, Serialize};

use super::ApiDocKey;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApiDocLink(pub ApiDocKey);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApiDocNamedLink {
    pub name: String,
    #[serde(alias = "documentation")]
    pub link: ApiDocKey,
}
