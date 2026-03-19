use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use tokio::sync::mpsc;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Debug)]
pub enum TrayMessage {
    Quit,
    DisconnectAll,
}

pub struct SystemTray {
    _tray: tray_icon::TrayIcon,
    pub event_rx: mpsc::UnboundedReceiver<TrayMessage>,
    pub viewer_count: Arc<AtomicUsize>,
}

impl SystemTray {
    pub fn new(host_id: &str, _password: &str) -> Result<Self> {
        // Create a simple 16x16 blue icon
        let icon_rgba: Vec<u8> = (0..16 * 16)
            .flat_map(|_| [0u8, 140u8, 200u8, 255u8])
            .collect();
        let icon = tray_icon::Icon::from_rgba(icon_rgba, 16, 16)?;

        let id_item = MenuItem::new(format!("ID: {}", host_id), false, None);
        let separator1 = MenuItem::new("---", false, None);
        let disconnect_all = MenuItem::new("Disconnect All", true, None);
        let separator2 = MenuItem::new("---", false, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let menu = Menu::new();
        menu.append(&id_item)?;
        menu.append(&separator1)?;
        menu.append(&disconnect_all)?;
        menu.append(&separator2)?;
        menu.append(&quit_item)?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(format!("Remote Work - ID: {}", host_id))
            .with_icon(icon)
            .build()?;

        let (tx, rx) = mpsc::unbounded_channel::<TrayMessage>();
        let viewer_count = Arc::new(AtomicUsize::new(0));

        let disconnect_id = disconnect_all.id().clone();
        let quit_id = quit_item.id().clone();

        std::thread::spawn(move || {
            let menu_channel = MenuEvent::receiver();
            loop {
                if let Ok(event) = menu_channel.try_recv() {
                    if event.id == quit_id {
                        let _ = tx.send(TrayMessage::Quit);
                        break;
                    } else if event.id == disconnect_id {
                        let _ = tx.send(TrayMessage::DisconnectAll);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        Ok(Self {
            _tray: tray,
            event_rx: rx,
            viewer_count,
        })
    }

    pub fn set_viewer_count(&self, count: usize) {
        self.viewer_count.store(count, Ordering::Relaxed);
    }
}
