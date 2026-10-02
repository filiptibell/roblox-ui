use std::sync::{Arc, OnceLock};

use anyhow::{bail, Context, Result};
use async_net::TcpStream;
use bytes::Bytes;
use futures_rustls::TlsConnector;
use http_body_util::{BodyExt, Full};
use hyper::{header::HOST, Request};
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::ServerName;
use url::Url;

mod io;

use io::FuturesIo;

const MAX_REDIRECTS: usize = 10;

fn tls_config() -> Arc<ClientConfig> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let mut roots = RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let config = ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("failed to build rustls protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
            Arc::new(config)
        })
        .clone()
}

/**
    An HTTP response: the status code, its reason phrase, the
    headers in the order the server sent them, and the body.
*/
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

/**
    Performs a single HTTP/HTTPS request, without following redirects.
*/
async fn request_once(
    method: &str,
    url: &Url,
    headers: &[(&str, &str)],
    body: Bytes,
) -> Result<Response> {
    let host = url.host_str().context("missing host in url")?.to_string();
    let is_https = match url.scheme() {
        "https" => true,
        "http" => false,
        other => bail!("unsupported url scheme '{other}'"),
    };
    let port = url.port_or_known_default().context("missing port in url")?;

    let path_and_query = match url.query() {
        Some(q) => format!("{}?{}", url.path(), q),
        None => url.path().to_string(),
    };

    let tcp = TcpStream::connect((host.as_str(), port))
        .await
        .with_context(|| format!("failed to connect to {host}:{port}"))?;

    // Build the request once - same shape regardless of TLS
    let mut builder = Request::builder()
        .method(method)
        .uri(&path_and_query)
        .header(HOST, &host);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let request = builder
        .body(Full::new(body))
        .context("failed to build request")?;

    // Send the request over either a plain or TLS-wrapped connection
    let response = if is_https {
        let connector = TlsConnector::from(tls_config());
        let server_name = ServerName::try_from(host.clone()).context("invalid dns name for tls")?;
        let tls = connector
            .connect(server_name, tcp)
            .await
            .context("tls handshake failed")?;
        send_request(FuturesIo::new(tls), request).await?
    } else {
        send_request(FuturesIo::new(tcp), request).await?
    };

    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            let value = String::from_utf8_lossy(value.as_bytes()).into_owned();
            (name.as_str().to_string(), value)
        })
        .collect();

    let body = response
        .into_body()
        .collect()
        .await
        .context("failed to read response body")?
        .to_bytes();

    Ok(Response {
        status: status.as_u16(),
        reason: status.canonical_reason().unwrap_or_default().to_string(),
        headers,
        body,
    })
}

/**
    The `Location` header of a redirect response, if the status is a redirect.
*/
fn redirect_location(response: &Response) -> Option<&str> {
    if !(300..400).contains(&response.status) {
        return None;
    }
    response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("location"))
        .map(|(_, value)| value.as_str())
}

async fn send_request<I>(
    io: I,
    request: Request<Full<Bytes>>,
) -> Result<hyper::Response<hyper::body::Incoming>>
where
    I: hyper::rt::Read + hyper::rt::Write + Send + Unpin + 'static,
{
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .context("http handshake failed")?;

    // The connection has to be driven concurrently with the request
    async_global_executor::spawn(async move {
        let _ = conn.await;
    })
    .detach();

    sender
        .send_request(request)
        .await
        .context("failed to send request")
}

/**
    Performs an HTTP/HTTPS GET request, following redirects, and
    returns the final status code together with the response body.

    Does not error on non-success status codes - the caller decides.
*/
pub async fn get(url: impl AsRef<str>) -> Result<(u16, Bytes)> {
    get_with_headers(url, &[]).await
}

/**
    Like [`get`], but attaches the given `headers` (name/value pairs) to the request.

    The headers are sent only on the INITIAL request, not re-sent across a redirect
    (a redirect to a different host — e.g. a signed CDN URL — must not carry auth).
*/
pub async fn get_with_headers(
    url: impl AsRef<str>,
    headers: &[(&str, &str)],
) -> Result<(u16, Bytes)> {
    let response = request("GET", url, headers, Bytes::new()).await?;
    Ok((response.status, response.body))
}

/**
    Performs an HTTP/HTTPS request with the given method, headers, and body.

    A `GET` or `HEAD` request follows redirects. The headers are sent only on
    the INITIAL request, not re-sent across a redirect (a redirect to a
    different host — e.g. a signed CDN URL — must not carry auth). Any other
    method returns a redirect response as it is.

    Does not error on non-success status codes - the caller decides.
*/
pub async fn request(
    method: &str,
    url: impl AsRef<str>,
    headers: &[(&str, &str)],
    body: Bytes,
) -> Result<Response> {
    let mut current = Url::parse(url.as_ref()).context("failed to parse url")?;
    let follows = method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("HEAD");
    let mut hop_headers = headers;

    for _ in 0..MAX_REDIRECTS {
        let response = request_once(method, &current, hop_headers, body.clone()).await?;
        if follows {
            if let Some(location) = redirect_location(&response) {
                current = current
                    .join(location)
                    .context("failed to resolve redirect location")?;
                hop_headers = &[]; // drop auth headers when following a (possibly cross-host) redirect
                continue;
            }
        }
        return Ok(response);
    }

    bail!("too many redirects (>{MAX_REDIRECTS})")
}

/**
    Performs an HTTP/HTTPS GET request, following redirects, and
    returns the response body. Errors on any non-success status code.
*/
pub async fn get_bytes(url: impl AsRef<str>) -> Result<Bytes> {
    get_bytes_with_headers(url, &[]).await
}

/**
    Like [`get_bytes`], but attaches the given `headers` to the initial request
    (see [`get_with_headers`]). Errors on any non-success status code.
*/
pub async fn get_bytes_with_headers(
    url: impl AsRef<str>,
    headers: &[(&str, &str)],
) -> Result<Bytes> {
    let url = url.as_ref();
    let (status, body) = get_with_headers(url, headers).await?;
    if !(200..300).contains(&status) {
        bail!("request to '{url}' failed with status {status}");
    }
    Ok(body)
}
