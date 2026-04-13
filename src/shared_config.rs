use crate::config::Config;
use std::sync::{Arc, RwLock};

/// Thread-safe hot-reloadable handle to the current config.
///
/// This is used so the TUI can update the config at runtime and the hostdo
/// server + proxy can see the new project list without restart.
#[derive(Clone)]
pub struct SharedConfig {
    inner: Arc<RwLock<Arc<Config>>>,
}

impl SharedConfig {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(config)),
        }
    }

    pub fn get(&self) -> Arc<Config> {
        self.inner.read().expect("config lock poisoned").clone()
    }

    pub fn set(&self, config: Arc<Config>) {
        *self.inner.write().expect("config lock poisoned") = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::Arc;

    #[test]
    fn shared_config_hot_reloads() {
        let config1 = Arc::new(Config::default());
        let shared = SharedConfig::new(config1);

        let config2 = Arc::new(Config {
            docker_dir: std::path::PathBuf::from("/new/docker"),
            ..Config::default()
        });

        shared.set(config2);

        let current = shared.get();
        assert_eq!(current.docker_dir, std::path::PathBuf::from("/new/docker"));
    }

    #[test]
    fn shared_config_clones_independent_reference() {
        let config1 = Arc::new(Config::default());
        let shared1 = SharedConfig::new(config1);
        let shared2 = shared1.clone();

        let config2 = Arc::new(Config {
            docker_dir: std::path::PathBuf::from("/shared/docker"),
            ..Config::default()
        });

        shared1.set(config2);

        // Both clones should see the update
        assert_eq!(
            shared2.get().docker_dir,
            std::path::PathBuf::from("/shared/docker")
        );
    }
}
