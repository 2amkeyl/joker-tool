//! GUI chính của Joker Tool.

use crate::avatar::{fetch_avatar, AVATAR_SIZE};
use crate::badge::{
    find_games, find_time, is_likely_fingerprint, run_badge_session, GAME_MILESTONES,
    TIME_MILESTONES,
};
use crate::config::AppConfig;
use crate::events::ProgressEvent;
use crate::hypesquad::set_hypesquad_house;
use crate::log_writer::LogWriter;
use crate::quest::{run_quest_session, validate_token, UserInfo};
use eframe::egui::{self, Align, Align2, Color32, FontFamily, FontId, Layout, Margin, RichText, Rounding, Sense, Stroke, TextStyle, Vec2};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Quest,
    Badge,
    HypeSquad,
}

pub struct JokerApp {
    _rt: tokio::runtime::Runtime,
    rt: tokio::runtime::Handle,
    cfg: AppConfig,
    log: LogWriter,
    tx: UnboundedSender<ProgressEvent>,
    rx: UnboundedReceiver<ProgressEvent>,

    token_input: String,
    user_info: Option<UserInfo>,
    checking_token: bool,
    current_tab: Tab,

    quest_running: bool,
    quest_cancel: Option<CancellationToken>,
    current_quest: Option<(String, String, f64, f64)>,
    quest_completed: Vec<String>,
    quest_failed: Vec<(String, String)>,
    quest_all_done: Option<(usize, usize, String)>,

    badge_time_key: String,
    badge_game_key: String,
    badge_fingerprint: String,
    badge_running: bool,
    badge_cancel: Option<CancellationToken>,
    badge_progress: (usize, usize, usize),
    badge_desc: String,
    badge_result: Option<(bool, String)>,

    hs_house: String,
    hs_running: bool,
    hs_result: Option<(bool, String)>,

    log_lines: Vec<String>,
    auto_scroll: bool,
    show_token: bool,

    avatar_pending: Option<Vec<u8>>,
    avatar_tex: Option<egui::TextureHandle>,
    logo_tex: Option<egui::TextureHandle>,
}

impl JokerApp {
    pub fn new(
        rt: tokio::runtime::Runtime,
        cfg: AppConfig,
        log: LogWriter,
        tx: UnboundedSender<ProgressEvent>,
        rx: UnboundedReceiver<ProgressEvent>,
    ) -> Self {
        let rt_handle = rt.handle().clone();
        let token_input = cfg.token.clone();
        let badge_time_key = if cfg.badge_time_key.is_empty() { "5".into() } else { cfg.badge_time_key.clone() };
        let badge_game_key = if cfg.badge_game_key.is_empty() { "3".into() } else { cfg.badge_game_key.clone() };

        Self {
            _rt: rt, rt: rt_handle, cfg, log, tx, rx,
            token_input, user_info: None, checking_token: false, current_tab: Tab::Quest,
            quest_running: false, quest_cancel: None, current_quest: None, quest_completed: Vec::new(), quest_failed: Vec::new(), quest_all_done: None,
            badge_time_key, badge_game_key, badge_fingerprint: String::new(), badge_running: false, badge_cancel: None, badge_progress: (0, 0, 0), badge_desc: String::new(), badge_result: None,
            hs_house: "bravery".into(), hs_running: false, hs_result: None,
            log_lines: Vec::new(), auto_scroll: true, show_token: false, avatar_pending: None, avatar_tex: None, logo_tex: None,
        }
    }

    fn handle_event(&mut self, ev: ProgressEvent) {
        match ev {
            ProgressEvent::Log(msg) => {
                self.log.write(&msg);
                self.log_lines.push(msg);
            }
            ProgressEvent::TokenStatus { valid, msg, user } => {
                self.checking_token = false;
                if valid {
                    self.user_info = user;
                    if let Some(u) = self.user_info.clone() {
                        let tx = self.tx.clone();
                        self.rt.spawn(async move {
                            if let Some(rgba) = fetch_avatar(&u).await {
                                let _ = tx.send(ProgressEvent::Avatar { user_id: u.id.clone(), rgba });
                            }
                        });
                    }
                    self.log.write(&format!("[OK] {}", msg));
                    self.log_lines.push(format!("[OK] {}", msg));
                } else {
                    self.user_info = None;
                    self.log.write(&format!("[X] {}", msg));
                    self.log_lines.push(format!("[X] {}", msg));
                }
            }
            ProgressEvent::Avatar { user_id, rgba } => {
                if self.user_info.as_ref().map(|u| u.id == user_id).unwrap_or(false) {
                    self.avatar_pending = Some(rgba);
                }
            }
            ProgressEvent::QuestUpdate { name, task_type, sec_done, sec_needed } => {
                self.current_quest = Some((name, task_type, sec_done, sec_needed));
            }
            ProgressEvent::QuestCompleted { name, success, reason } => {
                if success {
                    self.log_lines.push(format!("[OK] {}", name));
                    self.quest_completed.push(name);
                } else {
                    let r = reason.unwrap_or_default();
                    self.log_lines.push(format!("[X] {}: {}", name, r));
                    self.quest_failed.push((name, r));
                }
                self.current_quest = None;
            }
            ProgressEvent::QuestAllDone { completed, failed, reason } => {
                self.quest_running = false;
                self.quest_cancel = None;
                self.current_quest = None;
                let msg = format!(">> {} (OK: {} / Lỗi: {})", reason, completed, failed);
                self.log.write(&msg);
                self.log_lines.push(msg);
                self.quest_all_done = Some((completed, failed, reason));
            }
            ProgressEvent::BadgeUpdate { done, failed, total, desc } => {
                self.badge_progress = (done, failed, total);
                self.badge_desc = desc;
            }
            ProgressEvent::BadgeDone { success, done, total, reason } => {
                self.badge_running = false;
                self.badge_cancel = None;
                let msg = if success { format!("[OK] Hoàn tất {}/{} game", done, total) } else { format!("[X] Lỗi: {}", reason) };
                self.log.write(&msg);
                self.log_lines.push(msg.clone());
                self.badge_result = Some((success, msg));
            }
            ProgressEvent::HypeSquadResult { success, msg } => {
                self.hs_running = false;
                let out = if success { format!("[OK] {}", msg) } else { format!("[X] {}", msg) };
                self.log.write(&out);
                self.log_lines.push(out.clone());
                self.hs_result = Some((success, out));
            }
        }
    }

    fn check_token(&mut self) {
        let token = self.token_input.trim().to_string();
        if token.is_empty() { return; }
        self.checking_token = true;
        self.user_info = None;
        self.avatar_tex = None;
        self.avatar_pending = None;
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            match validate_token(&token).await {
                Ok(info) => { let _ = tx.send(ProgressEvent::TokenStatus { valid: true, msg: format!("Đăng nhập thành công: {}", info.display()), user: Some(info) }); }
                Err(e) => { let _ = tx.send(ProgressEvent::TokenStatus { valid: false, msg: e, user: None }); }
            }
        });
    }

    fn start_quest(&mut self) {
        let token = self.token_input.trim().to_string();
        if token.is_empty() { return; }
        let cancel = CancellationToken::new();
        self.quest_cancel = Some(cancel.clone());
        self.quest_running = true;
        self.current_quest = None;
        self.quest_completed.clear();
        self.quest_failed.clear();
        self.quest_all_done = None;
        let tx = self.tx.clone();
        self.rt.spawn(async move { run_quest_session(token, tx, cancel).await; });
    }

    fn start_badge(&mut self) {
        let token = self.token_input.trim().to_string();
        let fp = self.badge_fingerprint.trim().to_string();
        let time = match find_time(&self.badge_time_key) { Some(t) => t, None => return };
        let game = match find_games(&self.badge_game_key) { Some(g) => g, None => return };
        let hours_per_game = time.hours as f64 / game.games as f64;
        let games_count = game.games as usize;
        
        self.cfg.badge_time_key = self.badge_time_key.clone();
        self.cfg.badge_game_key = self.badge_game_key.clone();
        self.cfg.token = token.clone();
        
        if let Err(e) = self.cfg.save() {
            self.log_lines.push(format!("[X] Không lưu được config: {}", e));
        }

        let cancel = CancellationToken::new();
        self.badge_cancel = Some(cancel.clone());
        self.badge_running = true;
        self.badge_progress = (0, 0, games_count);
        self.badge_desc = "Đang khởi động...".into();
        self.badge_result = None;
        let tx = self.tx.clone();
        self.rt.spawn(async move { run_badge_session(token, fp, hours_per_game, games_count, tx, cancel).await; });
    }

    fn start_hypesquad(&mut self) {
        let token = self.token_input.trim().to_string();
        let house = self.hs_house.clone();
        self.hs_running = true;
        self.hs_result = None;
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let (success, msg) = set_hypesquad_house(&token, &house).await;
            let _ = tx.send(ProgressEvent::HypeSquadResult { success, msg });
        });
    }
}

// ===== Bảng màu: đen – đỏ (Persona 5) =====
const BG_LAYER_1: Color32 = Color32::from_rgb(14, 14, 17);     // Log, input
const BG_LAYER_2: Color32 = Color32::from_rgb(21, 21, 25);     // Sidebar, thanh trên
const BG_LAYER_3: Color32 = Color32::from_rgb(27, 27, 32);     // Nền chính
const CARD: Color32 = Color32::from_rgb(35, 35, 41);
const CARD_INNER: Color32 = Color32::from_rgb(24, 24, 29);
const BORDER: Color32 = Color32::from_rgb(56, 56, 66);
const TEXT_NORMAL: Color32 = Color32::from_rgb(238, 238, 244);
const TEXT_MUTED: Color32 = Color32::from_rgb(170, 172, 188);
const GREEN: Color32 = Color32::from_rgb(52, 199, 120);
const RED: Color32 = Color32::from_rgb(240, 70, 80);
const STOP: Color32 = Color32::from_rgb(150, 38, 48);
const ACCENT_RED: Color32 = Color32::from_rgb(232, 32, 54);
const ACCENT_AMBER: Color32 = Color32::from_rgb(255, 160, 40);
const ACCENT_PURPLE: Color32 = Color32::from_rgb(157, 104, 248);
const BLURPLE: Color32 = Color32::from_rgb(88, 101, 242);
const DISABLED_BG: Color32 = Color32::from_rgb(46, 46, 54);

const CONTENT_MAX_WIDTH: f32 = 720.0;
const MAX_LOG_LINES: usize = 2000;

fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

fn load_logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(include_bytes!("../icon.png")).ok()?
        .resize_exact(128, 128, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    Some(ctx.load_texture("joker_logo", egui::ColorImage::from_rgba_unmultiplied([128, 128], img.as_raw()), egui::TextureOptions::LINEAR))
}

fn accent_of(tab: Tab) -> Color32 {
    match tab {
        Tab::Quest => ACCENT_RED,
        Tab::Badge => ACCENT_AMBER,
        Tab::HypeSquad => ACCENT_PURPLE,
    }
}

/// Cài đặt theme một lần duy nhất (gọi trong main khi tạo app).
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = egui::Visuals::dark();
    v.window_fill = BG_LAYER_3;
    v.panel_fill = BG_LAYER_3;
    v.extreme_bg_color = BG_LAYER_1;
    v.selection.bg_fill = Color32::from_rgb(150, 30, 48);
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(10.0);

    let r = Rounding::same(8.0);
    v.widgets.noninteractive.bg_fill = BG_LAYER_2;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.rounding = r;

    for (w, fill, fg) in [
        (&mut v.widgets.inactive, Color32::from_rgb(48, 48, 57), TEXT_NORMAL),
        (&mut v.widgets.hovered, Color32::from_rgb(68, 68, 80), Color32::WHITE),
        (&mut v.widgets.active, ACCENT_RED, Color32::WHITE),
        (&mut v.widgets.open, Color32::from_rgb(68, 68, 80), Color32::WHITE),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, BORDER);
        w.fg_stroke = Stroke::new(1.0, fg);
        w.rounding = r;
        w.expansion = 0.0;
    }
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(110, 60, 70));

    style.visuals = v;
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(16.0, 8.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.combo_height = 260.0;
    style.spacing.scroll.bar_width = 8.0;
    style.text_styles.insert(TextStyle::Body, FontId::new(14.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(14.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));
    ctx.set_style(style);
}

// ---------- Widget helper ----------

fn card<R>(ui: &mut egui::Ui, accent: Color32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let out = egui::Frame::default()
        .fill(CARD)
        .rounding(12.0)
        .inner_margin(Margin::same(20.0))
        .stroke(Stroke::new(1.0, BORDER))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    let r = out.response.rect;
    ui.painter().rect_filled(egui::Rect::from_min_size(r.min + Vec2::new(18.0, 0.0), Vec2::new(54.0, 3.0)), Rounding::same(2.0), accent);
    out.inner
}

fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str, accent: Color32) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(30.0).color(Color32::WHITE).family(bold()));
    let (bar, _) = ui.allocate_exact_size(Vec2::new(64.0, 4.0), Sense::hover());
    ui.painter().rect_filled(bar, Rounding::same(2.0), accent);
    ui.add_space(4.0);
    ui.label(RichText::new(subtitle).color(TEXT_MUTED).size(14.0));
    ui.add_space(18.0);
}

fn action_button(ui: &mut egui::Ui, label: &str, fill: Color32, enabled: bool) -> bool {
    let (bg, fg) = if enabled { (fill, Color32::WHITE) } else { (DISABLED_BG, TEXT_MUTED) };
    let w = ui.available_width();
    ui.add_enabled_ui(enabled, |ui| {
        ui.add_sized(
            [w, 46.0],
            egui::Button::new(RichText::new(label).size(15.5).family(bold()).color(fg)).fill(bg).rounding(10.0),
        )
    })
    .inner
    .clicked()
}

fn login_hint(ui: &mut egui::Ui, logged_in: bool) {
    if !logged_in {
        ui.add_space(2.0);
        ui.label(RichText::new("Hãy nhập token và bấm \"Kiểm tra\" trước khi bắt đầu.").size(12.0).color(TEXT_MUTED));
    }
}

fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

fn account_card(ui: &mut egui::Ui, name: Option<String>, avatar: Option<egui::TextureId>, accent: Color32) {
    let w = (ui.available_width() - 20.0).max(60.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 58.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(12.0), CARD_INNER);
    p.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, BORDER));
    let c = egui::pos2(rect.left() + 28.0, rect.center().y);
    let logged = name.is_some();
    if let (true, Some(id)) = (logged, avatar) {
        // Avatar Discord thật
        p.image(id, egui::Rect::from_center_size(c, Vec2::splat(34.0)), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        p.circle_stroke(c, 18.0, Stroke::new(1.5, accent));
    } else {
        p.circle_filled(c, 17.0, if logged { accent.gamma_multiply(0.30) } else { DISABLED_BG });
        p.circle_stroke(c, 17.0, Stroke::new(1.5, if logged { accent } else { BORDER }));
        let initial = name.as_ref().and_then(|n| n.chars().next()).map(|ch| ch.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
        p.text(c, Align2::CENTER_CENTER, initial, FontId::new(15.0, bold()), Color32::WHITE);
    }
    let shown = match name {
        Some(n) if n.chars().count() > 14 => n.chars().take(13).collect::<String>() + "…",
        Some(n) => n,
        None => "Khách".to_string(),
    };
    p.text(egui::pos2(rect.left() + 54.0, rect.center().y - 8.0), Align2::LEFT_CENTER, shown, FontId::new(13.5, bold()), Color32::WHITE);
    let (st, col) = if logged { ("Đã đăng nhập", GREEN) } else { ("Chưa đăng nhập", TEXT_MUTED) };
    p.text(egui::pos2(rect.left() + 54.0, rect.center().y + 10.0), Align2::LEFT_CENTER, st, FontId::new(11.5, FontFamily::Proportional), col);
}

fn stat_chip(ui: &mut egui::Ui, w: f32, label: &str, value: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 60.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(10.0), CARD_INNER);
    p.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, BORDER));
    p.rect_filled(egui::Rect::from_min_size(rect.min + Vec2::new(0.0, 12.0), Vec2::new(3.0, 36.0)), Rounding::same(2.0), color);
    p.text(egui::pos2(rect.left() + 16.0, rect.top() + 19.0), Align2::LEFT_CENTER, label, FontId::new(11.0, FontFamily::Proportional), TEXT_MUTED);
    p.text(egui::pos2(rect.left() + 16.0, rect.top() + 41.0), Align2::LEFT_CENTER, value, FontId::new(18.0, bold()), color);
}

fn progress_bar(ui: &mut egui::Ui, frac: f32, text: &str, accent: Color32, animate: bool) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let pulse = if animate { 0.88 + 0.12 * (t * 4.0).sin() } else { 1.0 };
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(12.0), BG_LAYER_1);
    p.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, BORDER));
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let fw = (rect.width() * frac).max(24.0);
        let fr = egui::Rect::from_min_size(rect.min, Vec2::new(fw, rect.height()));
        p.rect_filled(fr, Rounding::same(12.0), accent.gamma_multiply(pulse));
    }
    p.text(rect.center(), Align2::CENTER_CENTER, text, FontId::new(12.0, bold()), Color32::WHITE);
}

fn draw_tab_icon(p: &egui::Painter, tab: Tab, c: egui::Pos2, color: Color32) {
    let at = |dx: f32, dy: f32| egui::pos2(c.x + dx, c.y + dy);
    match tab {
        Tab::Quest => {
            p.add(egui::Shape::convex_polygon(vec![at(0.0, -9.0), at(8.0, 0.0), at(0.0, 9.0), at(-8.0, 0.0)], color, Stroke::NONE));
        }
        Tab::Badge => {
            p.circle_stroke(c, 8.0, Stroke::new(2.0, color));
            p.circle_filled(c, 3.0, color);
        }
        Tab::HypeSquad => {
            p.add(egui::Shape::convex_polygon(vec![at(0.0, -9.0), at(9.0, 7.0), at(-9.0, 7.0)], color, Stroke::NONE));
        }
    }
}

fn nav_item(ui: &mut egui::Ui, text: &str, tab: Tab, selected: bool) -> egui::Response {
    let size = Vec2::new(ui.available_width(), 44.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let body = rect.shrink2(Vec2::new(10.0, 0.0));
    let accent = accent_of(tab);

    if selected {
        ui.painter().rect_filled(body, Rounding::same(10.0), accent.gamma_multiply(0.16));
        let bar = egui::Rect::from_center_size(egui::pos2(body.left() + 3.0, body.center().y), Vec2::new(4.0, 24.0));
        ui.painter().rect_filled(bar, Rounding::same(2.0), accent);
    } else if resp.hovered() {
        ui.painter().rect_filled(body, Rounding::same(10.0), Color32::from_rgb(34, 34, 40));
    }

    let color = if selected { Color32::WHITE } else if resp.hovered() { TEXT_NORMAL } else { TEXT_MUTED };
    let icon_color = if selected || resp.hovered() { accent } else { TEXT_MUTED };
    draw_tab_icon(ui.painter(), tab, egui::pos2(body.left() + 26.0, body.center().y), icon_color);
    ui.painter().text(
        egui::pos2(body.left() + 48.0, body.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::new(15.0, if selected { bold() } else { FontFamily::Proportional }),
        color,
    );
    resp
}

fn logo_block(ui: &mut egui::Ui, compact: bool, tex: Option<egui::TextureId>) {
    let h = if compact { 66.0 } else { 76.0 };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    let icon = if compact { 46.0 } else { 54.0 };
    let icon_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 20.0, rect.center().y - icon / 2.0), Vec2::splat(icon));
    let round = Rounding::same(icon * 0.24);

    // Quầng đỏ mờ phía sau icon
    p.rect_filled(icon_rect.expand(4.0), Rounding::same(icon * 0.30), ACCENT_RED.gamma_multiply(0.28));
    match tex {
        Some(id) => {
            p.image(id, icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => { p.rect_filled(icon_rect, round, ACCENT_RED); }
    }
    p.rect_stroke(icon_rect, round, Stroke::new(1.5, ACCENT_RED));

    let tx = icon_rect.right() + 14.0;
    p.text(egui::pos2(tx, rect.center().y - 11.0), Align2::LEFT_CENTER, "JOKER", FontId::new(if compact { 21.0 } else { 24.0 }, bold()), Color32::WHITE);
    p.text(egui::pos2(tx + 1.0, rect.center().y + 13.0), Align2::LEFT_CENTER, "T O O L", FontId::new(12.0, bold()), ACCENT_RED);
}

impl eframe::App for JokerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Nạp texture logo + avatar (cần egui Context nên làm ở đây)
        if self.logo_tex.is_none() { self.logo_tex = load_logo_texture(ctx); }
        if let Some(rgba) = self.avatar_pending.take() {
            let n = AVATAR_SIZE as usize;
            if rgba.len() == n * n * 4 {
                self.avatar_tex = Some(ctx.load_texture("discord_avatar", egui::ColorImage::from_rgba_unmultiplied([n, n], &rgba), egui::TextureOptions::LINEAR));
            }
        }

        // Xử lý sự kiện (luôn chạy kể cả khi đang thu nhỏ để không mất log)
        let mut got_event = false;
        while let Ok(ev) = self.rx.try_recv() { self.handle_event(ev); got_event = true; }
        if self.log_lines.len() > MAX_LOG_LINES {
            let extra = self.log_lines.len() - MAX_LOG_LINES;
            self.log_lines.drain(0..extra);
        }
        if got_event { ctx.request_repaint(); }
        else if self.quest_running || self.badge_running || self.checking_token || self.hs_running { ctx.request_repaint_after(std::time::Duration::from_millis(50)); }

        // Khi cửa sổ bị thu nhỏ.
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        let screen = ctx.screen_rect();
        if minimized || screen.width() < 300.0 || screen.height() < 240.0 {
            return;
        }

        let narrow = screen.width() < 860.0;
        let accent = accent_of(self.current_tab);

        // 1. SIDEBAR
        let mut next_tab = self.current_tab;
        let logo = self.logo_tex.as_ref().map(|t| t.id());
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(if narrow { 180.0 } else { 224.0 })
            .frame(egui::Frame::default().fill(BG_LAYER_2).stroke(Stroke::new(1.0, Color32::from_rgb(10, 10, 12))))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.add_space(18.0);
                logo_block(ui, narrow, logo);
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    ui.label(RichText::new("Phantom Thieves").color(TEXT_MUTED).size(12.0).italics());
                });
                ui.add_space(18.0);
                if nav_item(ui, "Cày Quest", Tab::Quest, self.current_tab == Tab::Quest).clicked() { next_tab = Tab::Quest; }
                if nav_item(ui, "Buff Badge", Tab::Badge, self.current_tab == Tab::Badge).clicked() { next_tab = Tab::Badge; }
                if nav_item(ui, "HypeSquad", Tab::HypeSquad, self.current_tab == Tab::HypeSquad).clicked() { next_tab = Tab::HypeSquad; }
                let who = self.user_info.as_ref().map(|i| i.display().to_string());
                let av = if self.user_info.is_some() { self.avatar_tex.as_ref().map(|t| t.id()) } else { None };
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).size(11.5).color(Color32::from_rgb(90, 92, 104)));
                    ui.add_space(4.0);
                    account_card(ui, who, av, accent);
                });
            });
        self.current_tab = next_tab;

        // 2. LOG (dưới) — chiều cao bị giới hạn theo cửa sổ
        let max_log_h = (screen.height() * 0.40).max(100.0);
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(150.0_f32.min(max_log_h))
            .height_range(80.0..=max_log_h)
            .frame(egui::Frame::default().inner_margin(Margin::symmetric(16.0, 10.0)).fill(BG_LAYER_1).stroke(Stroke::new(1.0, Color32::from_rgb(10, 10, 12))))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    status_dot(ui, accent);
                    ui.label(RichText::new("NHẬT KÝ HOẠT ĐỘNG").family(bold()).size(12.0).color(TEXT_MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.checkbox(&mut self.auto_scroll, RichText::new("Tự động cuộn").size(12.5).color(TEXT_MUTED));
                        if ui.small_button("Xoá").clicked() { self.log_lines.clear(); }
                        if ui.small_button("Sao chép").clicked() { ui.ctx().copy_text(self.log_lines.join("\n")); }
                        if ui.small_button("Mở thư mục").clicked() { self.log.open_folder(); }
                    });
                });
                ui.add_space(2.0);
                ui.separator();

                egui::ScrollArea::vertical().stick_to_bottom(self.auto_scroll).auto_shrink([false; 2]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    if self.log_lines.is_empty() {
                        ui.label(RichText::new("Chưa có hoạt động nào.").italics().size(12.5).color(TEXT_MUTED));
                    }
                    for line in &self.log_lines {
                        let color = if line.contains("[OK]") || line.contains("✓") { GREEN }
                        else if line.contains("[X]") || line.contains("⚠️") { RED }
                        else { TEXT_NORMAL };
                        ui.add(egui::Label::new(RichText::new(line).color(color).monospace().size(12.5)).wrap());
                    }
                });
            });

        // 3. TOKEN (trên)
        let mut do_check = false;
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::default().inner_margin(Margin::symmetric(24.0, 14.0)).fill(BG_LAYER_2).stroke(Stroke::new(1.0, Color32::from_rgb(10, 10, 12))))
            .show(ctx, |ui| {
                // Vạch nhấn màu theo tab ở mép trên cửa sổ
                let top = ui.max_rect().top() - 14.0;
                ui.ctx().layer_painter(ui.layer_id()).rect_filled(
                    egui::Rect::from_min_max(egui::pos2(screen.left(), top), egui::pos2(screen.right(), top + 3.0)), 0.0, accent);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("DISCORD TOKEN").size(11.5).color(TEXT_MUTED).family(bold()));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let can_save = self.user_info.is_some();
                        if ui.add_enabled(can_save, egui::Button::new(RichText::new("Lưu").color(Color32::WHITE))).clicked() {
                            self.cfg.token = self.token_input.trim().to_string();
                            if let Err(e) = self.cfg.save() { self.log_lines.push(format!("[X] Lỗi khi lưu token: {}", e)); }
                            else { self.log_lines.push("[OK] Đã lưu token an toàn".into()); }
                        }
                        if ui.add_enabled(!self.checking_token && !self.token_input.trim().is_empty(),
                            egui::Button::new(RichText::new("Kiểm tra").color(Color32::WHITE).family(bold())).fill(ACCENT_RED)).clicked() { do_check = true; }
                        ui.checkbox(&mut self.show_token, RichText::new("Hiện").size(12.5).color(TEXT_MUTED));
                        let res = ui.add_enabled(
                            !self.checking_token,
                            egui::TextEdit::singleline(&mut self.token_input)
                                .password(!self.show_token)
                                .desired_width((ui.available_width() - 28.0).max(120.0))
                                .margin(Margin::symmetric(10.0, 7.0))
                                .hint_text("Dán token của bạn vào đây..."),
                        );
                        if res.changed() { self.user_info = None; }
                        if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { do_check = true; }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if self.checking_token {
                        ui.spinner();
                        ui.label(RichText::new("Đang kiểm tra...").color(TEXT_MUTED).size(12.5));
                    } else if let Some(info) = &self.user_info {
                        status_dot(ui, GREEN);
                        ui.label(RichText::new(format!("{} (ID: {})", info.display(), info.id)).color(GREEN).family(bold()).size(12.5));
                    } else {
                        status_dot(ui, TEXT_MUTED);
                        ui.label(RichText::new("Chưa đăng nhập — nhập token và bấm Kiểm tra").color(TEXT_MUTED).size(12.5));
                    }
                });
            });
        if do_check { self.check_token(); }

        // 4. NỘI DUNG CHÍNH
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(Margin::symmetric(24.0, 16.0)).fill(BG_LAYER_3))
            .show(ctx, |ui| {
                // Họa tiết tam giác góc, đổi màu theo tab
                let r = ui.max_rect().expand2(Vec2::new(24.0, 16.0));
                let deco = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 22);
                let p = ui.painter();
                p.add(egui::Shape::convex_polygon(vec![egui::pos2(r.right() - 260.0, r.top()), egui::pos2(r.right(), r.top()), egui::pos2(r.right(), r.top() + 150.0)], deco, Stroke::NONE));
                p.add(egui::Shape::convex_polygon(vec![egui::pos2(r.left(), r.bottom() - 130.0), egui::pos2(r.left() + 220.0, r.bottom()), egui::pos2(r.left(), r.bottom())], deco, Stroke::NONE));

                let top_pad = ((ui.available_height() - 520.0) * 0.12).clamp(0.0, 70.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let avail = ui.available_width();
                    let w = avail.min(CONTENT_MAX_WIDTH);
                    let pad = ((avail - w) / 2.0).max(0.0);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(pad);
                        ui.vertical(|ui| {
                            ui.set_width(w);
                            // item_spacing.x đã bị đặt 0 ở trên → trả lại để các thẻ/ô không dính nhau
                            ui.spacing_mut().item_spacing.x = 12.0;
                            ui.add_space(top_pad);
                            match self.current_tab {
                                Tab::Quest => self.render_quest_tab(ui),
                                Tab::Badge => self.render_badge_tab(ui),
                                Tab::HypeSquad => self.render_hs_tab(ui),
                            }
                            ui.add_space(16.0);
                        });
                    });
                });
            });
    }
}

impl JokerApp {
    fn render_quest_tab(&mut self, ui: &mut egui::Ui) {
        let accent = accent_of(Tab::Quest);
        page_header(ui, "Cày Quest", "Tự động nhận và hoàn thành tất cả quest Discord của bạn.", accent);

        let logged_in = self.user_info.is_some();
        if !self.quest_running {
            if action_button(ui, "BẮT ĐẦU", accent, logged_in) { self.start_quest(); }
            login_hint(ui, logged_in);
        } else if action_button(ui, "DỪNG", STOP, true) {
            if let Some(c) = self.quest_cancel.take() { c.cancel(); }
        }
        ui.add_space(14.0);

        let cw = (ui.available_width() - 24.0) / 3.0;
        ui.horizontal(|ui| {
            stat_chip(ui, cw, "HOÀN THÀNH", &self.quest_completed.len().to_string(), GREEN);
            stat_chip(ui, cw, "LỖI", &self.quest_failed.len().to_string(), RED);
            let (txt, col) = if self.quest_running { ("Đang chạy", accent) } else { ("Sẵn sàng", TEXT_MUTED) };
            stat_chip(ui, cw, "TRẠNG THÁI", txt, col);
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            ui.set_min_height(110.0);
            ui.vertical_centered(|ui| {
                if let Some((name, task, done, needed)) = &self.current_quest {
                    ui.label(RichText::new(format!("Đang cày: {}", name)).size(17.0).family(bold()).color(Color32::from_rgb(255, 150, 160)));
                    ui.add_space(2.0);
                    ui.label(RichText::new(format!("Tác vụ: {}", task)).color(TEXT_MUTED).size(12.5));
                    ui.add_space(16.0);
                    let pct = if *needed > 0.0 { (*done / *needed).min(1.0) as f32 } else { 0.0 };
                    progress_bar(ui, pct, &format!("{:.0}/{:.0}s  ·  {:.0}%", done, needed, pct * 100.0), accent, true);
                } else if self.quest_running {
                    ui.add_space(22.0);
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(RichText::new("Đang quét danh sách quest...").italics().color(TEXT_MUTED));
                } else {
                    ui.add_space(34.0);
                    ui.label(RichText::new("Chưa có tiến trình nào đang chạy.").color(TEXT_MUTED).italics());
                }
            });
        });

        if !self.quest_completed.is_empty() || !self.quest_failed.is_empty() {
            ui.add_space(14.0);
            card(ui, GREEN, |ui| {
                egui::ScrollArea::vertical().max_height(150.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for n in &self.quest_completed { ui.label(RichText::new(format!("[OK] {}", n)).color(GREEN).size(13.0)); }
                    for (n, r) in &self.quest_failed { ui.label(RichText::new(format!("[X] {}: {}", n, r)).color(RED).size(13.0)); }
                });
            });
        }

        if let Some((c, f, reason)) = &self.quest_all_done {
            ui.add_space(14.0);
            egui::Frame::default().fill(Color32::from_rgb(52, 30, 36)).stroke(Stroke::new(1.0, Color32::from_rgb(110, 50, 60))).rounding(10.0).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!(">> {} (OK: {} / Lỗi: {})", reason, c, f)).color(Color32::from_rgb(255, 190, 196)).family(bold()));
            });
        }
    }

    fn render_badge_tab(&mut self, ui: &mut egui::Ui) {
        let accent = accent_of(Tab::Badge);
        page_header(ui, "Buff Badge", "Buff giờ chơi + số game để mở khoá badge Discord.", accent);

        let running = self.badge_running;
        card(ui, accent, |ui| {
            let label_w = 110.0;
            let combo_w = (ui.available_width() - label_w - 20.0).max(160.0);
            egui::Grid::new("badge_grid").num_columns(2).min_col_width(label_w).spacing([20.0, 16.0]).show(ui, |ui| {
                ui.label(RichText::new("Game Time").color(TEXT_NORMAL).family(bold()));
                ui.add_enabled_ui(!running, |ui| {
                    egui::ComboBox::from_id_salt("time_milestone")
                        .selected_text(find_time(&self.badge_time_key).map(|m| format!("Mốc {} - {} ({} giờ)", m.key, m.name, m.hours)).unwrap_or_else(|| "-- Chọn --".into()))
                        .width(combo_w)
                        .show_ui(ui, |ui| {
                            for m in TIME_MILESTONES { ui.selectable_value(&mut self.badge_time_key, m.key.to_string(), format!("Mốc {} - {} ({} giờ)", m.key, m.name, m.hours)); }
                        });
                });
                ui.end_row();

                ui.label(RichText::new("Game Variety").color(TEXT_NORMAL).family(bold()));
                ui.add_enabled_ui(!running, |ui| {
                    egui::ComboBox::from_id_salt("game_milestone")
                        .selected_text(find_games(&self.badge_game_key).map(|m| format!("Mốc {} - {} ({} game)", m.key, m.name, m.games)).unwrap_or_else(|| "-- Chọn --".into()))
                        .width(combo_w)
                        .show_ui(ui, |ui| {
                            for m in GAME_MILESTONES { ui.selectable_value(&mut self.badge_game_key, m.key.to_string(), format!("Mốc {} - {} ({} game)", m.key, m.name, m.games)); }
                        });
                });
                ui.end_row();

                ui.label(RichText::new("Fingerprint").color(TEXT_NORMAL).family(bold()));
                ui.add_enabled(!running, egui::TextEdit::singleline(&mut self.badge_fingerprint).hint_text("(Tùy chọn)").margin(Margin::symmetric(8.0, 6.0)).desired_width(combo_w - 16.0));
                ui.end_row();
            });

            if !self.badge_fingerprint.is_empty() && !is_likely_fingerprint(&self.badge_fingerprint) {
                ui.add_space(6.0);
                ui.label(RichText::new("Fingerprint không đúng định dạng.").size(12.0).color(RED));
            }
        });

        if let (Some(t), Some(g)) = (find_time(&self.badge_time_key), find_games(&self.badge_game_key)) {
            let hpg = t.hours as f64 / g.games as f64;
            ui.add_space(14.0);
            let cw = (ui.available_width() - 24.0) / 3.0;
            ui.horizontal(|ui| {
                stat_chip(ui, cw, "SỐ GAME", &g.games.to_string(), accent);
                stat_chip(ui, cw, "TỔNG GIỜ", &t.hours.to_string(), Color32::from_rgb(255, 205, 120));
                stat_chip(ui, cw, "MỖI GAME", &format!("{:.1}h", hpg), TEXT_NORMAL);
            });
        }

        ui.add_space(16.0);
        let logged_in = self.user_info.is_some();
        let can_start = !self.badge_running && logged_in && find_time(&self.badge_time_key).is_some() && find_games(&self.badge_game_key).is_some() && (self.badge_fingerprint.is_empty() || is_likely_fingerprint(&self.badge_fingerprint));

        if !self.badge_running {
            if action_button(ui, "BẮT ĐẦU", accent, can_start) { self.start_badge(); }
            login_hint(ui, logged_in);
        } else if action_button(ui, "DỪNG", STOP, true) {
            if let Some(c) = self.badge_cancel.take() { c.cancel(); }
        }

        if self.badge_running || self.badge_progress.2 > 0 {
            ui.add_space(16.0);
            let (done, failed, total) = self.badge_progress;
            let pct = if total > 0 { done as f32 / total as f32 } else { 0.0 };
            progress_bar(ui, pct, &format!("{}/{} game  ·  lỗi: {}", done, total, failed), accent, self.badge_running);
            if !self.badge_desc.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&self.badge_desc).italics().color(TEXT_MUTED).size(13.0));
            }
        }

        if let Some((ok, msg)) = &self.badge_result {
            ui.add_space(12.0);
            ui.label(RichText::new(msg).color(if *ok { GREEN } else { RED }).family(bold()));
        }
    }

    fn render_hs_tab(&mut self, ui: &mut egui::Ui) {
        let accent = accent_of(Tab::HypeSquad);
        page_header(ui, "HypeSquad", "Đổi nhà HypeSquad để nhận huy hiệu.", accent);

        let houses: [(&str, &str, &str, Color32); 4] = [
            ("bravery", "Bravery", "Dũng Cảm", Color32::from_rgb(157, 104, 248)),
            ("brilliance", "Brilliance", "Kiệt Xuất", Color32::from_rgb(244, 123, 103)),
            ("balance", "Balance", "Cân Bằng", Color32::from_rgb(69, 221, 192)),
            ("leave", "Gỡ HypeSquad", "Rời khỏi nhà hiện tại", Color32::from_rgb(130, 134, 148)),
        ];
        let tile_w = (ui.available_width() - 12.0) / 2.0;
        let mut chosen: Option<&str> = None;
        for row in houses.chunks(2) {
            ui.horizontal(|ui| {
                for (key, name, sub, color) in row {
                    let selected = self.hs_house == *key;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(tile_w, 70.0), Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let p = ui.painter();
                    let fill = if selected { color.gamma_multiply(0.18) } else if resp.hovered() { Color32::from_rgb(42, 42, 50) } else { CARD };
                    let stroke = if selected { Stroke::new(2.0, *color) } else { Stroke::new(1.0, BORDER) };
                    p.rect_filled(rect, Rounding::same(12.0), fill);
                    p.rect_stroke(rect, Rounding::same(12.0), stroke);
                    let c = egui::pos2(rect.left() + 34.0, rect.center().y);
                    p.circle_filled(c, 15.0, color.gamma_multiply(0.25));
                    p.circle_filled(c, 9.0, *color);
                    if selected { p.circle_stroke(c, 15.0, Stroke::new(1.5, *color)); }
                    p.text(egui::pos2(rect.left() + 62.0, rect.center().y - 10.0), Align2::LEFT_CENTER, *name, FontId::new(15.5, bold()), Color32::WHITE);
                    p.text(egui::pos2(rect.left() + 62.0, rect.center().y + 11.0), Align2::LEFT_CENTER, *sub, FontId::new(12.0, FontFamily::Proportional), TEXT_MUTED);
                    if resp.clicked() { chosen = Some(*key); }
                }
            });
        }
        if let Some(k) = chosen { self.hs_house = k.to_string(); }

        ui.add_space(16.0);
        let logged_in = self.user_info.is_some();
        let can_start = !self.hs_running && logged_in && !self.hs_house.is_empty();
        if action_button(ui, "ÁP DỤNG", accent, can_start) { self.start_hypesquad(); }
        if self.hs_running {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Đang xử lý...").color(TEXT_MUTED));
            });
        } else {
            login_hint(ui, logged_in);
        }

        if let Some((ok, msg)) = &self.hs_result {
            ui.add_space(12.0);
            ui.label(RichText::new(msg).color(if *ok { GREEN } else { RED }).family(bold()));
        }
    }
}
