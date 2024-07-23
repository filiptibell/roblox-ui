use std::{collections::HashMap, net::SocketAddr};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use tokio::net::TcpStream;

pub struct RojoSessionClient {
    client: reqwest::Client,
    url_info: String,
    url_read: String,
}

impl RojoSessionClient {
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

        let client = reqwest::Client::new();
        Ok(Self {
            client,
            url_info: format!("http://{addr}/api/rojo"),
            url_read: format!("http://{addr}/api/read/"),
        })
    }

    /**
        Get info about the current serve session.

        May fail if the serve session is no longer available.
    */
    pub async fn get_info(&self) -> Result<RojoSessionInfo> {
        let info_res = self
            .client
            .get(&self.url_info)
            .send()
            .await
            .context("failed to make request")?;

        if !info_res.status().is_success() {
            bail!(
                "{} {}",
                info_res.status().as_u16(),
                info_res.status().canonical_reason().unwrap_or("N/A")
            )
        }

        let info_bytes = info_res
            .bytes()
            .await
            .context("failed to get response bytes")?;
        serde_json::from_slice(&info_bytes).context("failed to deserialize response")
    }

    /**
        Read the instance with the given id.

        May fail if the serve session is no longer available.
    */
    pub async fn read(&self, id: impl AsRef<str>) -> Result<RojoSessionReadResponse> {
        let read_res = self
            .client
            .get(&format!("{}{}", self.url_read, id.as_ref()))
            .send()
            .await
            .context("failed to make request")?;

        if !read_res.status().is_success() {
            bail!(
                "{} {}",
                read_res.status().as_u16(),
                read_res.status().canonical_reason().unwrap_or("N/A")
            )
        }

        let read_bytes = read_res
            .bytes()
            .await
            .context("failed to get response bytes")?;
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
pub struct RojoSessionInstanceMetadata {}
