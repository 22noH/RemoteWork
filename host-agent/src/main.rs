mod app;
mod config;
mod tray;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("host_agent=debug".parse()?),
        )
        .init();

    let config = config::Config::load()?;
    tracing::info!("Starting host agent");
    tracing::info!("Your ID:       {}", config.host_id);
    tracing::debug!("Your password: {}", config.password);
    tracing::info!("Share these credentials with the viewer.");

    // Initialize system tray (best-effort; non-fatal if tray fails)
    match tray::SystemTray::new(&config.host_id, &config.password) {
        Ok(mut system_tray) => {
            let app_future = app::App::new(config).run();

            tokio::select! {
                result = app_future => result?,
                Some(msg) = system_tray.event_rx.recv() => {
                    match msg {
                        tray::TrayMessage::Quit => {
                            tracing::info!("Quit via tray");
                        }
                        tray::TrayMessage::DisconnectAll => {
                            tracing::info!("Disconnect all via tray (not yet implemented)");
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!("System tray unavailable: {}. Running without tray.", e);
            app::App::new(config).run().await?;
        }
    }

    Ok(())
}
