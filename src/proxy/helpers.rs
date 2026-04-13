use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use futures::StreamExt;
use globset::Glob;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::config::Config;
use crate::proxy::SourceIdentityStatus;
use crate::proxy::http::is_hop_by_hop;

pub(crate) fn parse_source_from_headers(
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

pub(crate) fn decode_source_from_proxy_authorization(
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

pub(crate) fn container_tls_passthrough_matches(
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

pub(crate) fn bypass_host_matches(pattern: &str, host: &str) -> bool {
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

pub(crate) fn extract_host(headers: &[(String, String)], path: &str) -> Option<String> {
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

pub(crate) fn strip_port(host: &str) -> String {
    if host.starts_with('[') {
        if let Some(end) = host.find(']') {
            return host[1..end].to_string();
        }
        return host.to_string();
    }
    host.split(':').next().unwrap_or(host).to_string()
}

pub(crate) fn strip_scheme_and_host(path: &str) -> String {
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

pub(crate) async fn write_response_any<W>(sink: &mut W, response: reqwest::Response) -> Result<()>
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

pub(crate) async fn write_error_any<W>(sink: &mut W, code: u16, msg: &str) -> Result<()>
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

pub(crate) fn is_expected_disconnect(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_ascii_lowercase();
    msg.contains("close_notify")
        || msg.contains("unexpected eof")
        || msg.contains("connection reset by peer")
        || msg.contains("broken pipe")
}
