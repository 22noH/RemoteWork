use crate::app::Shared;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::time::Duration;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(59, 130, 246); // blue-500
const GREEN: egui::Color32 = egui::Color32::from_rgb(34, 197, 94);
const DANGER: egui::Color32 = egui::Color32::from_rgb(220, 38, 38);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(247, 248, 250);
const CARD_STROKE: egui::Color32 = egui::Color32::from_rgb(228, 231, 236);
const LABEL: egui::Color32 = egui::Color32::from_rgb(120, 128, 140);
const VALUE: egui::Color32 = egui::Color32::from_rgb(24, 28, 35);

/// Small always-on host window: credentials, connection status, live view-only
/// toggle. Runs on the main thread (eframe owns the event loop); the tokio
/// network stack runs on a background thread and talks through `Shared`.
pub fn run(host_id: String, password: String, shared: Shared) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([380.0, 470.0])
            .with_min_inner_size([380.0, 470.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Remote Work",
        options,
        Box::new(move |cc| {
            style(&cc.egui_ctx);
            Ok(Box::new(HostUi {
                host_id,
                password,
                shared,
            }))
        }),
    )
}

fn style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = egui::Color32::WHITE;
    visuals.selection.bg_fill = ACCENT;
    visuals.widgets.hovered.rounding = 6.0.into();
    visuals.widgets.active.rounding = 6.0.into();
    visuals.widgets.inactive.rounding = 6.0.into();
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    ctx.set_style(style);
}

struct HostUi {
    host_id: String,
    password: String,
    shared: Shared,
}

impl eframe::App for HostUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(18.0)).fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🖥").size(22.0));
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Remote Work").size(19.0).strong().color(VALUE));
                        ui.label(egui::RichText::new("Remote support session").size(11.0).color(LABEL));
                    });
                });

                ui.add_space(14.0);

                // Credentials card
                card(ui, |ui| {
                    cred_row(ui, "ID", &group_id(&self.host_id), &self.host_id);
                    ui.add_space(10.0);
                    cred_row(ui, "PASSWORD", &self.password, &self.password);
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Share these with the person connecting.").size(11.0).color(LABEL));

                ui.add_space(14.0);

                // Status
                let count = self.shared.viewer_count.load(Ordering::Relaxed);
                ui.horizontal(|ui| {
                    let (dot, text) = if count == 0 {
                        (LABEL, "Waiting for connection".to_owned())
                    } else {
                        (GREEN, format!("{count} connected"))
                    };
                    dot_indicator(ui, dot);
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(text).size(14.0).strong().color(VALUE));
                });

                ui.add_space(12.0);

                // View-only toggle card
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut allow = self.shared.allow_control.load(Ordering::Relaxed);
                        if toggle(ui, &mut allow).changed() {
                            self.shared.allow_control.store(allow, Ordering::Relaxed);
                        }
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Allow remote control").size(14.0).color(VALUE));
                            let sub = if allow {
                                "Viewer can control your PC"
                            } else {
                                "View-only — viewer cannot control"
                            };
                            ui.label(egui::RichText::new(sub).size(11.0).color(LABEL));
                        });
                    });
                });

                // Bottom: disconnect
                ui.add_space(16.0);
                let btn = egui::Button::new(egui::RichText::new("Disconnect all").color(egui::Color32::WHITE).size(13.0))
                    .fill(if count == 0 { egui::Color32::from_rgb(203, 210, 217) } else { DANGER })
                    .rounding(7.0)
                    .min_size(egui::vec2(ui.available_width(), 34.0));
                if ui.add_enabled(count > 0, btn).clicked() {
                    self.shared.disconnect_all.store(true, Ordering::Relaxed);
                }
            });
    }
}

/// A rounded light card container.
fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, CARD_STROKE))
        .rounding(10.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// One credential row: small label above a big monospace value + Copy button.
fn cred_row(ui: &mut egui::Ui, label: &str, display: &str, copy_value: &str) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(label).size(10.0).color(LABEL));
            ui.label(egui::RichText::new(display).size(22.0).monospace().strong().color(VALUE));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(egui::RichText::new("Copy").size(12.0)).clicked() {
                ui.output_mut(|o| o.copied_text = copy_value.to_owned());
            }
        });
    });
}

/// A colored status dot.
fn dot_indicator(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

/// A pill toggle switch (standard egui custom widget).
fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired = egui::vec2(40.0, 22.0);
    let (rect, mut resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let how_on = ui.ctx().animate_bool(resp.id, *on);
    let radius = 0.5 * rect.height();
    let track = if *on { ACCENT } else { egui::Color32::from_rgb(200, 205, 212) };
    ui.painter().rect_filled(rect, radius, track);
    let cx = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
    ui.painter()
        .circle_filled(egui::pos2(cx, rect.center().y), radius - 3.0, egui::Color32::WHITE);
    resp
}

/// Group an all-digit id into blocks of 3 for readability (994379228 -> 994 379 228).
fn group_id(id: &str) -> String {
    if !id.chars().all(|c| c.is_ascii_digit()) {
        return id.to_owned();
    }
    let mut out = String::new();
    for (i, c) in id.chars().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}
