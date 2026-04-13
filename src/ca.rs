/// Certificate Authority management for the void-claw MITM proxy.
///
/// Generates a self-signed CA on first run and persists it to disk.
/// Derives per-domain leaf certificates on demand (cached in memory).
/// The CA cert PEM is exposed so it can be injected into containers.
use anyhow::{Context, Result};
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct CaStore {
    /// CA cert PEM — inject this into containers so they trust the proxy.
    pub cert_pem: String,
    ca_key: KeyPair,
    /// Reconstructed CA cert for signing leaf certs (may differ in validity
    /// period from the on-disk cert, but uses the same key and DN).
    ca_cert_for_signing: rcgen::Certificate,
    /// Original CA cert DER — included in leaf cert chains so TLS clients
    /// can verify the chain against what they imported.
    ca_cert_der: Vec<u8>,
    cert_cache: Mutex<HashMap<String, Arc<ServerConfig>>>,
}

impl CaStore {
    /// Load the CA from `dir`, or generate and persist a new one.
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let cert_path = dir.join("ca.crt");
        let key_path = dir.join("ca.key");

        if cert_path.exists() && key_path.exists() {
            return Self::load(&cert_path, &key_path);
        }
        Self::generate_and_save(&cert_path, &key_path)
    }

    fn load(cert_path: &Path, key_path: &Path) -> Result<Self> {
        let cert_pem = std::fs::read_to_string(cert_path)
            .with_context(|| format!("reading {}", cert_path.display()))?;
        let key_pem = std::fs::read_to_string(key_path)
            .with_context(|| format!("reading {}", key_path.display()))?;

        let ca_key = KeyPair::from_pem(&key_pem).context("parsing CA private key")?;

        // Reconstruct a signable Certificate from the same DN (fixed values).
        let ca_cert_for_signing = Self::build_ca_cert(&ca_key)?;

        // Extract the original DER bytes from the PEM for chain inclusion.
        let ca_cert_der = Self::pem_to_der(&cert_pem)?;

        Ok(Self {
            cert_pem,
            ca_key,
            ca_cert_for_signing,
            ca_cert_der,
            cert_cache: Mutex::new(HashMap::new()),
        })
    }

    fn generate_and_save(cert_path: &Path, key_path: &Path) -> Result<Self> {
        let ca_key = KeyPair::generate().context("generating CA key pair")?;
        let ca_cert = Self::build_ca_cert(&ca_key)?;

        let cert_pem = ca_cert.pem();
        let key_pem = ca_key.serialize_pem();
        let ca_cert_der = ca_cert.der().to_vec();

        std::fs::write(cert_path, &cert_pem)
            .with_context(|| format!("writing {}", cert_path.display()))?;
        std::fs::write(key_path, &key_pem)
            .with_context(|| format!("writing {}", key_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("setting permissions on {}", key_path.display()))?;
        }

        Ok(Self {
            cert_pem,
            ca_key,
            ca_cert_for_signing: ca_cert,
            ca_cert_der,
            cert_cache: Mutex::new(HashMap::new()),
        })
    }

    fn build_ca_cert(key: &KeyPair) -> Result<rcgen::Certificate> {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, "Void Claw Proxy CA");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "void-claw");
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2124, 1, 1);
        params.self_signed(key).context("generating CA certificate")
    }

    fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
        let mut buf = pem.as_bytes();
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut buf)
            .collect::<std::result::Result<_, _>>()
            .context("parsing CA cert PEM")?;
        certs
            .into_iter()
            .next()
            .map(|c: CertificateDer<'static>| c.to_vec())
            .context("no certificate found in CA PEM")
    }

    /// Return (or generate and cache) a rustls `ServerConfig` presenting a
    /// leaf certificate for `domain`, signed by this CA.
    pub fn leaf_server_config(&self, domain: &str) -> Result<Arc<ServerConfig>> {
        {
            let cache = self.cert_cache.lock().unwrap();
            if let Some(cfg) = cache.get(domain) {
                return Ok(Arc::clone(cfg));
            }
        }

        let leaf_key = KeyPair::generate().context("generating leaf key")?;
        let mut params =
            CertificateParams::new(vec![domain.to_string()]).context("building leaf params")?;
        params.is_ca = IsCa::NoCa;
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(2034, 1, 1);

        let leaf_cert = params
            .signed_by(&leaf_key, &self.ca_cert_for_signing, &self.ca_key)
            .context("signing leaf certificate")?;

        // Chain: leaf + original CA cert (what the container's trust store knows).
        let cert_chain: Vec<CertificateDer<'static>> = vec![
            CertificateDer::from(leaf_cert.der().to_vec()),
            CertificateDer::from(self.ca_cert_der.clone()),
        ];

        // Private key for the leaf cert.
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));

        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key_der)
            .context("building leaf ServerConfig")?;

        let config = Arc::new(server_config);
        self.cert_cache
            .lock()
            .unwrap()
            .insert(domain.to_string(), config.clone());
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_or_create_is_idempotent() {
        let dir = tempdir().expect("create temp dir");
        let path = dir.path();

        // 1. Create for the first time
        let store1 = CaStore::load_or_create(path).expect("first create");
        let cert1 = store1.cert_pem.clone();
        assert!(path.join("ca.crt").exists());
        assert!(path.join("ca.key").exists());

        // 2. Load again from the same dir
        let store2 = CaStore::load_or_create(path).expect("second load");
        assert_eq!(
            store2.cert_pem, cert1,
            "CA certificate should be persistent"
        );
    }

    #[test]
    fn leaf_server_config_caches_results() {
        let dir = tempdir().expect("create temp dir");
        let store = CaStore::load_or_create(dir.path()).expect("create store");

        let config1 = store.leaf_server_config("example.com").expect("first leaf");
        let config2 = store
            .leaf_server_config("example.com")
            .expect("second leaf");

        assert!(
            Arc::ptr_eq(&config1, &config2),
            "server configs should be cached"
        );

        let config3 = store
            .leaf_server_config("other.com")
            .expect("different domain");
        assert!(
            !Arc::ptr_eq(&config1, &config3),
            "different domains should have different configs"
        );
    }
}
