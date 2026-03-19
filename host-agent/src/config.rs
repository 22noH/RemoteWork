use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host_id: String,
    pub password: String,
    pub signaling_server_url: String,
    pub stun_servers: Vec<String>,
    pub turn_server: Option<TurnConfig>,
    pub allowed_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnConfig {
    pub url: String,
    pub username: String,
    pub credential: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path();
        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&content)?
        } else {
            let config = Self::default_config();
            config.save()?;
            config
        };

        // Env var overrides (take precedence over config file)
        if let Ok(url) = std::env::var("SIGNALING_URL") {
            config.signaling_server_url = url;
        }
        if config.turn_server.is_none() {
            config.turn_server = Self::turn_from_env();
        }

        Ok(config)
    }

    fn default_config() -> Self {
        let (host_id, password) = auth::generate_credentials();
        Self {
            host_id,
            password,
            signaling_server_url: std::env::var("SIGNALING_URL")
                .unwrap_or_else(|_| "ws://localhost:8080".to_string()),
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_server: Self::turn_from_env(),
            allowed_dirs: vec![
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            ],
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Build TurnConfig from TURN_URL, TURN_USERNAME, TURN_CREDENTIAL env vars.
    /// Returns None if TURN_URL is not set.
    fn turn_from_env() -> Option<TurnConfig> {
        let url = std::env::var("TURN_URL").ok()?;
        Some(TurnConfig {
            url,
            username: std::env::var("TURN_USERNAME").unwrap_or_default(),
            credential: std::env::var("TURN_CREDENTIAL").unwrap_or_default(),
        })
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("remote-work")
            .join("config.json")
    }
}
