use crate::app::Shared;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Small always-on host window: shows credentials, connection status, and a
/// live view-only toggle. Runs on the main thread (eframe owns the event loop);
/// the tokio network stack runs on a background thread and communicates through
/// the shared atomics in `Shared`.
pub fn run(host_id: String, password: String, shared: Shared) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([340.0, 250.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Remote Work",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(HostUi {
                host_id,
                password,
                shared,
            }))
        }),
    )
}

struct HostUi {
    host_id: String,
    password: String,
    shared: Shared,
}

impl eframe::App for HostUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Status/viewer-count is updated from another thread; repaint so it stays live.
        ctx.request_repaint_after(Duration::from_millis(500));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            ui.heading("Remote Work");
            ui.separator();

            egui::Grid::new("creds").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label("ID");
                ui.add(egui::Label::new(egui::RichText::new(&self.host_id).size(18.0).monospace()).selectable(true));
                ui.end_row();
                ui.label("Password");
                ui.add(egui::Label::new(egui::RichText::new(&self.password).size(18.0).monospace()).selectable(true));
                ui.end_row();
            });
            ui.label(egui::RichText::new("Read these to the person connecting.").small().weak());

            ui.separator();

            let count = self.shared.viewer_count.load(Ordering::Relaxed);
            let status = if count == 0 {
                "● Waiting for connection".to_owned()
            } else {
                format!("● {count} connected")
            };
            ui.label(egui::RichText::new(status).strong());

            let mut allow = self.shared.allow_control.load(Ordering::Relaxed);
            if ui.checkbox(&mut allow, "Allow remote control").changed() {
                self.shared.allow_control.store(allow, Ordering::Relaxed);
            }

            ui.separator();
            if ui.button("Disconnect all").clicked() {
                self.shared.disconnect_all.store(true, Ordering::Relaxed);
            }
        });
    }
}
