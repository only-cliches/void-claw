/// MITM HTTP/HTTPS proxy enforcing network policies from void-claw-rules.toml.
///
/// Containers route all traffic through this proxy. Plain HTTP requests are
/// intercepted and parsed directly. HTTPS traffic is intercepted via CONNECT
/// tunnels: the proxy terminates TLS with a per-domain leaf cert signed by
/// the void-claw CA (which containers are configured to trust), inspects the
/// inner HTTP request, then forwards to the real server.
///
/// Network policy (auto/prompt/deny) is determined by matching the composed
/// rules against method + host + path of each request.
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use futures::StreamExt;
use globset::Glob;
use reqwest::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

use crate::ca::CaStore;
use crate::config::{self, Config};
use crate::rules::NetworkPolicy;
use crate::shared_config::SharedConfig;

/// A network request waiting on the TUI for an allow/deny decision.
pub struct PendingNetworkItem {
    pub source_project: Option<String>,
    pub source_container: Option<String>,
    pub source_status: String,
    pub has_proxy_authorization: bool,
    pub method: String,
    pub host: String,
    pub path: String,
    pub response_tx: oneshot::Sender<NetworkDecision>,
}

/// The result returned by the TUI for a pending network request.
#[derive(Debug)]
pub enum NetworkDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
struct FixedSourceIdentity {
    project: String,
    container: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceIdentityStatus {
    Ok,
    ListenerBoundSource,
    MissingProxyAuthorization,
    MalformedAuthHeader,
    UnsupportedAuthScheme,
    InvalidBase64,
    InvalidUtf8,
    MissingUsernamePasswordDelimiter,
    UnexpectedUsername,
    MissingProjectContainerDelimiter,
    InvalidProjectEncoding,
    InvalidContainerEncoding,
}

impl SourceIdentityStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::ListenerBoundSource => "listener_bound_source",
            Self::MissingProxyAuthorization => "missing_proxy_authorization",
            Self::MalformedAuthHeader => "malformed_auth_header",
            Self::UnsupportedAuthScheme => "unsupported_auth_scheme",
            Self::InvalidBase64 => "invalid_base64",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::MissingUsernamePasswordDelimiter => "missing_username_password_delimiter",
            Self::UnexpectedUsername => "unexpected_username",
            Self::MissingProjectContainerDelimiter => "missing_project_container_delimiter",
            Self::InvalidProjectEncoding => "invalid_project_encoding",
            Self::InvalidContainerEncoding => "invalid_container_encoding",
        }
    }
}

// ── Proxy state ───────────────────────────────────────────────────────────────

#[derive(Clone)]
/// Shared proxy state used by all listener tasks.
pub struct ProxyState {
    pub ca: Arc<CaStore>,
    pub config: SharedConfig,
    pub pending_tx: mpsc::Sender<PendingNetworkItem>,
    client: reqwest::Client,
    fixed_source: Option<FixedSourceIdentity>,
}

impl ProxyState {
    pub fn new(
        ca: Arc<CaStore>,
        config: SharedConfig,
        pending_tx: mpsc::Sender<PendingNetworkItem>,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            ca,
            config,
            pending_tx,
            client,
            fixed_source: None,
        })
    }

    fn with_fixed_source(&self, project: &str, container: &str) -> Self {
        let mut cloned = self.clone();
        cloned.fixed_source = Some(FixedSourceIdentity {
            project: project.to_string(),
            container: container.to_string(),
        });
        cloned
    }
}

/// A scoped listener task that is aborted when dropped.
pub struct ScopedProxyListener {
    pub addr: String,
    abort_handle: tokio::task::AbortHandle,
}

impl Drop for ScopedProxyListener {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(state: ProxyState, addr: String) -> Result<()> {
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("proxy bind {addr}: {e}"))?;
    run_with_listener(state, listener).await
}

async fn run_with_listener(state: ProxyState, listener: TcpListener) -> Result<()> {
    loop {
        let (stream, _peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, state).await {
                if is_expected_disconnect(&e) {
                    debug!("proxy: {e}");
                } else {
                    error!("proxy: {e}");
                }
            }
        });
    }
}

/// Start a per-container proxy listener bound to the supplied host/port.
pub fn spawn_scoped_listener(
    state: &ProxyState,
    bind_host: &str,
    project: &str,
    container: &str,
) -> Result<ScopedProxyListener> {
    let bind_addr = format!("{bind_host}:0");
    let std_listener = std::net::TcpListener::bind(&bind_addr)
        .map_err(|e| anyhow::anyhow!("proxy bind {bind_addr}: {e}"))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| anyhow::anyhow!("proxy set_nonblocking {bind_addr}: {e}"))?;
    let local_addr = std_listener.local_addr()?;
    let listener = TcpListener::from_std(std_listener)?;
    let addr = format!("{}:{}", bind_host, local_addr.port());
    let fixed_state = state.with_fixed_source(project, container);
    let task = tokio::spawn(async move {
        if let Err(e) = run_with_listener(fixed_state, listener).await {
            error!("scoped proxy server error: {e}");
        }
    });
    Ok(ScopedProxyListener {
        addr,
        abort_handle: task.abort_handle(),
    })
}

// ── Connection dispatch ───────────────────────────────────────────────────────

async fn handle_connection(stream: TcpStream, state: ProxyState) -> Result<()> {
    let mut peek = [0u8; 8];
    let n = stream.peek(&mut peek).await?;

    // Prefer explicit CONNECT first, then fall back to sniffing for raw TLS.
    // This lets the same listener handle both proxy-aware clients and clients
    // that try to talk TLS directly to the gateway.
    if n >= 7 && &peek[..7] == b"CONNECT" {
        handle_connect(stream, state).await
    } else if looks_like_tls_client_hello(&peek[..n]) {
        handle_transparent_tls(stream, state).await
    } else {
        handle_plain_http(stream, state).await
    }
}

fn looks_like_tls_client_hello(buf: &[u8]) -> bool {
    buf.len() >= 3 && buf[0] == 0x16 && buf[1] == 0x03 && (0x01..=0x04).contains(&buf[2])
}

// ── Transparent TLS (no CONNECT) ─────────────────────────────────────────────

struct PrefixedTcpStream {
    prefix: std::io::Cursor<Vec<u8>>,
    inner: TcpStream,
}

impl AsyncRead for PrefixedTcpStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if (self.prefix.position() as usize) < self.prefix.get_ref().len() {
            let before = buf.filled().len();
            let pos = self.prefix.position();
            let rem = &self.prefix.get_ref()[pos as usize..];
            let to_copy = rem.len().min(buf.remaining());
            buf.put_slice(&rem[..to_copy]);
            self.prefix.set_position(pos + to_copy as u64);
            let after = buf.filled().len();
            debug_assert!(after > before);
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for PrefixedTcpStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        data: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.inner).poll_write(cx, data)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

async fn handle_transparent_tls(mut stream: TcpStream, state: ProxyState) -> Result<()> {
    let (source_project, source_container, source_status, has_proxy_authorization) =
        if let Some(fixed) = &state.fixed_source {
            (
                Some(fixed.project.clone()),
                Some(fixed.container.clone()),
                SourceIdentityStatus::ListenerBoundSource,
                false,
            )
        } else {
            (
                None,
                None,
                SourceIdentityStatus::MissingProxyAuthorization,
                false,
            )
        };

    let cfg = state.config.get();

    let prefix = read_tls_client_hello_prefix(&mut stream).await?;
    let Some(host) = parse_sni_from_tls_client_hello(&prefix) else {
        warn!("transparent TLS connection missing SNI; dropping");
        return Ok(());
    };

    if container_tls_passthrough_matches(&cfg, source_container.as_deref(), &host) {
        info!(
            host = %host,
            source_project = ?source_project,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            "proxy transparent TLS passthrough"
        );
        let mut upstream = TcpStream::connect(format!("{host}:443"))
            .await
            .map_err(|e| {
                anyhow::anyhow!("transparent passthrough connect to {host}:443 failed: {e}")
            })?;
        upstream.write_all(&prefix).await?;
        let _ = copy_bidirectional(&mut stream, &mut upstream).await;
        return Ok(());
    }

    let rules = match config::load_composed_rules_for_project(&cfg, source_project.as_deref()) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("proxy rules load error: {e}");
            return Ok(());
        }
    };
    let preflight_policy = rules.match_network("CONNECT", &host, "/");
    let preflight_allowed = match preflight_policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                "CONNECT",
                &host,
                "/",
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
            )
            .await
        }
    };
    if !preflight_allowed {
        return Ok(());
    }

    let prefixed = PrefixedTcpStream {
        prefix: std::io::Cursor::new(prefix),
        inner: stream,
    };

    let server_config = state.ca.leaf_server_config(&host)?;
    let acceptor = TlsAcceptor::from(server_config);
    let mut tls_stream = acceptor
        .accept(prefixed)
        .await
        .map_err(|e| anyhow::anyhow!("TLS accept for {host}: {e}"))?;

    debug!("proxy TLS established for host={host} (transparent)");

    let (inner_head, inner_remainder) = read_request_head_any(&mut tls_stream).await?;
    let inner_str = match std::str::from_utf8(&inner_head) {
        Ok(s) => s,
        Err(_) => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };
    let (method, path, headers) = match parse_request_line_and_headers(inner_str) {
        Some(r) => r,
        None => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };
    let body = read_body_any(&mut tls_stream, &headers, inner_remainder).await?;

    if source_project.is_none() {
        warn!(
            host = %host,
            method = %method,
            path = %path,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            has_proxy_authorization,
            "proxy request missing source project metadata; permanent network rule persistence will not know which project to update"
        );
    }

    let policy = rules.match_network(&method, &host, &path);
    let allowed = match policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                &method,
                &host,
                &path,
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
            )
            .await
        }
    };
    if !allowed {
        write_error_any(&mut tls_stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }
    let url = format!("https://{host}{path}");
    let response = forward_request(&state.client, &method, &url, &headers, body).await?;
    write_response_any(&mut tls_stream, response).await
}

async fn read_tls_client_hello_prefix(stream: &mut TcpStream) -> Result<Vec<u8>> {
    // We only need enough of the ClientHello to recover SNI and route policy;
    // the rest of the handshake is forwarded untouched.
    let mut hdr = [0u8; 5];
    stream.read_exact(&mut hdr).await?;
    if hdr[0] != 0x16 {
        anyhow::bail!("not a TLS handshake record");
    }
    let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
    if len > 64 * 1024 {
        anyhow::bail!("TLS record too large");
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    let mut out = Vec::with_capacity(5 + len);
    out.extend_from_slice(&hdr);
    out.extend_from_slice(&body);
    Ok(out)
}

fn parse_sni_from_tls_client_hello(record: &[u8]) -> Option<String> {
    if record.len() < 5 + 4 {
        return None;
    }
    if record[0] != 0x16 {
        return None;
    }
    let rec_len = u16::from_be_bytes([record[3], record[4]]) as usize;
    if record.len() < 5 + rec_len {
        return None;
    }
    let mut i = 5;
    if record.get(i)? != &0x01 {
        return None;
    }
    i += 1;
    let hs_len = ((record.get(i)? as &u8).to_owned() as usize) << 16
        | (((record.get(i + 1)? as &u8).to_owned() as usize) << 8)
        | (record.get(i + 2)? as &u8).to_owned() as usize;
    i += 3;
    if record.len() < i + hs_len {
        return None;
    }
    i += 2 + 32;
    let sid_len = *record.get(i)? as usize;
    i += 1 + sid_len;
    let cs_len = u16::from_be_bytes([*record.get(i)?, *record.get(i + 1)?]) as usize;
    i += 2 + cs_len;
    let comp_len = *record.get(i)? as usize;
    i += 1 + comp_len;
    let ext_len = u16::from_be_bytes([*record.get(i)?, *record.get(i + 1)?]) as usize;
    i += 2;
    let ext_end = i + ext_len;
    if record.len() < ext_end {
        return None;
    }
    while i + 4 <= ext_end {
        let et = u16::from_be_bytes([record[i], record[i + 1]]);
        let el = u16::from_be_bytes([record[i + 2], record[i + 3]]) as usize;
        i += 4;
        if i + el > ext_end {
            return None;
        }
        if et == 0x0000 && el >= 2 {
            let list_len = u16::from_be_bytes([record[i], record[i + 1]]) as usize;
            let mut j = i + 2;
            let list_end = j + list_len;
            if list_end > i + el {
                return None;
            }
            while j + 3 <= list_end {
                let name_type = record[j];
                let name_len = u16::from_be_bytes([record[j + 1], record[j + 2]]) as usize;
                j += 3;
                if j + name_len > list_end {
                    return None;
                }
                if name_type == 0 {
                    let sni = String::from_utf8_lossy(&record[j..j + name_len]).to_string();
                    if !sni.is_empty() {
                        return Some(sni);
                    }
                }
                j += name_len;
            }
        }
        i += el;
    }
    None
}

// ── HTTPS CONNECT tunnel ──────────────────────────────────────────────────────

async fn handle_connect(mut stream: TcpStream, state: ProxyState) -> Result<()> {
    let (head, connect_remainder) = read_request_head_any(&mut stream).await?;
    let head_str = std::str::from_utf8(&head).unwrap_or("");

    let (host, port) = parse_connect_target(head_str)
        .ok_or_else(|| anyhow::anyhow!("malformed CONNECT request"))?;
    let (source_project, source_container, source_status, connect_has_proxy_authorization) =
        if let Some(fixed) = &state.fixed_source {
            (
                Some(fixed.project.clone()),
                Some(fixed.container.clone()),
                SourceIdentityStatus::ListenerBoundSource,
                false,
            )
        } else {
            let (project, container, status) = parse_source_from_connect_head(head_str);
            let has_auth = connect_head_has_proxy_authorization(head_str);
            (project, container, status, has_auth)
        };

    let cfg = state.config.get();

    if container_tls_passthrough_matches(&cfg, source_container.as_deref(), &host) {
        info!(
            host = %host,
            source_project = ?source_project,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            connect_has_proxy_authorization,
            "proxy CONNECT passthrough"
        );
        stream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?;
        let mut upstream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .map_err(|e| {
                anyhow::anyhow!("CONNECT passthrough connect to {host}:{port} failed: {e}")
            })?;
        if !connect_remainder.is_empty() {
            upstream.write_all(&connect_remainder).await?;
        }
        let _ = copy_bidirectional(&mut stream, &mut upstream).await;
        return Ok(());
    }

    let rules = match config::load_composed_rules_for_project(&cfg, source_project.as_deref()) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("proxy rules load error: {e}");
            write_error_any(&mut stream, 500, "Invalid void-claw-rules.toml configuration").await?;
            return Ok(());
        }
    };
    let preflight_policy = rules.match_network("CONNECT", &host, "/");
    let preflight_allowed = match preflight_policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                "CONNECT",
                &host,
                "/",
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                connect_has_proxy_authorization,
            )
            .await
        }
    };
    if !preflight_allowed {
        write_error_any(&mut stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }

    if port != 443 {
        info!(
            host = %host,
            port,
            source_project = ?source_project,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            connect_has_proxy_authorization,
            "proxy CONNECT raw tunnel path"
        );
        stream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?;
        let mut upstream = TcpStream::connect(format!("{host}:{port}"))
            .await
            .map_err(|e| {
                anyhow::anyhow!("CONNECT raw tunnel connect to {host}:{port} failed: {e}")
            })?;
        if !connect_remainder.is_empty() {
            upstream.write_all(&connect_remainder).await?;
        }
        let _ = copy_bidirectional(&mut stream, &mut upstream).await;
        return Ok(());
    }

    info!(
        host = %host,
        source_project = ?source_project,
        source_container = ?source_container,
        source_status = source_status.as_str(),
        connect_has_proxy_authorization,
        "proxy CONNECT MITM path"
    );

    stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;

    let server_config = state.ca.leaf_server_config(&host)?;
    let acceptor = TlsAcceptor::from(server_config);
    let mut tls_stream = acceptor
        .accept(stream)
        .await
        .map_err(|e| anyhow::anyhow!("TLS accept for {host}: {e}"))?;

    debug!("proxy TLS established for host={host}");

    let (inner_head, inner_remainder) = read_request_head_any(&mut tls_stream).await?;
    let inner_str = match std::str::from_utf8(&inner_head) {
        Ok(s) => s,
        Err(_) => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };

    let (method, path, headers) = match parse_request_line_and_headers(inner_str) {
        Some(r) => r,
        None => {
            write_error_any(&mut tls_stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };

    let body = read_body_any(&mut tls_stream, &headers, inner_remainder).await?;

    if source_project.is_none() {
        warn!(
            host = %host,
            method = %method,
            path = %path,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            connect_has_proxy_authorization,
            "proxy request missing source project metadata; permanent network rule persistence will not know which project to update"
        );
    }

    let policy = rules.match_network(&method, &host, &path);

    let allowed = match policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                &method,
                &host,
                &path,
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                connect_has_proxy_authorization,
            )
            .await
        }
    };

    if !allowed {
        write_error_any(&mut tls_stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }

    let url = format!("https://{host}{path}");
    let response = forward_request(&state.client, &method, &url, &headers, body).await?;
    write_response_any(&mut tls_stream, response).await
}

// ── Plain HTTP ────────────────────────────────────────────────────────────────

async fn handle_plain_http(mut stream: TcpStream, state: ProxyState) -> Result<()> {
    let (head, body_remainder) = read_request_head_any(&mut stream).await?;
    let head_str = match std::str::from_utf8(&head) {
        Ok(s) => s,
        Err(_) => {
            write_error_any(&mut stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };

    let cfg = state.config.get();

    let (method, path, headers) = match parse_request_line_and_headers(head_str) {
        Some(r) => r,
        None => {
            write_error_any(&mut stream, 400, "Bad Request").await?;
            return Ok(());
        }
    };
    let (source_project, source_container, source_status, has_proxy_authorization) =
        if let Some(fixed) = &state.fixed_source {
            (
                Some(fixed.project.clone()),
                Some(fixed.container.clone()),
                SourceIdentityStatus::ListenerBoundSource,
                false,
            )
        } else {
            let (project, container, status) = parse_source_from_headers(&headers);

            let has_auth = headers
                .iter()
                .any(|(n, _)| n.eq_ignore_ascii_case("proxy-authorization"));
            (project, container, status, has_auth)
        };

    let host = extract_host(&headers, &path).unwrap_or_default();
    let path = strip_scheme_and_host(&path);

    let body = read_body_any(&mut stream, &headers, body_remainder).await?;

    if source_project.is_none() {
        warn!(
            host = %host,
            method = %method,
            path = %path,
            source_container = ?source_container,
            source_status = source_status.as_str(),
            has_proxy_authorization,
            "proxy request missing source project metadata; permanent network rule persistence will not know which project to update"
        );
    }

    let rules = match config::load_composed_rules_for_project(&cfg, source_project.as_deref()) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("proxy rules load error: {e}");
            write_error_any(&mut stream, 500, "Invalid void-claw-rules.toml configuration").await?;
            return Ok(());
        }
    };
    let policy = rules.match_network(&method, &host, &path);

    let allowed = match policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            prompt_network(
                &state,
                &method,
                &host,
                &path,
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
            )
            .await
        }
    };

    if !allowed {
        write_error_any(&mut stream, 403, "Forbidden by void-claw policy").await?;
        return Ok(());
    }

    let url = format!("http://{host}{path}");
    let response = forward_request(&state.client, &method, &url, &headers, body).await?;
    write_response_any(&mut stream, response).await
}

async fn prompt_network(
    state: &ProxyState,
    method: &str,
    host: &str,
    path: &str,
    source_project: Option<String>,
    source_container: Option<String>,
    source_status: &str,
    has_proxy_authorization: bool,
) -> bool {
    let (tx, rx) = oneshot::channel();
    let item = PendingNetworkItem {
        source_project,
        source_container,
        source_status: source_status.to_string(),
        has_proxy_authorization,
        method: method.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        response_tx: tx,
    };
    if state.pending_tx.send(item).await.is_err() {
        return false;
    }
    match tokio::time::timeout(Duration::from_secs(300), rx).await {
        Ok(Ok(NetworkDecision::Allow)) => true,
        _ => false,
    }
}

async fn forward_request(
    client: &reqwest::Client,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Vec<u8>,
) -> Result<reqwest::Response> {
    let method = reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET);

    let mut req = client.request(method, url);
    for (name, value) in headers {
        if !is_hop_by_hop(name) {
            req = req.header(name.as_str(), value.as_str());
        }
    }
    if !body.is_empty() {
        req = req.body(body);
    }
    let response = req.send().await?;
    Ok(response)
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

async fn read_request_head_any<R>(stream: &mut R) -> Result<(Vec<u8>, Vec<u8>)>
where
    R: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if contains_double_crlf(&buf) {
            break;
        }
        if buf.len() > 64 * 1024 {
            anyhow::bail!("request head too large");
        }
    }
    split_head_and_remainder(buf)
}

fn contains_double_crlf(buf: &[u8]) -> bool {
    buf.windows(4).any(|w| w == b"\r\n\r\n")
}

fn split_head_and_remainder(buf: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>)> {
    if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        let end = end + 4;
        Ok((buf[..end].to_vec(), buf[end..].to_vec()))
    } else {
        anyhow::bail!("incomplete request head")
    }
}

async fn read_body_any<R>(
    stream: &mut R,
    headers: &[(String, String)],
    initial: Vec<u8>,
) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let content_length = content_length_from_headers(headers);
    if content_length == 0 {
        return Ok(vec![]);
    }
    let mut body = Vec::with_capacity(content_length);
    body.extend_from_slice(&initial[..initial.len().min(content_length)]);
    if body.len() < content_length {
        let mut rest = vec![0u8; content_length - body.len()];
        stream.read_exact(&mut rest).await?;
        body.extend_from_slice(&rest);
    }
    Ok(body)
}

fn content_length_from_headers(headers: &[(String, String)]) -> usize {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

fn parse_connect_target(head: &str) -> Option<(String, u16)> {
    let first_line = head.lines().next()?;
    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let authority = parts[1];
    if authority.starts_with('[') {
        let end = authority.find(']')?;
        let host = authority[1..end].to_string();
        let port = authority[end + 1..].strip_prefix(':')?.parse().ok()?;
        return Some((host, port));
    }
    let (host, port) = authority.rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?))
}

fn parse_request_line_and_headers(head: &str) -> Option<(String, String, Vec<(String, String)>)> {
    let mut lines = head.lines();
    let first = lines.next()?;
    let parts: Vec<&str> = first.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    Some((method, path, headers))
}

fn parse_source_from_connect_head(
    head: &str,
) -> (Option<String>, Option<String>, SourceIdentityStatus) {
    let Some((_, _, headers)) = parse_request_line_and_headers(head) else {
        return (None, None, SourceIdentityStatus::MalformedAuthHeader);
    };
    parse_source_from_headers(&headers)
}

fn connect_head_has_proxy_authorization(head: &str) -> bool {
    let Some((_, _, headers)) = parse_request_line_and_headers(head) else {
        return false;
    };
    headers
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case("proxy-authorization"))
}

fn parse_source_from_headers(
    headers: &[(String, String)],
) -> (Option<String>, Option<String>, SourceIdentityStatus) {
    let auth = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("proxy-authorization"))
        .map(|(_, v)| v.as_str());
    let Some(auth) = auth else {
        return (None, None, SourceIdentityStatus::MissingProxyAuthorization);
    };
    decode_source_from_proxy_authorization(auth)
}

fn decode_source_from_proxy_authorization(
    value: &str,
) -> (Option<String>, Option<String>, SourceIdentityStatus) {
    let Some((scheme, payload)) = value.split_once(' ') else {
        return (None, None, SourceIdentityStatus::MalformedAuthHeader);
    };
    if !scheme.eq_ignore_ascii_case("basic") {
        return (None, None, SourceIdentityStatus::UnsupportedAuthScheme);
    }
    let decoded = match STANDARD.decode(payload.trim()) {
        Ok(bytes) => bytes,
        Err(_) => return (None, None, SourceIdentityStatus::InvalidBase64),
    };
    let creds = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => return (None, None, SourceIdentityStatus::InvalidUtf8),
    };
    let Some((username, password)) = creds.split_once(':') else {
        return (
            None,
            None,
            SourceIdentityStatus::MissingUsernamePasswordDelimiter,
        );
    };
    if username != "zcsrc" {
        return (None, None, SourceIdentityStatus::UnexpectedUsername);
    }
    let Some((project_enc, container_enc)) = password.split_once('.') else {
        return (
            None,
            None,
            SourceIdentityStatus::MissingProjectContainerDelimiter,
        );
    };
    let project = match URL_SAFE_NO_PAD.decode(project_enc.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(value) => value,
            Err(_) => return (None, None, SourceIdentityStatus::InvalidProjectEncoding),
        },
        Err(_) => return (None, None, SourceIdentityStatus::InvalidProjectEncoding),
    };
    let container = match URL_SAFE_NO_PAD.decode(container_enc.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(value) => value,
            Err(_) => return (None, None, SourceIdentityStatus::InvalidContainerEncoding),
        },
        Err(_) => return (None, None, SourceIdentityStatus::InvalidContainerEncoding),
    };
    (Some(project), Some(container), SourceIdentityStatus::Ok)
}

fn container_tls_passthrough_matches(
    config: &Config,
    source_container: Option<&str>,
    host: &str,
) -> bool {
    let Some(source_container) = source_container else {
        return false;
    };
    let Some(container) = config
        .containers
        .iter()
        .find(|c| c.name == source_container)
    else {
        return false;
    };
    container
        .bypass_proxy
        .iter()
        .any(|pattern| bypass_host_matches(pattern, host))
}

fn bypass_host_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    if pattern == "*" {
        return true;
    }

    let host_lc = host.to_ascii_lowercase();
    let pattern_lc = pattern.to_ascii_lowercase();

    if let Some(apex) = pattern_lc.strip_prefix('.') {
        return host_lc == apex || host_lc.ends_with(&format!(".{apex}"));
    }

    if let Some(apex) = pattern_lc.strip_prefix("*.") {
        return host_lc == apex || host_lc.ends_with(&format!(".{apex}"));
    }

    if !pattern_lc.contains('*') {
        return host_lc == pattern_lc;
    }

    Glob::new(&pattern_lc)
        .ok()
        .map(|g| g.compile_matcher().is_match(&host_lc))
        .unwrap_or(false)
}

fn extract_host(headers: &[(String, String)], path: &str) -> Option<String> {
    if let Some((_, v)) = headers.iter().find(|(n, _)| n.eq_ignore_ascii_case("host")) {
        return Some(strip_port(v.trim()));
    }
    if path.starts_with("http://") || path.starts_with("https://") {
        if let Ok(url) = path.parse::<url::Url>() {
            return url.host_str().map(|h| h.to_string());
        }
    }
    None
}

fn strip_port(host: &str) -> String {
    if host.starts_with('[') {
        if let Some(end) = host.find(']') {
            return host[1..end].to_string();
        }
        return host.to_string();
    }
    host.split(':').next().unwrap_or(host).to_string()
}

fn strip_scheme_and_host(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        if let Ok(url) = path.parse::<url::Url>() {
            let mut result = url.path().to_string();
            if let Some(q) = url.query() {
                result.push('?');
                result.push_str(q);
            }
            return result;
        }
    }
    path.to_string()
}

async fn write_response_any<W>(sink: &mut W, response: reqwest::Response) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let status = response.status().as_u16();
    let reason = response.status().canonical_reason().unwrap_or("Unknown");

    let resp_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter(|(name, _)| !is_hop_by_hop(name.as_str()))
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.to_string(), v.to_string()))
        })
        .collect();

    let content_length: Option<u64> = resp_headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok());

    let use_chunked = content_length.is_none();

    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in &resp_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if use_chunked {
        head.push_str("Transfer-Encoding: chunked\r\n");
    }
    head.push_str("Connection: close\r\n");
    head.push_str("\r\n");
    sink.write_all(head.as_bytes()).await?;

    let mut body_stream = response.bytes_stream();
    while let Some(chunk) = body_stream.next().await {
        let chunk = chunk?;
        if chunk.is_empty() {
            continue;
        }
        if use_chunked {
            sink.write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await?;
            sink.write_all(&chunk).await?;
            sink.write_all(b"\r\n").await?;
        } else {
            sink.write_all(&chunk).await?;
        }
    }
    if use_chunked {
        sink.write_all(b"0\r\n\r\n").await?;
    }
    Ok(())
}

async fn write_error_any<W>(sink: &mut W, code: u16, msg: &str) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let body = msg.as_bytes();
    let reason = StatusCode::from_u16(code)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        .canonical_reason()
        .unwrap_or("Unknown");

    let out = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut raw = out.into_bytes();
    raw.extend_from_slice(body);
    sink.write_all(&raw).await?;
    Ok(())
}

fn is_expected_disconnect(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_ascii_lowercase();
    msg.contains("close_notify")
        || msg.contains("unexpected eof")
        || msg.contains("connection reset by peer")
        || msg.contains("broken pipe")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn decode_source_from_proxy_authorization_works() {
        let auth_payload = format!("zcsrc:{}.{}", 
            URL_SAFE_NO_PAD.encode("myproj"), 
            URL_SAFE_NO_PAD.encode("mycont")
        );
        let header_value = format!("Basic {}", STANDARD.encode(auth_payload));
        let (project, container, status) = decode_source_from_proxy_authorization(&header_value);
        assert_eq!(status, SourceIdentityStatus::Ok);
        assert_eq!(project, Some("myproj".to_string()));
        assert_eq!(container, Some("mycont".to_string()));
    }

    #[test]
    fn bypass_host_matches_wildcards() {
        assert!(bypass_host_matches("*.google.com", "api.google.com"));
        assert!(bypass_host_matches("*.google.com", "google.com"));
        assert!(bypass_host_matches(".google.com", "api.google.com"));
        assert!(bypass_host_matches("google.com", "google.com"));
        assert!(!bypass_host_matches("google.com", "notgoogle.com"));
    }

    #[tokio::test]
    async fn prompt_network_sends_to_pending_tx() {
        let (_ca_tx, _ca_rx) = mpsc::channel::<()>(1); // dummy
        let (pending_tx, mut pending_rx) = mpsc::channel(1);
        let ca = Arc::new(CaStore::load_or_create(&std::env::temp_dir().join("proxy-test-ca")).unwrap());
        // Wait, I can just use build_test_app logic if I want but let's just make a dummy config.
        let raw = r#"
docker_dir = "/tmp"
[manager]
global_rules_file = "/tmp/global.toml"
[workspace]
root = "/tmp/ws"
"#;
        let cfg: crate::config::Config = toml::from_str(raw).unwrap();
        let state = ProxyState::new(ca, SharedConfig::new(Arc::new(cfg)), pending_tx).unwrap();

        let prompt_task = tokio::spawn(async move {
            prompt_network(&state, "GET", "example.com", "/test", Some("p".into()), Some("c".into()), "ok", true).await
        });

        // TUI side: receive the item
        let item = pending_rx.recv().await.expect("should receive pending item");
        assert_eq!(item.host, "example.com");
        
        // TUI side: allow it
        item.response_tx.send(NetworkDecision::Allow).unwrap();
        
        let result = prompt_task.await.unwrap();
        assert!(result, "prompt_network should return true for Allow");
    }
}
