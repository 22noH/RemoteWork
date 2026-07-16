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
    /// Listening address. If unset, uses $PORT (Render/Railway/Fly/Heroku
    /// inject it) as 0.0.0.0:$PORT, else 0.0.0.0:8080.
    #[arg(long, env = "LISTEN_ADDR")]
    listen: Option<String>,

    /// Path to TLS certificate (PEM). If set, enables WSS.
    #[arg(long, env = "TLS_CERT")]
    tls_cert: Option<String>,

    /// Path to TLS private key (PEM). Required if --tls-cert is set.
    #[arg(long, env = "TLS_KEY")]
    tls_key: Option<String>,

    /// Allow plaintext WS with no TLS. Viewer passwords then travel in
    /// cleartext — for local development only, never production.
    #[arg(long, env = "ALLOW_INSECURE")]
    insecure: bool,
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
    let listen = cli.listen.clone().unwrap_or_else(default_listen);
    tracing::info!("Starting signaling server on {}", listen);

    let registry = session_registry::SessionRegistry::new();

    if cli.tls_cert.is_some() || cli.tls_key.is_some() {
        let cert_path = cli
            .tls_cert
            .ok_or_else(|| anyhow::anyhow!("--tls-cert required when TLS is enabled"))?;
        let key_path = cli
            .tls_key
            .ok_or_else(|| anyhow::anyhow!("--tls-key required when TLS is enabled"))?;
        tracing::info!("TLS enabled (cert={}, key={})", cert_path, key_path);
        ws_server::run_server_tls(listen, registry, cert_path, key_path).await
    } else if cli.insecure {
        tracing::warn!(
            "TLS disabled (--insecure): viewer passwords travel in CLEARTEXT. Local development only — never production."
        );
        ws_server::run_server(listen, registry).await
    } else {
        anyhow::bail!(
            "Refusing to start without TLS — viewer passwords would travel in cleartext. \
             Provide --tls-cert and --tls-key, or pass --insecure for local development."
        );
    }
}

/// Managed hosts (Render/Railway/Fly/Heroku) inject the port via $PORT; fall
/// back to 8080 for local/dev and the deploy/ compose (which reaches it by name).
// ponytail: $PORT covers the common PaaS; --listen / LISTEN_ADDR overrides anything else.
fn default_listen() -> String {
    match std::env::var("PORT") {
        Ok(p) if !p.trim().is_empty() => format!("0.0.0.0:{}", p.trim()),
        _ => "0.0.0.0:8080".to_string(),
    }
}
