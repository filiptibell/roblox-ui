#![allow(dead_code)]

use std::io;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

fn next_data_id() -> u64 {
    static ID_COUNTER: AtomicU64 = AtomicU64::new(0);
    ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum RpcError {
    Io(#[from] io::Error),
    Json(#[from] serde_json::Error),
}

pub type RpcResult<T> = Result<T, RpcError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcData {
    id: u64,
    method: String,
    value: Option<JsonValue>,
}

impl RpcData {
    fn new(method: impl Into<String>) -> Self {
        Self {
            id: next_data_id(),
            method: method.into(),
            value: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum RpcMessage {
    Request(RpcData),
    Response(RpcData),
}

impl RpcMessage {
    pub fn new_request(method: impl Into<String>) -> Self {
        Self::Request(RpcData::new(method))
    }

    pub fn new_response(method: impl Into<String>) -> Self {
        Self::Response(RpcData::new(method))
    }

    pub const fn is_request(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    pub const fn is_response(&self) -> bool {
        matches!(self, Self::Response(_))
    }

    pub fn respond(&self) -> Self {
        if let Self::Request(req) = self {
            let mut data = RpcData::new(&req.method);
            data.id = req.id;
            Self::Response(data)
        } else {
            panic!("can only respond to requests")
        }
    }

    pub fn with_data(mut self, data: impl Serialize) -> RpcResult<Self> {
        let value = serde_json::to_value(data)?;
        let inner = match self {
            Self::Request(ref mut d) => &mut d.value,
            Self::Response(ref mut d) => &mut d.value,
        };
        inner.replace(value);
        Ok(self)
    }

    pub fn get_data<T>(&self) -> RpcResult<T>
    where
        T: DeserializeOwned,
    {
        let inner = match self {
            Self::Request(d) => d,
            Self::Response(d) => d,
        };
        Ok(serde_json::from_value(match inner.value.as_ref() {
            None => JsonValue::Null,
            Some(v) => v.clone(),
        })?)
    }

    pub fn get_method(&self) -> &str {
        let inner = match self {
            Self::Request(d) => d,
            Self::Response(d) => d,
        };
        &inner.method
    }

    pub async fn read_from<R>(reader: &mut R) -> Option<RpcResult<Self>>
    where
        R: AsyncBufRead + Unpin,
    {
        let mut buf = String::new();
        let num_bytes = match reader.read_line(&mut buf).await {
            Err(e) => return Some(Err(e.into())),
            Ok(v) => v,
        };
        if num_bytes > 0 {
            match serde_json::from_str(&buf) {
                Err(e) => Some(Err(e.into())),
                Ok(v) => Some(Ok(v)),
            }
        } else {
            None
        }
    }

    pub async fn write_to<R>(self, writer: &mut R) -> RpcResult<()>
    where
        R: AsyncWrite + Unpin,
    {
        let mut line = serde_json::to_string(&self)?;
        line.push('\n');
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }
}
