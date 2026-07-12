mod app;
mod config;
mod ui;

use app::Shared;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize},
    Arc, OnceLock,
};
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("host_agent=debug".parse()?),
        )
        .init();

    let config = config::Config::load()?;
    tracing::info!("Starting host agent");
    tracing::info!("Your ID:       {}", config.host_id);
    tracing::info!("Your password: {}", config.password);

    let host_id = config.host_id.clone();
    let password = config.password.clone();

    let shared = Shared {
        allow_control: Arc::new(AtomicBool::new(config.allow_control)),
        viewer_count: Arc::new(AtomicUsize::new(0)),
        disconnect_all: Arc::new(AtomicBool::new(false)),
        pending_approval: Arc::new(AtomicBool::new(false)),
        approval_decision: Arc::new(AtomicU8::new(0)),
        ctx: Arc::new(OnceLock::new()),
        chat_log: Arc::new(std::sync::Mutex::new(Vec::new())),
        chat_send: Arc::new(std::sync::Mutex::new(None)),
        chat_input: Arc::new(std::sync::Mutex::new(String::new())),
        chat_open: Arc::new(AtomicBool::new(false)),
        pending_file: Arc::new(std::sync::Mutex::new(None)),
        file_decision: Arc::new(AtomicU8::new(0)),
    };

    // The network stack is async; egui must own the main thread. Run tokio on a
    // background thread and let both sides talk through `shared`.
    let app_shared = shared.clone();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to start tokio runtime: {}", e);
                return;
            }
        };
        rt.block_on(async move {
            if let Err(e) = app::App::new(config, app_shared).run().await {
                tracing::error!("App exited with error: {}", e);
            }
        });
    });

    // Blocks until the window is closed; process exit then tears down the runtime.
    ui::run(host_id, password, shared).map_err(|e| anyhow::anyhow!("UI error: {e}"))
}
