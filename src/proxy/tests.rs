#[cfg(test)]
mod tests {
    use crate::ca::CaStore;
    use crate::proxy::core::{NetworkDecision, ProxyState, SourceIdentityStatus};
    use crate::proxy::helpers::{bypass_host_matches, decode_source_from_proxy_authorization};
    use crate::proxy::http::prompt_network;
    use crate::shared_config::SharedConfig;
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[test]
    fn decode_source_from_proxy_authorization_works() {
        let auth_payload = format!(
            "zcsrc:{}.{}",
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
        let ca =
            Arc::new(CaStore::load_or_create(&std::env::temp_dir().join("proxy-test-ca")).unwrap());
        // Wait, I can just use build_test_app logic if I want but let's just make a dummy config.
        let raw = r#"
docker_dir = "/tmp"
[workspace]

[manager]
global_rules_file = "/tmp/global.toml""#;
        let cfg: crate::config::Config = toml::from_str(raw).unwrap();
        let state = ProxyState::new(ca, SharedConfig::new(Arc::new(cfg)), pending_tx).unwrap();

        let prompt_task = tokio::spawn(async move {
            prompt_network(
                &state,
                "GET",
                "example.com",
                "/test",
                Some("p".into()),
                Some("c".into()),
                "ok",
                true,
            )
            .await
        });

        // TUI side: receive the item
        let item = pending_rx
            .recv()
            .await
            .expect("should receive pending item");
        assert_eq!(item.host, "example.com");

        // TUI side: allow it
        item.response_tx.send(NetworkDecision::Allow).unwrap();

        let result = prompt_task.await.unwrap();
        assert!(result, "prompt_network should return true for Allow");
    }
}
