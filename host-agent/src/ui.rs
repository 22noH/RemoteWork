use crate::app::Shared;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIcon, TrayIconBuilder,
};

const ACCENT: egui::Color32 = egui::Color32::from_rgb(59, 130, 246); // blue-500
const GREEN: egui::Color32 = egui::Color32::from_rgb(34, 197, 94);
const DANGER: egui::Color32 = egui::Color32::from_rgb(220, 38, 38);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(247, 248, 250);
const CARD_STROKE: egui::Color32 = egui::Color32::from_rgb(228, 231, 236);
const LABEL: egui::Color32 = egui::Color32::from_rgb(120, 128, 140);
const VALUE: egui::Color32 = egui::Color32::from_rgb(24, 28, 35);

/// UI strings for the two supported languages (chosen from the OS locale).
struct Strings {
    subtitle: &'static str,
    password_label: &'static str,
    share: &'static str,
    waiting: &'static str,
    allow: &'static str,
    on_sub: &'static str,
    off_sub: &'static str,
    disconnect: &'static str,
    tray_open: &'static str,
    tray_quit: &'static str,
    approval_title: &'static str,
    approval_body: &'static str,
    approve: &'static str,
    deny: &'static str,
}

const EN: Strings = Strings {
    subtitle: "Remote support session",
    password_label: "PASSWORD",
    share: "Share these with the person connecting.",
    waiting: "Waiting for connection",
    allow: "Allow remote control",
    on_sub: "Viewer can control your PC",
    off_sub: "View-only — viewer cannot control",
    disconnect: "Disconnect all",
    tray_open: "Open",
    tray_quit: "Quit",
    approval_title: "Connection request",
    approval_body: "Someone is trying to connect to this computer. Allow it?",
    approve: "Allow",
    deny: "Deny",
};

const KO: Strings = Strings {
    subtitle: "원격 지원 세션",
    password_label: "비밀번호",
    share: "연결하는 사람에게 알려주세요.",
    waiting: "연결 대기 중",
    allow: "원격 조작 허용",
    on_sub: "상대가 내 PC를 조작할 수 있습니다",
    off_sub: "보기 전용 — 조작할 수 없습니다",
    disconnect: "모두 연결 끊기",
    tray_open: "열기",
    tray_quit: "종료",
    approval_title: "연결 요청",
    approval_body: "누군가 이 컴퓨터에 접속하려고 합니다. 허용할까요?",
    approve: "허용",
    deny: "거부",
};

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
            // Hand the context to the network thread so it can pop the window
            // (from the tray) when a connection request needs approval.
            let _ = shared.ctx.set(cc.egui_ctx.clone());
            // Korean UI only if the OS locale is Korean AND the bundled-with-Windows
            // Korean font loads (egui's default fonts have no CJK glyphs).
            let korean = locale_is_korean() && load_korean_font(&cc.egui_ctx);
            let s: &'static Strings = if korean { &KO } else { &EN };

            // System tray so closing the window hides (keeps the session alive)
            // instead of quitting. Tray thread wakes the egui loop via ctx.
            let tray_show = Arc::new(AtomicBool::new(false));
            let tray_quit = Arc::new(AtomicBool::new(false));
            let tray = build_tray(&host_id, s, cc.egui_ctx.clone(), tray_show.clone(), tray_quit.clone());

            Ok(Box::new(HostUi {
                host_id,
                password,
                shared,
                korean,
                s,
                tray,
                tray_show,
                tray_quit,
            }))
        }),
    )
}

fn locale_is_korean() -> bool {
    sys_locale::get_locale()
        .map(|l| l.to_lowercase().starts_with("ko"))
        .unwrap_or(false)
}

/// Load a Windows Korean font at runtime (no bundling → exe stays small).
fn load_korean_font(ctx: &egui::Context) -> bool {
    let candidates = [
        r"C:\Windows\Fonts\malgun.ttf",  // Malgun Gothic
        r"C:\Windows\Fonts\malgunsl.ttf",
        r"C:\Windows\Fonts\gulim.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("kr".to_owned(), egui::FontData::from_owned(bytes));
            // Korean text is proportional; keep it first there. Also append to
            // monospace as a fallback so ID/password digits still render.
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "kr".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("kr".to_owned());
            ctx.set_fonts(fonts);
            return true;
        }
    }
    false
}

/// Build the system tray icon with Open/Quit, and a thread that turns menu
/// clicks into flags + wakes the egui loop (so it works even while hidden).
fn build_tray(
    host_id: &str,
    s: &'static Strings,
    ctx: egui::Context,
    show: Arc<AtomicBool>,
    quit: Arc<AtomicBool>,
) -> Option<TrayIcon> {
    let rgba: Vec<u8> = (0..16 * 16).flat_map(|_| [0u8, 140, 200, 255]).collect();
    let icon = tray_icon::Icon::from_rgba(rgba, 16, 16).ok()?;

    let open = MenuItem::new(s.tray_open, true, None);
    let quit_item = MenuItem::new(s.tray_quit, true, None);
    let menu = Menu::new();
    menu.append(&open).ok()?;
    menu.append(&quit_item).ok()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(format!("Remote Work - ID: {host_id}"))
        .with_icon(icon)
        .build()
        .ok()?;

    let open_id = open.id().clone();
    let quit_id = quit_item.id().clone();
    std::thread::spawn(move || {
        let rx = MenuEvent::receiver();
        while let Ok(event) = rx.recv() {
            if event.id == quit_id {
                quit.store(true, Ordering::Relaxed);
            } else if event.id == open_id {
                show.store(true, Ordering::Relaxed);
            }
            ctx.request_repaint();
        }
    });
    Some(tray)
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
    korean: bool,
    s: &'static Strings,
    /// Kept alive so the tray icon stays visible; None if tray creation failed.
    tray: Option<TrayIcon>,
    tray_show: Arc<AtomicBool>,
    tray_quit: Arc<AtomicBool>,
}

impl eframe::App for HostUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));

        // Tray "Quit" → really close (let the window close request through).
        if self.tray_quit.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        // Tray "Open" → re-show a hidden window.
        if self.tray_show.swap(false, Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        // Window close (X): if we have a tray, hide instead of quitting so the
        // remote session keeps running. Without a tray, let it close normally.
        if self.tray.is_some() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // A connection is waiting for approval — show the prompt instead of the
        // normal UI until the host decides.
        if self.shared.pending_approval.load(Ordering::Relaxed) {
            self.approval_prompt(ctx);
            return;
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(18.0)).fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🖥").size(22.0));
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Remote Work").size(19.0).strong().color(VALUE));
                        ui.label(egui::RichText::new(self.s.subtitle).size(11.0).color(LABEL));
                    });
                });

                ui.add_space(14.0);

                // Credentials card
                card(ui, |ui| {
                    cred_row(ui, "ID", &group_id(&self.host_id), &self.host_id);
                    ui.add_space(10.0);
                    cred_row(ui, self.s.password_label, &self.password, &self.password);
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new(self.s.share).size(11.0).color(LABEL));

                ui.add_space(14.0);

                // Status
                let count = self.shared.viewer_count.load(Ordering::Relaxed);
                ui.horizontal(|ui| {
                    let (dot, text) = if count == 0 {
                        (LABEL, self.s.waiting.to_owned())
                    } else if self.korean {
                        (GREEN, format!("{count}명 접속 중"))
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
                            ui.label(egui::RichText::new(self.s.allow).size(14.0).color(VALUE));
                            let sub = if allow { self.s.on_sub } else { self.s.off_sub };
                            ui.label(egui::RichText::new(sub).size(11.0).color(LABEL));
                        });
                    });
                });

                // Bottom: disconnect
                ui.add_space(16.0);
                let btn = egui::Button::new(
                    egui::RichText::new(self.s.disconnect).color(egui::Color32::WHITE).size(13.0),
                )
                .fill(if count == 0 { egui::Color32::from_rgb(203, 210, 217) } else { DANGER })
                .rounding(7.0)
                .min_size(egui::vec2(ui.available_width(), 34.0));
                if ui.add_enabled(count > 0, btn).clicked() {
                    self.shared.disconnect_all.store(true, Ordering::Relaxed);
                }
            });
    }
}

impl HostUi {
    /// Full-window Allow/Deny prompt shown while a connection awaits approval.
    fn approval_prompt(&self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(18.0)).fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("🔔").size(40.0));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(self.s.approval_title).size(18.0).strong().color(VALUE));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(self.s.approval_body).size(13.0).color(LABEL));
                });
                ui.add_space(26.0);
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    let deny = egui::Button::new(
                        egui::RichText::new(self.s.deny).color(egui::Color32::WHITE).size(14.0),
                    )
                    .fill(DANGER)
                    .rounding(7.0)
                    .min_size(egui::vec2(w, 40.0));
                    if ui.add(deny).clicked() {
                        self.shared.approval_decision.store(2, Ordering::Relaxed);
                    }
                    let allow = egui::Button::new(
                        egui::RichText::new(self.s.approve).color(egui::Color32::WHITE).size(14.0),
                    )
                    .fill(GREEN)
                    .rounding(7.0)
                    .min_size(egui::vec2(w, 40.0));
                    if ui.add(allow).clicked() {
                        self.shared.approval_decision.store(1, Ordering::Relaxed);
                    }
                });
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
