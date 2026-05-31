use std::{collections::HashMap, net::SocketAddr, path::PathBuf};

use anyhow::{bail, Context, Result};
use async_net::TcpStream;
use serde::Deserialize;

pub struct RojoClient {
    url_info: String,
    url_read: String,
}

impl RojoClient {
    /**
        Connect and create a client for the rojo serve session at the given `addr`.

        Note that it is not guaranteed that Rojo is listening on the address when
        this returns, only that we were able to connect, and have an http client.

        To ensure that Rojo is listening, use [`RojoSessionClient::get_info`].
    */
    pub async fn connect(addr: impl Into<SocketAddr>) -> Result<Self> {
        let addr: SocketAddr = addr.into();
        if !addr.ip().is_loopback() {
            bail!("address must be local/loopback")
        }

        TcpStream::connect(addr)
            .await
            .context("failed to connect")?;

        Ok(Self {
            url_info: format!("http://{addr}/api/rojo"),
            url_read: format!("http://{addr}/api/read/"),
        })
    }

    /**
        Get info about the current serve session.

        May fail if the serve session is no longer available.
    */
    pub async fn get_info(&self) -> Result<RojoSessionInfo> {
        let (status, info_bytes) = roblox_ui_http::get(&self.url_info)
            .await
            .context("failed to make request")?;

        if !(200..300).contains(&status) {
            bail!("request failed with status {status}")
        }

        serde_json::from_slice(&info_bytes).context("failed to deserialize response")
    }

    /**
        Read the instance with the given id.

        May fail if the serve session is no longer available.
    */
    pub async fn read(&self, id: impl AsRef<str>) -> Result<RojoSessionReadResponse> {
        let url = format!("{}{}", self.url_read, id.as_ref());
        let (status, read_bytes) = roblox_ui_http::get(&url)
            .await
            .context("failed to make request")?;

        if !(200..300).contains(&status) {
            bail!("request failed with status {status}")
        }

        serde_json::from_slice(&read_bytes).context("failed to deserialize response")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RojoSessionInfo {
    pub session_id: String,
    #[allow(dead_code)]
    pub project_name: String,
    pub root_instance_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RojoSessionReadResponse {
    pub session_id: String,
    pub instances: HashMap<String, RojoSessionInstance>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RojoSessionInstance {
    pub metadata: Option<RojoSessionInstanceMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RojoSessionInstanceMetadata {
    /**
        NOTE: This field does not exist yet, but will probably be added in a future PR

        https://github.com/rojo-rbx/rojo/pull/337
    */
    #[serde(default)]
    pub relevant_paths: Vec<PathBuf>,
}
