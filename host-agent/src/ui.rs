use crate::app::{ChatLine, PendingFile, Shared};
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
    quit: &'static str,
    approval_title: &'static str,
    approval_body: &'static str,
    approve: &'static str,
    deny: &'static str,
    chat_title: &'static str,
    chat_placeholder: &'static str,
    chat_send: &'static str,
    chat_empty: &'static str,
    chat_open_btn: &'static str,
    file_title: &'static str,
    file_body: &'static str,
    file_accept: &'static str,
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
    quit: "Quit",
    approval_title: "Connection request",
    approval_body: "Someone is trying to connect to this computer. Allow it?",
    approve: "Allow",
    deny: "Deny",
    chat_title: "Chat",
    chat_placeholder: "Type a message…",
    chat_send: "Send",
    chat_empty: "No messages yet.",
    chat_open_btn: "💬 Open chat",
    file_title: "Incoming file",
    file_body: "The viewer wants to send this file. Receive it?",
    file_accept: "Receive",
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
    quit: "종료",
    approval_title: "연결 요청",
    approval_body: "누군가 이 컴퓨터에 접속하려고 합니다. 허용할까요?",
    approve: "허용",
    deny: "거부",
    chat_title: "채팅",
    chat_placeholder: "메시지를 입력하세요…",
    chat_send: "전송",
    chat_empty: "아직 메시지가 없습니다.",
    chat_open_btn: "💬 채팅 열기",
    file_title: "파일 받기",
    file_body: "상대가 이 파일을 보내려 합니다. 받으시겠습니까?",
    file_accept: "받기",
};

/// Small always-on host window: credentials, connection status, live view-only
/// toggle. Runs on the main thread (eframe owns the event loop); the tokio
/// network stack runs on a background thread and talks through `Shared`.
pub fn run(host_id: String, password: String, shared: Shared) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([380.0, 590.0])
            .with_min_inner_size([380.0, 590.0])
            .with_resizable(false)
            // Fixed-size window: maximize is pointless, and closing (X) already
            // minimizes to the taskbar, so a separate minimize button is
            // redundant. Leave just the close button.
            .with_maximize_button(false)
            .with_minimize_button(false),
        ..Default::default()
    };
    eframe::run_native(
        "Remote Work",
        options,
        Box::new(move |cc| {
            style(&cc.egui_ctx);
            // Hand the context to the network thread so it can pop the window to
            // the front when a connection request needs approval.
            let _ = shared.ctx.set(cc.egui_ctx.clone());
            // Korean UI only if the OS locale is Korean AND the bundled-with-Windows
            // Korean font loads (egui's default fonts have no CJK glyphs).
            let korean = locale_is_korean() && load_korean_font(&cc.egui_ctx);
            let s: &'static Strings = if korean { &KO } else { &EN };

            Ok(Box::new(HostUi {
                host_id,
                password,
                shared,
                korean,
                s,
                chat_seen_len: 0,
                quitting: false,
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
    /// Chat transcript length last seen, to reopen the window on new messages.
    chat_seen_len: usize,
    /// Set by the Quit button so the close request is allowed through.
    quitting: bool,
}

impl eframe::App for HostUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));

        // Window close (X): minimize to the taskbar instead of quitting, so the
        // remote session keeps running. Quit explicitly via the Quit button.
        if !self.quitting && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        // A connection is waiting for approval — show the prompt instead of the
        // normal UI until the host decides.
        if self.shared.pending_approval.load(Ordering::Relaxed) {
            self.approval_prompt(ctx);
            return;
        }

        // A file transfer is waiting for accept/deny.
        let pending_file = self.shared.pending_file.lock().unwrap().clone();
        if let Some(file) = pending_file {
            self.file_prompt(ctx, &file);
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

                // Open the chat window (host can start a conversation, or reopen
                // it after closing).
                if count > 0 {
                    ui.add_space(12.0);
                    let chat_btn = egui::Button::new(
                        egui::RichText::new(self.s.chat_open_btn).color(ACCENT).size(13.0),
                    )
                    .fill(egui::Color32::from_rgb(235, 242, 255))
                    .stroke(egui::Stroke::new(1.0, ACCENT))
                    .rounding(7.0)
                    .min_size(egui::vec2(ui.available_width(), 32.0));
                    if ui.add(chat_btn).clicked() {
                        self.shared.chat_open.store(true, Ordering::Relaxed);
                    }
                }

                // Bottom: disconnect
                ui.add_space(12.0);
                let btn = egui::Button::new(
                    egui::RichText::new(self.s.disconnect).color(egui::Color32::WHITE).size(13.0),
                )
                .fill(if count == 0 { egui::Color32::from_rgb(203, 210, 217) } else { DANGER })
                .rounding(7.0)
                .min_size(egui::vec2(ui.available_width(), 34.0));
                if ui.add_enabled(count > 0, btn).clicked() {
                    self.shared.disconnect_all.store(true, Ordering::Relaxed);
                }

                // Quit. Closing the window (X) only minimizes to the taskbar so
                // the session survives; this actually exits.
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(self.s.quit).size(12.0).color(LABEL),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.quitting = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });

        // Chat lives in its own always-on-top window near the screen bottom, so
        // it stays reachable even when the main window is hidden or covered. It
        // exists only while a viewer is connected; closing it hides it until a
        // new message arrives (notification-style).
        if self.shared.chat_send.lock().unwrap().is_some() {
            let len = self.shared.chat_log.lock().unwrap().len();
            if len > self.chat_seen_len {
                self.shared.chat_open.store(true, Ordering::Relaxed);
            }
            self.chat_seen_len = len;
            if self.shared.chat_open.load(Ordering::Relaxed) {
                chat_window(ctx, &self.shared, self.s);
            }
        } else {
            self.chat_seen_len = 0;
        }
    }
}

/// Separate always-on-top chat window (an egui viewport), docked bottom-right.
fn chat_window(ctx: &egui::Context, shared: &Shared, s: &'static Strings) {
    let monitor = ctx.input(|i| i.viewport().monitor_size).unwrap_or(egui::vec2(1920.0, 1080.0));
    let size = egui::vec2(360.0, 420.0);
    let pos = egui::pos2(monitor.x - size.x - 24.0, monitor.y - size.y - 64.0);
    let builder = egui::ViewportBuilder::default()
        .with_title(s.chat_title)
        .with_inner_size(size)
        .with_min_inner_size([300.0, 280.0])
        .with_position(pos)
        .with_maximize_button(false)
        .with_always_on_top();

    let shared = shared.clone();
    ctx.show_viewport_deferred(
        egui::ViewportId::from_hash_of("host_chat"),
        builder,
        move |ctx, _class| {
            ctx.request_repaint_after(Duration::from_millis(400));

            // Close (X): stop showing the window. The main loop then stops
            // recreating this viewport, so it actually closes.
            if ctx.input(|i| i.viewport().close_requested()) {
                shared.chat_open.store(false, Ordering::Relaxed);
            }

            // Input row: a viewport-level bottom panel with a fixed height. The
            // text box and Send button are given the same explicit height and
            // vertically centered, so nothing is clipped or misaligned.
            egui::TopBottomPanel::bottom("chat_input_row")
                .exact_height(58.0)
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .inner_margin(egui::Margin::symmetric(12.0, 0.0)),
                )
                .show(ctx, |ui| {
                    let row_h = 32.0;
                    let send_w = 58.0;
                    ui.horizontal_centered(|ui| {
                        let mut input = shared.chat_input.lock().unwrap();
                        let field_w = (ui.available_width() - send_w - 8.0).max(60.0);
                        let resp = ui.add_sized(
                            [field_w, row_h],
                            egui::TextEdit::singleline(&mut *input)
                                .vertical_align(egui::Align::Center)
                                .hint_text(s.chat_placeholder),
                        );
                        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let send = ui
                            .add_sized([send_w, row_h], egui::Button::new(s.chat_send))
                            .clicked();
                        if (enter || send) && !input.trim().is_empty() {
                            let text = input.trim().to_string();
                            if let Some(tx) = shared.chat_send.lock().unwrap().as_ref() {
                                let _ = tx.send(text);
                            }
                            input.clear();
                            resp.request_focus();
                        }
                    });
                });

            // Transcript fills the space above the input row.
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .inner_margin(egui::Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let log = shared.chat_log.lock().unwrap();
                            if log.is_empty() {
                                ui.label(egui::RichText::new(s.chat_empty).size(12.0).color(LABEL));
                            } else {
                                for line in log.iter() {
                                    chat_bubble(ui, line);
                                    ui.add_space(6.0);
                                }
                            }
                        });
                });
        },
    );
}

/// Human-readable byte size (e.g. "2.4 MB").
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

impl HostUi {
    /// Full-window Receive/Deny prompt shown while a file awaits the host's OK.
    fn file_prompt(&self, ctx: &egui::Context, file: &PendingFile) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(18.0)).fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("📄").size(38.0));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(self.s.file_title).size(18.0).strong().color(VALUE));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(&file.name).size(15.0).strong().color(VALUE));
                    ui.label(egui::RichText::new(format_size(file.size)).size(12.0).color(LABEL));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(self.s.file_body).size(12.0).color(LABEL));
                });
                ui.add_space(22.0);
                ui.horizontal(|ui| {
                    let w = (ui.available_width() - 8.0) / 2.0;
                    let deny = egui::Button::new(
                        egui::RichText::new(self.s.deny).color(egui::Color32::WHITE).size(14.0),
                    )
                    .fill(DANGER)
                    .rounding(7.0)
                    .min_size(egui::vec2(w, 40.0));
                    if ui.add(deny).clicked() {
                        self.shared.file_decision.store(2, Ordering::Relaxed);
                    }
                    let accept = egui::Button::new(
                        egui::RichText::new(self.s.file_accept).color(egui::Color32::WHITE).size(14.0),
                    )
                    .fill(GREEN)
                    .rounding(7.0)
                    .min_size(egui::vec2(w, 40.0));
                    if ui.add(accept).clicked() {
                        self.shared.file_decision.store(1, Ordering::Relaxed);
                    }
                });
            });
    }

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

/// Render one chat message as a bubble: peer on the left, host on the right.
fn chat_bubble(ui: &mut egui::Ui, line: &ChatLine) {
    let max_w = (ui.available_width() * 0.72).max(80.0);
    let (fill, text_col) = if line.from_me {
        (ACCENT, egui::Color32::WHITE)
    } else {
        (egui::Color32::from_rgb(233, 236, 240), VALUE)
    };
    let layout = if line.from_me {
        egui::Layout::right_to_left(egui::Align::Min)
    } else {
        egui::Layout::left_to_right(egui::Align::Min)
    };
    ui.with_layout(layout, |ui| {
        egui::Frame::none()
            .fill(fill)
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.set_max_width(max_w);
                ui.label(egui::RichText::new(&line.text).size(13.0).color(text_col));
            });
    });
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
