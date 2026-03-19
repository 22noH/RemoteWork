mod ws_server;
mod session_registry;
mod auth;
mod relay;
mod json_protocol;

use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "signaling-server", about = "Remote Work Signaling Server")]
struct Cli {
    /// Listening address
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:8080")]
    listen: String,

    /// Path to TLS certificate (PEM). If set, enables WSS.
    #[arg(long, env = "TLS_CERT")]
    tls_cert: Option<String>,

    /// Path to TLS private key (PEM). Required if --tls-cert is set.
    #[arg(long, env = "TLS_KEY")]
    tls_key: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("signaling_server=debug".parse()?),
        )
        .init();

    let cli = Cli::parse();
    tracing::info!("Starting signaling server on {}", cli.listen);

    let registry = session_registry::SessionRegistry::new();

    if cli.tls_cert.is_some() || cli.tls_key.is_some() {
        let cert_path = cli
            .tls_cert
            .ok_or_else(|| anyhow::anyhow!("--tls-cert required when TLS is enabled"))?;
        let key_path = cli
            .tls_key
            .ok_or_else(|| anyhow::anyhow!("--tls-key required when TLS is enabled"))?;
        tracing::info!("TLS enabled (cert={}, key={})", cert_path, key_path);
        ws_server::run_server_tls(cli.listen, registry, cert_path, key_path).await
    } else {
        tracing::info!("TLS disabled — plaintext WS");
        ws_server::run_server(cli.listen, registry).await
    }
}
