use anyhow::Result;
use tokio::io::{AsyncWriteExt, copy_bidirectional};
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

use crate::config;
use crate::proxy::helpers::{
    container_tls_passthrough_matches, write_error_any, write_response_any,
};
use crate::proxy::http::{
    connect_head_has_proxy_authorization, forward_request, parse_connect_target,
    parse_request_line_and_headers, parse_source_from_connect_head, prompt_network, read_body_any,
    read_request_head_any,
};
use crate::proxy::{ProxyState, SourceIdentityStatus};
use crate::rules::NetworkPolicy;

pub(crate) fn parse_sni_from_tls_client_hello(record: &[u8]) -> Option<String> {
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

pub(crate) async fn handle_connect(mut stream: TcpStream, state: ProxyState) -> Result<()> {
    let (head, connect_remainder) = read_request_head_any(&mut stream).await?;
    let head_str = std::str::from_utf8(&head).unwrap_or("");

    let (host, port) = parse_connect_target(head_str)
        .ok_or_else(|| anyhow::anyhow!("malformed CONNECT request"))?;
    let (source_project, source_container, source_status, connect_has_proxy_authorization): (
        Option<String>,
        Option<String>,
        SourceIdentityStatus,
        bool,
    ) = if let Some(fixed) = &state.fixed_source {
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
            write_error_any(&mut stream, 500, "Invalid void-rules.toml configuration").await?;
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
