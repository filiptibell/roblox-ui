use std::{net::SocketAddr, path::PathBuf};

use anyhow::Result;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue};

use super::rojo_client::{RojoSessionClient, RojoSessionInfo};

/**
    Stub representing a rojo project file instance node.
*/
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RojoProjectFileNode {
    #[serde(rename = "$path")]
    pub path: Option<PathBuf>,
    #[serde(rename = "$className")]
    pub class_name: Option<String>,
    #[serde(flatten)]
    pub other_fields: JsonMap<String, JsonValue>,
}

/**
    Stub representing a rojo project file.
*/
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RojoProjectFile {
    pub name: String,
    pub tree: RojoProjectFileNode,
    pub serve_address: Option<String>,
    pub serve_port: Option<u16>,
}

impl RojoProjectFile {
    pub fn from_json(json: impl AsRef<str>) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Self>(json.as_ref())
    }

    /**
        Get the serve address for this project file.

        Can be used to create a [`RojoSessionClient`].
    */
    pub fn serve_address(&self) -> SocketAddr {
        if let Some(addr) = &self.serve_address {
            if let Ok(parsed) = addr.as_str().parse() {
                return parsed;
            }
        }

        let port = self.serve_port.unwrap_or(34872);
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    /**
        Creates a new, temporary [`RojoSessionClient`] and makes requests
        to check if this project file is currently used in a serve session.
    */
    pub async fn find_serve_session(&self) -> Option<RojoSessionInfo> {
        let addr = self.serve_address();

        // Try to connect and request info about any current serve session
        let client = RojoSessionClient::connect(addr).await.ok()?;
        let info = client.get_info().await.ok()?;

        // Now that we have the info struct we need to verify that it actually
        // came from this project file, which we can do by comparing paths
        // For this we can read the root instance which should have this data
        let root_id = info.root_instance_id;
        let root_res = client.read(&root_id).await.ok()?;

        if root_res.session_id == info.session_id {
            let root = root_res
                .instances
                .iter()
                .find_map(|(instance_id, instance)| {
                    if instance_id == &root_id {
                        Some(instance)
                    } else {
                        None
                    }
                })?;
            if let Some(_meta) = &root.metadata {
                // TODO: Compare paths when paths are included in the metadata
                // https://github.com/rojo-rbx/rojo/pull/337
            }
        }

        None
    }
}
