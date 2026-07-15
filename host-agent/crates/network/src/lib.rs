pub mod signaling_client;
pub mod peer_connection;

pub use signaling_client::{SignalingClient, SignalingEvent};
pub use peer_connection::{DataChannelHandlers, HostPeerConnection};

/// Minimal config passed from host-agent to the network crate
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub host_id: String,
    pub password: String,
    pub signaling_server_url: String,
    pub stun_servers: Vec<String>,
}
