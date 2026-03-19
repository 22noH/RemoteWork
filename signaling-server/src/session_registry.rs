use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub type SessionToken = String;

#[derive(Debug, Clone)]
pub struct HostEntry {
    pub host_id: String,
    pub password_hash: String,
    pub client_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub failed_attempts: u32,
    pub blocked_until: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
pub struct ViewerEntry {
    pub viewer_session_id: String,
    pub client_tx: mpsc::UnboundedSender<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub host_id: String,
    pub viewer_session_id: String,
    pub session_token: SessionToken,
}

pub struct SessionRegistry {
    pub hosts: Arc<DashMap<String, HostEntry>>,
    pub viewers: Arc<DashMap<String, ViewerEntry>>,
    pub sessions: Arc<DashMap<SessionToken, ActiveSession>>,
}

impl SessionRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            hosts: Arc::new(DashMap::new()),
            viewers: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
        })
    }

    pub fn register_host(
        &self,
        host_id: String,
        password_hash: String,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        self.hosts.insert(
            host_id.clone(),
            HostEntry {
                host_id,
                password_hash,
                client_tx: tx,
                failed_attempts: 0,
                blocked_until: None,
            },
        );
    }

    pub fn register_viewer(&self, viewer_session_id: String, tx: mpsc::UnboundedSender<Vec<u8>>) {
        self.viewers.insert(
            viewer_session_id.clone(),
            ViewerEntry { viewer_session_id, client_tx: tx },
        );
    }

    pub fn get_host_tx(&self, host_id: &str) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        self.hosts.get(host_id).map(|h| h.client_tx.clone())
    }

    pub fn get_viewer_tx(&self, viewer_session_id: &str) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        self.viewers.get(viewer_session_id).map(|v| v.client_tx.clone())
    }

    pub fn remove_host(&self, host_id: &str) {
        self.hosts.remove(host_id);
    }

    pub fn remove_viewer(&self, viewer_session_id: &str) {
        self.viewers.remove(viewer_session_id);
        self.sessions
            .retain(|_, s| s.viewer_session_id != viewer_session_id);
    }

    pub fn create_session(&self, host_id: String, viewer_session_id: String) -> SessionToken {
        let token = Uuid::new_v4().to_string();
        self.sessions.insert(
            token.clone(),
            ActiveSession {
                host_id,
                viewer_session_id,
                session_token: token.clone(),
            },
        );
        token
    }

    pub fn get_session_peer_tx(
        &self,
        session_token: &str,
        sender_is_host: bool,
    ) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        let session = self.sessions.get(session_token)?;
        if sender_is_host {
            self.get_viewer_tx(&session.viewer_session_id)
        } else {
            self.get_host_tx(&session.host_id)
        }
    }

    pub fn check_rate_limit(&self, host_id: &str) -> bool {
        if let Some(host) = self.hosts.get(host_id) {
            if let Some(blocked_until) = host.blocked_until {
                return std::time::Instant::now() < blocked_until;
            }
        }
        false
    }

    pub fn record_failed_attempt(&self, host_id: &str) {
        if let Some(mut host) = self.hosts.get_mut(host_id) {
            host.failed_attempts += 1;
            if host.failed_attempts >= 5 {
                host.blocked_until = Some(
                    std::time::Instant::now() + std::time::Duration::from_secs(600),
                );
                host.failed_attempts = 0;
                tracing::warn!(
                    "Host {} blocked for 10 minutes due to failed attempts",
                    host_id
                );
            }
        }
    }

    pub fn reset_failed_attempts(&self, host_id: &str) {
        if let Some(mut host) = self.hosts.get_mut(host_id) {
            host.failed_attempts = 0;
            host.blocked_until = None;
        }
    }
}
