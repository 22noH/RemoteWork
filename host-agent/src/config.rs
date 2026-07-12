use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host_id: String,
    /// One-time password: regenerated every launch and never written to disk,
    /// so a leaked password is useless after the session ends.
    #[serde(skip)]
    pub password: String,
    pub signaling_server_url: String,
    pub stun_servers: Vec<String>,
    pub turn_server: Option<TurnConfig>,
    pub allowed_dirs: Vec<PathBuf>,
    /// When false, the viewer can watch but not control (input is ignored).
    /// serde default keeps older config.json files working (defaults to true).
    #[serde(default = "default_allow_control")]
    pub allow_control: bool,
}

fn default_allow_control() -> bool {
    true
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
            Self::default_config()
        };

        // One-time password: freshly generated each launch, never persisted.
        config.password = auth::generate_password();

        // Env var overrides (take precedence over config file)
        if let Ok(url) = std::env::var("SIGNALING_URL") {
            config.signaling_server_url = url;
        }
        if let Ok(v) = std::env::var("ALLOW_CONTROL") {
            config.allow_control = !(v == "0" || v.eq_ignore_ascii_case("false"));
        }
        if config.turn_server.is_none() {
            config.turn_server = Self::turn_from_env();
        }

        // Persist host_id + settings (password is #[serde(skip)], so this also
        // strips any password left in an older config file).
        config.save()?;
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
            allow_control: true,
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
