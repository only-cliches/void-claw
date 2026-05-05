/// MITM HTTP/HTTPS proxy enforcing network policies from harness-rules.toml.
///
/// Containers route all traffic through this proxy. Plain HTTP requests are
/// intercepted and parsed directly. HTTPS traffic is intercepted via CONNECT
/// tunnels: the proxy terminates TLS with a per-domain leaf cert signed by
/// the harness-hat CA (which containers are configured to trust), inspects the
/// inner HTTP request, then forwards to the real server.
///
/// Network policy (auto/prompt/deny) is determined by matching the composed
/// rules against method + host + path of each request.
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

use crate::activity::{
    Activity, ActivityEvent, ActivityKind, ActivityState, payload_preview, wait_cancelled,
};
use crate::ca::CaStore;
use crate::config;
use crate::proxy::connect::{handle_connect, parse_sni_from_tls_client_hello};
use crate::proxy::helpers::{
    container_tls_passthrough_matches, is_expected_disconnect, write_error_any,
};
use crate::proxy::http::{
    forward_request_with_activity, handle_plain_http, parse_request_line_and_headers,
    prompt_network, read_body_any, read_request_head_any,
};
use crate::rules::NetworkPolicy;
use crate::shared_config::SharedConfig;
use tracing::instrument;

/// A network request waiting on the TUI for an allow/deny decision.
pub struct PendingNetworkItem {
    pub activity_id: String,
    pub cancel_flag: Arc<std::sync::atomic::AtomicBool>,
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
pub(crate) struct FixedSourceIdentity {
    pub(crate) project: String,
    pub(crate) container: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceIdentityStatus {
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
    pub(crate) fn as_str(self) -> &'static str {
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
    pub activity_tx: mpsc::UnboundedSender<ActivityEvent>,
    pub(crate) client: reqwest::Client,
    pub(crate) fixed_source: Option<FixedSourceIdentity>,
}

impl ProxyState {
    pub fn new(
        ca: Arc<CaStore>,
        config: SharedConfig,
        pending_tx: mpsc::Sender<PendingNetworkItem>,
        activity_tx: mpsc::UnboundedSender<ActivityEvent>,
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
            activity_tx,
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

impl ProxyState {
    pub(crate) fn start_network_activity(
        &self,
        source_project: Option<String>,
        source_container: Option<String>,
        method: &str,
        host: &str,
        path: &str,
        protocol: &str,
        headers: &[(String, String)],
        body: &[u8],
        state: ActivityState,
    ) -> Activity {
        let cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (payload_preview, payload_truncated) = payload_preview(body);
        let content_type = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.clone());
        let content_length = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok());
        let activity = Activity::new(
            source_project.unwrap_or_else(|| "unknown-workspace".to_string()),
            source_container,
            ActivityKind::Network {
                method: method.to_string(),
                host: host.to_string(),
                path: path.to_string(),
                protocol: protocol.to_string(),
                payload_preview,
                payload_truncated,
                content_type,
                content_length,
            },
            state,
            cancel_flag,
        );
        let _ = self
            .activity_tx
            .send(ActivityEvent::Started(activity.clone()));
        activity
    }

    pub(crate) fn activity_state(
        &self,
        id: &str,
        state: ActivityState,
        status: impl Into<Option<String>>,
    ) {
        let _ = self.activity_tx.send(ActivityEvent::State {
            id: id.to_string(),
            state,
            status: status.into(),
        });
    }

    pub(crate) fn activity_line(&self, id: &str, line: impl Into<String>) {
        let _ = self.activity_tx.send(ActivityEvent::Line {
            id: id.to_string(),
            line: line.into(),
        });
    }

    pub(crate) fn activity_finished(
        &self,
        id: &str,
        state: ActivityState,
        status: impl Into<Option<String>>,
    ) {
        let _ = self.activity_tx.send(ActivityEvent::Finished {
            id: id.to_string(),
            state,
            status: status.into(),
        });
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

#[instrument(skip(state))]
pub async fn run(state: ProxyState, addr: String) -> Result<()> {
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("proxy bind {addr}: {e}"))?;
    run_with_listener(state, listener).await
}

#[instrument(skip(state, listener))]
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
#[instrument(skip(state))]
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

    let connect_activity = state.start_network_activity(
        source_project.clone(),
        source_container.clone(),
        "CONNECT",
        &host,
        "/",
        "transparent-tls",
        &[],
        &[],
        ActivityState::Forwarding,
    );
    state.activity_line(&connect_activity.id, format!("target {host}:443"));

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
        state.activity_state(
            &connect_activity.id,
            ActivityState::Forwarding,
            Some(format!("tunneling {host}:443")),
        );
        tokio::select! {
            result = copy_bidirectional(&mut stream, &mut upstream) => match result {
                Ok((from_client, from_server)) => {
                    state.activity_line(
                        &connect_activity.id,
                        format!("tunnel closed after {from_client} bytes upstream, {from_server} bytes downstream"),
                    );
                    state.activity_finished(
                        &connect_activity.id,
                        ActivityState::Complete,
                        Some("tunnel closed".to_string()),
                    );
                }
                Err(e) => {
                    state.activity_finished(
                        &connect_activity.id,
                        ActivityState::Failed,
                        Some(e.to_string()),
                    );
                }
            },
            _ = wait_cancelled(connect_activity.cancel_flag.clone()) => {
                state.activity_finished(
                    &connect_activity.id,
                    ActivityState::Cancelled,
                    Some("cancelled".to_string()),
                );
            }
        }
        return Ok(());
    }

    let rules = match config::load_composed_rules_for_workspace(&cfg, source_project.as_deref()) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("proxy rules load error: {e}");
            state.activity_finished(
                &connect_activity.id,
                ActivityState::Failed,
                Some("invalid harness-rules.toml configuration".to_string()),
            );
            return Ok(());
        }
    };
    let preflight_policy = rules.match_network("CONNECT", &host, "/");
    let preflight_allowed = match preflight_policy {
        NetworkPolicy::Auto => true,
        NetworkPolicy::Deny => false,
        NetworkPolicy::Prompt => {
            state.activity_state(
                &connect_activity.id,
                ActivityState::PendingApproval,
                Some("waiting for CONNECT approval".to_string()),
            );
            prompt_network(
                &state,
                "CONNECT",
                &host,
                "/",
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
                Some(&connect_activity),
            )
            .await
        }
    };
    if !preflight_allowed {
        let state_label = if connect_activity.is_cancelled() {
            ActivityState::Cancelled
        } else {
            ActivityState::Denied
        };
        state.activity_finished(
            &connect_activity.id,
            state_label,
            Some("blocked by network policy".to_string()),
        );
        return Ok(());
    }

    let prefixed = PrefixedTcpStream {
        prefix: std::io::Cursor::new(prefix),
        inner: stream,
    };

    let server_config = state.ca.leaf_server_config(&host)?;
    let acceptor = TlsAcceptor::from(server_config);
    let mut tls_stream = acceptor.accept(prefixed).await.map_err(|e| {
        state.activity_finished(
            &connect_activity.id,
            ActivityState::Failed,
            Some(format!("TLS accept for {host}: {e}")),
        );
        anyhow::anyhow!("TLS accept for {host}: {e}")
    })?;

    debug!("proxy TLS established for host={host} (transparent)");
    state.activity_finished(
        &connect_activity.id,
        ActivityState::Complete,
        Some("TLS tunnel established".to_string()),
    );

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
    let activity = state.start_network_activity(
        source_project.clone(),
        source_container.clone(),
        &method,
        &host,
        &path,
        "https",
        &headers,
        &body,
        ActivityState::Forwarding,
    );
    state.activity_line(&activity.id, "request body read");

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
            state.activity_state(
                &activity.id,
                ActivityState::PendingApproval,
                Some("waiting for network approval".to_string()),
            );
            prompt_network(
                &state,
                &method,
                &host,
                &path,
                source_project.clone(),
                source_container.clone(),
                source_status.as_str(),
                has_proxy_authorization,
                Some(&activity),
            )
            .await
        }
    };
    if !allowed {
        let state_label = if activity.is_cancelled() {
            ActivityState::Cancelled
        } else {
            ActivityState::Denied
        };
        state.activity_finished(
            &activity.id,
            state_label,
            Some("blocked by network policy".to_string()),
        );
        write_error_any(&mut tls_stream, 403, "Forbidden by harness-hat policy").await?;
        return Ok(());
    }
    forward_request_with_activity(
        &state,
        &mut tls_stream,
        &activity,
        "https",
        &host,
        &path,
        &method,
        &headers,
        body,
    )
    .await
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
