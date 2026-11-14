use crate::avatar::{fetch_avatar, AVATAR_SIZE};
use crate::badge::{
    find_games, find_time, generate_fingerprint, is_likely_fingerprint, run_badge_session, BadgeJob, GAME_MILESTONES,
    TIME_MILESTONES,
};
use crate::config::AppConfig;
use crate::events::ProgressEvent;
use crate::i18n::{self, t, tf, Lang};
use crate::platform;
use crate::shared::Shared;
use crate::hypesquad::set_hypesquad_house;
use crate::log_writer::LogWriter;
use crate::quest::{list_quests, run_quest_session, validate_token, watch_quests, QuestInfo, UserInfo};
use crate::ui::theme::{self, *};
use crate::ui::widgets::*;
use crate::ui::Tab;
use crate::updater::{self, UpdateInfo};
use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Id, Layout, Order, Rect, RichText, Sense, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;
use regex::Regex;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

const CONTENT_MAX_WIDTH: f32 = 720.0;
const MAX_LOG_LINES: usize = 2000;
const WATCH_INTERVAL_SECS: u64 = 30 * 60;
const KOFI_URL: &str = "https://ko-fi.com/2amkeyl";
const REPO_URL: &str = "https://github.com/2amkeyl/joker-tool";

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogKind { Ok, Err, Info }

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogFilter { All, Errors }

struct LogEntry { time: String, kind: LogKind, text: String }

struct Toast { kind: ToastKind, text: String, born: Instant }
impl Toast {
    fn life(&self) -> Duration { if self.kind == ToastKind::Err { Duration::from_millis(5500) } else { Duration::from_millis(3800) } }
    fn alpha(&self) -> f32 {
        let age = self.born.elapsed().as_secs_f32();
        let life = self.life().as_secs_f32();
        let fade_in = (age / 0.18).clamp(0.0, 1.0);
        let fade_out = ((life - age) / 0.5).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }
}

struct QuestRow { name: String, ok: bool, reason: String }

pub struct JokerApp {
    _rt: tokio::runtime::Runtime,
    rt: tokio::runtime::Handle,
    cfg: AppConfig,
    log: LogWriter,
    tx: UnboundedSender<ProgressEvent>,
    rx: UnboundedReceiver<ProgressEvent>,
    shared: Arc<Shared>,
    ctx: egui::Context,
    hwnd: isize,
    tray: Option<platform::Tray>,

    token_input: String,
    token_error: Option<String>,
    remember: bool,
    show_token: bool,
    user_info: Option<UserInfo>,
    checking_token: bool,

    current_tab: Tab,
    tab_changed: Instant,

    quest_running: bool,
    quest_cancel: Option<CancellationToken>,
    current_quest: Option<(String, String, f64, f64)>,
    quest_rows: Vec<QuestRow>,
    quest_list: Option<Vec<QuestInfo>>,
    quest_selected: HashSet<String>,
    quest_scanning: bool,
    quest_scan_err: Option<String>,

    badge_time_key: String,
    badge_game_key: String,
    badge_fingerprint: String,
    badge_cookie: String,
    badge_fp_busy: bool,
    badge_running: bool,
    badge_cancel: Option<CancellationToken>,
    badge_progress: (usize, usize, usize),
    badge_desc: String,
    badge_result: Option<(bool, String)>,

    hs_house: String,
    hs_running: bool,
    hs_result: Option<(bool, String)>,

    log_entries: Vec<LogEntry>,
    log_filter: LogFilter,
    log_open: bool,
    auto_scroll: bool,

    toasts: Vec<Toast>,
    confirm_close: bool,
    allow_close: bool,
    confirm_clear_stats: bool,
    onboard_step: usize,
    onboard_ack: bool,
    rate_limit_until: Option<Instant>,
    watch_cancel: Option<CancellationToken>,
    palette_open: bool,
    palette_query: String,
    palette_sel: usize,
    start_minimized: bool,
    frame_no: u32,
    sys_theme_check: Instant,

    update_info: Option<UpdateInfo>,
    update_checking: bool,
    update_installing: bool,
    update_uptodate: bool,

    avatar_pending: Option<Vec<u8>>,
    avatar_tex: Option<egui::TextureHandle>,
    logo_tex: Option<egui::TextureHandle>,
}

fn load_logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(include_bytes!("../../assets/icon.png")).ok()?
        .resize_exact(128, 128, image::imageops::FilterType::Lanczos3).to_rgba8();
    Some(ctx.load_texture("joker_logo", egui::ColorImage::from_rgba_unmultiplied([128, 128], img.as_raw()), egui::TextureOptions::LINEAR))
}

fn task_label(task: &str) -> &'static str {
    match task {
        "WATCH_VIDEO" => t("task.watch_video"),
        "WATCH_VIDEO_ON_MOBILE" => t("task.watch_video_mobile"),
        "PLAY_ON_DESKTOP" => t("task.play_desktop"),
        "STREAM_ON_DESKTOP" => t("task.stream_desktop"),
        "PLAY_ACTIVITY" => t("task.play_activity"),
        _ => t("task.other"),
    }
}

fn sort_quests(list: &mut [QuestInfo]) {
    fn key(q: &QuestInfo) -> (u8, i64, String) {
        let group = if q.selectable() && q.enrolled && q.done > 0.0 { 0 }
            else if q.selectable() { 1 }
            else if !q.completed { 2 }
            else { 3 };
        let expiry = q.expires.as_deref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d| d.timestamp()).unwrap_or(i64::MAX);
        (group, expiry, q.name.to_lowercase())
    }
    list.sort_by_key(key);
}

fn fmt_mmss(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn fmt_dur(secs: u64) -> String {
    if secs >= 3600 { format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60) }
    else if secs >= 60 { format!("{}m {:02}s", secs / 60, secs % 60) }
    else { format!("{}s", secs) }
}

impl JokerApp {
    pub fn new(
        rt: tokio::runtime::Runtime, cfg: AppConfig, log: LogWriter,
        tx: UnboundedSender<ProgressEvent>, rx: UnboundedReceiver<ProgressEvent>,
        shared: Arc<Shared>, ctx: egui::Context, hwnd: isize,
    ) -> Self {
        updater::cleanup_old();
        let rt_handle = rt.handle().clone();

        let token_input = cfg.token.clone();
        let saved_cookie = cfg.cookie.clone();
        let saved_fp = cfg.badge_fingerprint.clone();
        let remember = !token_input.is_empty();
        
        let badge_time_key = if cfg.badge_time_key.is_empty() { "5".into() } else { cfg.badge_time_key.clone() };
        let badge_game_key = if cfg.badge_game_key.is_empty() { "3".into() } else { cfg.badge_game_key.clone() };
        let onboarded = cfg.onboarded;
        // Keep the application at its designed scale; no application zoom shortcuts.
        ctx.options_mut(|options| options.zoom_with_keyboard = false);

        let mut app = Self {
            _rt: rt, rt: rt_handle, cfg, log, tx, rx, shared, ctx, hwnd, tray: None,
            token_input, token_error: None,
            remember, show_token: false,
            user_info: None, checking_token: false, current_tab: Tab::Quest,
            tab_changed: Instant::now() - Duration::from_secs(5),
            quest_running: false, quest_cancel: None, current_quest: None, quest_rows: Vec::new(),
            quest_list: None, quest_selected: HashSet::new(), quest_scanning: false, quest_scan_err: None,
            badge_time_key, badge_game_key, badge_fingerprint: saved_fp, badge_cookie: saved_cookie,
            badge_fp_busy: false, badge_running: false, badge_cancel: None, badge_progress: (0, 0, 0),
            badge_desc: String::new(), badge_result: None, hs_house: "bravery".into(), hs_running: false, hs_result: None,
            log_entries: Vec::new(), log_filter: LogFilter::All, log_open: true, auto_scroll: true,
            toasts: Vec::new(),
            confirm_close: false, allow_close: false, confirm_clear_stats: false,
            onboard_step: if onboarded { usize::MAX } else { 0 }, onboard_ack: false,
            rate_limit_until: None, watch_cancel: None, palette_open: false, palette_query: String::new(),
            palette_sel: 0, start_minimized: std::env::args().any(|a| a == "--minimized"), frame_no: 0,
            sys_theme_check: Instant::now(),
            update_info: None, update_checking: false, update_installing: false, update_uptodate: false,
            avatar_pending: None, avatar_tex: None, logo_tex: None,
        };
        app.apply_tray(false);
        if app.cfg.autostart { let _ = platform::set_autostart(true); }
        app.check_update(false);
        if remember && onboarded { app.check_token(); }
        app
    }

    fn onboarding_active(&self) -> bool { self.onboard_step != usize::MAX }
    fn account_name(&self) -> String { self.user_info.as_ref().map(|u| u.display()).unwrap_or_default() }
    
    fn is_busy(&self) -> bool {
        self.quest_running || self.badge_running || self.hs_running || self.quest_scanning || self.badge_fp_busy
    }

    fn activity_label(&self) -> Option<&'static str> {
        if self.quest_running { Some(t("activity.quest")) }
        else if self.badge_running { Some(t("activity.badge")) }
        else if self.hs_running { Some(t("activity.hype")) }
        else { None }
    }

    fn tab_busy(&self, tab: Tab) -> bool {
        match tab {
            Tab::Quest => self.quest_running || self.quest_scanning,
            Tab::Badge => self.badge_running || self.badge_fp_busy,
            Tab::HypeSquad => self.hs_running,
            Tab::Stats | Tab::Settings => false,
        }
    }

    fn set_tab(&mut self, tab: Tab) {
        if self.current_tab != tab { self.current_tab = tab; self.tab_changed = Instant::now(); }
    }

    fn push_log(&mut self, msg: String) {
        self.log.write(&msg);
        let kind = if msg.contains("[X]") || msg.contains("⚠️") { LogKind::Err }
            else if msg.contains("[OK]") || msg.contains("✓") { LogKind::Ok }
            else { LogKind::Info };
        let text = msg.trim_start_matches("[OK] ").trim_start_matches("[X] ")
            .trim_start_matches("→ ").trim_start_matches(">> ").to_string();
        self.log_entries.push(LogEntry { time: chrono::Local::now().format("%H:%M:%S").to_string(), kind, text });
    }

    fn toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.toasts.push(Toast { kind, text: text.into(), born: Instant::now() });
        if self.toasts.len() > 4 { self.toasts.remove(0); }
    }

    fn persist_config(&mut self) {
        if self.user_info.is_some() && self.remember {
            self.cfg.token = self.token_input.trim().to_string();
            self.cfg.cookie = self.badge_cookie.trim().to_string();
            self.cfg.badge_fingerprint = self.badge_fingerprint.trim().to_string();
        } else if !self.remember {
            self.cfg.token.clear();
            self.cfg.cookie.clear();
            self.cfg.badge_fingerprint.clear();
        }

        self.cfg.badge_time_key = self.badge_time_key.clone();
        self.cfg.badge_game_key = self.badge_game_key.clone();
        if let Err(e) = self.cfg.save() {
            self.push_log(format!("[X] Config: {}", e));
            self.toast(ToastKind::Err, t("toast.save_fail"));
        }
    }

    fn reset_session_views(&mut self) {
        self.quest_list = None; self.quest_selected.clear(); self.quest_scan_err = None;
        self.quest_rows.clear(); self.current_quest = None; self.badge_result = None; self.hs_result = None;
    }

    fn drop_session(&mut self) {
        self.stop_watcher(); self.user_info = None; self.avatar_tex = None; self.avatar_pending = None; self.reset_session_views();
    }

    fn logout(&mut self) {
        if self.is_busy() {
            self.toast(ToastKind::Err, t("toast.stop_first"));
            return;
        }
        self.token_input.clear();
        self.badge_cookie.clear();
        self.badge_fingerprint.clear();
        self.remember = false;
        self.token_error = None;
        self.drop_session();
        self.persist_config();
        self.toast(ToastKind::Info, t("toast.logged_out"));
    }

    fn set_language(&mut self, l: Lang) {
        i18n::set_lang(l);
        self.cfg.lang = l.code().to_string();
        self.persist_config();
        self.apply_tray(true);
    }

    fn apply_tray(&mut self, rebuild: bool) {
        if rebuild { self.tray = None; }
        if self.cfg.tray_on_close {
            if self.tray.is_none() {
                self.tray = platform::create_tray(
                    self.ctx.clone(),
                    self.hwnd,
                    platform::TrayLabels {
                        open: t("tray.open"),
                        quest: t("tray.quest"),
                        badge: t("tray.badge"),
                        hypesquad: t("tray.hypesquad"),
                        stop_tasks: t("tray.stop_tasks"),
                        stats: t("tray.stats"),
                        settings: t("tray.settings"),
                        quit: t("tray.quit"),
                    },
                );
                if self.tray.is_none() {
                    self.cfg.tray_on_close = false;
                    self.toast(ToastKind::Err, t("settings.tray_fail"));
                }
            }
        } else {
            self.tray = None;
        }
    }

    fn handle_tray_action(&mut self, action: platform::TrayAction) {
        use platform::TrayAction;

        match action {
            TrayAction::Open => platform::show_window(self.hwnd),
            TrayAction::OpenQuest => {
                self.set_tab(Tab::Quest);
                platform::show_window(self.hwnd);
            }
            TrayAction::OpenBadge => {
                self.set_tab(Tab::Badge);
                platform::show_window(self.hwnd);
            }
            TrayAction::OpenHypeSquad => {
                self.set_tab(Tab::HypeSquad);
                platform::show_window(self.hwnd);
            }
            TrayAction::StopTasks => {
                if self.quest_running || self.badge_running {
                    self.stop_all();
                    self.toast(ToastKind::Info, t("tray.tasks_stopping"));
                    if self.cfg.notify { platform::notify("Joker Tool", t("tray.tasks_stopping")); }
                } else {
                    self.toast(ToastKind::Info, t("tray.no_tasks"));
                    if self.cfg.notify { platform::notify("Joker Tool", t("tray.no_tasks")); }
                }
            }
            TrayAction::OpenStats => {
                self.set_tab(Tab::Stats);
                platform::show_window(self.hwnd);
            }
            TrayAction::OpenSettings => {
                self.set_tab(Tab::Settings);
                platform::show_window(self.hwnd);
            }
            TrayAction::Quit => {
                if self.is_busy() {
                    self.confirm_close = true;
                    platform::show_window(self.hwnd);
                } else {
                    self.allow_close = true;
                    self.ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        self.ctx.request_repaint();
    }

    fn handle_event(&mut self, ev: ProgressEvent) {
        match ev {
            ProgressEvent::Log(msg) => self.push_log(msg),
            ProgressEvent::QuestList(res) => {
                self.quest_scanning = false;
                match res {
                    Ok(mut list) => {
                        sort_quests(&mut list);
                        self.quest_selected = list.iter().filter(|q| q.selectable()).map(|q| q.id.clone()).collect();
                        self.quest_scan_err = None;
                        let n = list.iter().filter(|q| q.selectable()).count();
                        self.push_log(format!("[OK] {}", tf("log.scan_ok", &[&list.len().to_string(), &n.to_string()])));
                        self.quest_list = Some(list);
                    }
                    Err(e) => {
                        let shown = if e == "network" { t("quest.scan_network").to_string() } else { e };
                        self.push_log(format!("[X] {}", tf("quest.scan_err", &[&shown])));
                        self.quest_scan_err = Some(shown);
                    }
                }
            }
            ProgressEvent::RateLimit { secs } => {
                let until = Instant::now() + Duration::from_secs(secs);
                if self.rate_limit_until.map(|u| u < until).unwrap_or(true) { self.rate_limit_until = Some(until); }
            }
            ProgressEvent::QuestsFound { new, all_ids } => {
                let mut seen: Vec<String> = all_ids;
                if seen.len() > 300 { seen.drain(0..seen.len() - 300); }
                
                let old_set: HashSet<_> = self.cfg.seen_quests.iter().cloned().collect();
                let new_set: HashSet<_> = seen.iter().cloned().collect();
                if old_set != new_set {
                    self.cfg.seen_quests = seen;
                    let _ = self.cfg.save();
                }
                
                if !new.is_empty() {
                    let first = new[0].name.clone();
                    let message = tf("toast.new_quests", &[&new.len().to_string(), &first]);
                    self.push_log(format!("[OK] {}", tf("log.new_quests", &[&new.len().to_string(), &first])));
                    self.toast(ToastKind::Info, message.clone());
                }
            }
            ProgressEvent::TokenStatus { valid, msg, user } => {
                self.checking_token = false;
                if valid {
                    self.user_info = user;
                    self.token_error = None;
                    if let Some(u) = self.user_info.clone() {
                        let greeting = tf("toast.hello", &[&u.display()]);
                        let tx = self.tx.clone();
                        self.rt.spawn(async move {
                            if let Some(rgba) = fetch_avatar(&u).await {
                                let _ = tx.send(ProgressEvent::Avatar { user_id: u.id.clone(), rgba });
                            }
                        });
                        self.toast(ToastKind::Ok, greeting.clone());
                    }
                    self.push_log(format!("[OK] {}", msg));
                    self.persist_config();
                    self.restart_watcher();
                } else {
                    self.user_info = None;
                    self.token_error = Some(msg.clone());
                    self.push_log(format!("[X] {}", msg));
                    self.toast(ToastKind::Err, msg.clone());
                }
            }
            ProgressEvent::UpdateChecked { result, manual } => {
                self.update_checking = false;
                match result {
                    Ok(Some(info)) => {
                        let message = tf("toast.update_available", &[&info.version]);
                        self.push_log(format!("[OK] {}", tf("log.update_available", &[&info.version])));
                        self.toast(ToastKind::Info, message.clone());
                        self.update_info = Some(info);
                    }
                    Ok(None) => {
                        self.update_info = None;
                        self.update_uptodate = true;
                        if manual { self.toast(ToastKind::Ok, t("toast.uptodate")); }
                    }
                    Err(e) => {
                        if manual {
                            self.push_log(format!("[X] {}", tf("log.update_check_fail", &[&e])));
                            self.toast(ToastKind::Err, t("toast.update_check_fail"));
                        }
                    }
                }
            }
            ProgressEvent::UpdateInstalled(res) => {
                self.update_installing = false;
                match res {
                    Ok(()) => {
                        self.push_log(format!("[OK] {}", t("log.update_done")));
                        updater::restart();
                    }
                    Err(e) => {
                        self.push_log(format!("[X] {}", tf("log.update_fail", &[&e])));
                        self.toast(ToastKind::Err, t("toast.update_fail"));
                    }
                }
            }
            ProgressEvent::FingerprintResult { ok, value } => {
                self.badge_fp_busy = false;
                if ok {
                    self.badge_fingerprint = value;
                    self.push_log(format!("[OK] {}", t("log.fp_ok")));
                    self.toast(ToastKind::Ok, t("toast.fp_ok"));
                    self.persist_config();
                } else {
                    self.push_log(format!("[X] {}", value));
                    self.toast(ToastKind::Err, value);
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
                    self.push_log(format!("[OK] {}", name));
                    self.quest_rows.push(QuestRow { name, ok: true, reason: String::new() });
                } else {
                    let r = reason.unwrap_or_default();
                    self.push_log(format!("[X] {}: {}", name, r));
                    self.quest_rows.push(QuestRow { name, ok: false, reason: r });
                }
                self.current_quest = None;
            }
            ProgressEvent::QuestAllDone { completed, failed, reason } => {
                self.quest_running = false;
                self.quest_cancel = None;
                self.current_quest = None;
                self.quest_list = None;
                self.restart_watcher();
                self.push_log(format!(">> {} (OK: {} / {}: {})", reason, completed, t("common.error"), failed));
                if failed == 0 && completed > 0 {
                    let message = tf("toast.quest_all_ok", &[&completed.to_string()]);
                    self.toast(ToastKind::Ok, message.clone());
                } else if completed == 0 && failed == 0 {
                    self.toast(ToastKind::Info, reason.clone());
                } else {
                    let message = tf("toast.quest_mixed", &[&completed.to_string(), &failed.to_string()]);
                    self.toast(ToastKind::Info, message.clone());
                }
            }
            ProgressEvent::BadgeUpdate { done, failed, total, desc } => {
                self.badge_progress = (done, failed, total);
                self.badge_desc = desc;
            }
            ProgressEvent::BadgeDone { success, done, failed: _, total, reason } => {
                self.badge_running = false;
                self.badge_cancel = None;
                let msg = if success { tf("badge.done", &[&done.to_string(), &total.to_string()]) } else { tf("badge.fail", &[&reason]) };
                self.push_log(format!("{} {}", if success { "[OK]" } else { "[X]" }, msg));
                if success {
                    self.toast(ToastKind::Ok, msg.clone());
                } else {
                    self.toast(ToastKind::Err, msg.clone());
                }
                self.badge_result = Some((success, msg));
            }
            ProgressEvent::HypeSquadResult { success, msg } => {
                self.hs_running = false;
                self.push_log(format!("{} {}", if success { "[OK]" } else { "[X]" }, msg));
                self.toast(if success { ToastKind::Ok } else { ToastKind::Err }, msg.clone());
                self.hs_result = Some((success, msg));
            }
        }
    }

    fn check_token(&mut self) {
        let token = self.token_input.trim().to_string();
        if token.is_empty() { return; }
        self.checking_token = true;
        self.token_error = None;
        self.user_info = None;
        self.avatar_tex = None;
        self.avatar_pending = None;
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            match validate_token(&token).await {
                Ok(info) => {
                    let _ = tx.send(ProgressEvent::TokenStatus { valid: true, msg: tf("log.login_ok", &[&info.display()]), user: Some(info) });
                }
                Err(e) => {
                    let _ = tx.send(ProgressEvent::TokenStatus { valid: false, msg: e, user: None });
                }
            }
        });
    }

    fn scan_quests(&mut self) {
        let token = self.token_input.trim().to_string();
        if token.is_empty() || self.quest_scanning || self.quest_running { return; }
        self.quest_scanning = true;
        self.quest_scan_err = None;
        let tx = self.tx.clone();
        let cancel = CancellationToken::new();
        self.rt.spawn(async move {
            list_quests(token, tx, cancel).await;
        });
    }

    fn start_quest(&mut self, selection: Option<HashSet<String>>) {
        let token = self.token_input.trim().to_string();
        if token.is_empty() { return; }
        let cancel = CancellationToken::new();
        self.quest_cancel = Some(cancel.clone());
        self.quest_running = true;
        self.stop_watcher();
        self.current_quest = None;
        self.quest_rows.clear();
        self.shared.stats.lock().begin("quest", &self.account_name());
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            run_quest_session(token, tx, cancel, selection).await;
        });
    }

    fn start_badge(&mut self) {
        let token = self.token_input.trim().to_string();
        let fp = self.badge_fingerprint.trim().to_string();
        let cookie = self.badge_cookie.trim().to_string();
        if cookie.is_empty() {
            self.toast(ToastKind::Err, t("badge.cookie_required"));
            return;
        }
        let time = match find_time(&self.badge_time_key) { Some(x) => x, None => return };
        let game = match find_games(&self.badge_game_key) { Some(g) => g, None => return };
        let hours_per_game = time.hours as f64 / game.games as f64;
        let games_count = game.games as usize;

        self.persist_config();

        let cancel = CancellationToken::new();
        self.badge_cancel = Some(cancel.clone());
        self.badge_running = true;
        self.badge_progress = (0, 0, games_count);
        self.badge_desc = t("badge.starting").to_string();
        self.badge_result = None;
        self.shared.stats.lock().begin("badge", &self.account_name());
        let tx = self.tx.clone();
        let job = BadgeJob { token, cookie, fingerprint: fp, hours_per_game, games_count };
        self.rt.spawn(async move {
            run_badge_session(job, tx, cancel).await;
        });
    }

    fn start_fingerprint(&mut self, picked: Option<PathBuf>) {
        if self.badge_fp_busy { return; }
        self.badge_fp_busy = true;
        let token = self.token_input.trim().to_string();
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            let path = match picked {
                Some(p) => p,
                None => {
                    let Some(p) = rfd::FileDialog::new().set_title(t("fp.dialog_title"))
                        .add_filter("Executable (.exe)", &["exe"]).pick_file() 
                    else {
                        let _ = tx.send(ProgressEvent::FingerprintResult { ok: false, value: t("fp.cancelled").into() });
                        return;
                    };
                    p
                }
            };

            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let file_stem = path.file_stem().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            let rt_check = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(r) => r,
                Err(_) => {
                    let _ = tx.send(ProgressEvent::FingerprintResult { ok: false, value: t("fp.runtime_err").into() });
                    return;
                }
            };

            let tx_check = tx.clone();
            let is_known = rt_check.block_on(async move {
                let games = crate::badge::load_games(&tx_check).await;
                games.iter().any(|g| {
                    let game_exe_stem = g.exe.to_lowercase().replace(".exe", "");
                    game_exe_stem == file_stem || game_exe_stem.contains(&file_stem) || file_stem.contains(&game_exe_stem)
                })
            });

            if !is_known {
                let _ = tx.send(ProgressEvent::FingerprintResult { ok: false, value: tf("fp.unknown_game", &[&file_name]) });
                return;
            }
            let ev = match generate_fingerprint(&token, &path.to_string_lossy()) {
                Ok(fp) => ProgressEvent::FingerprintResult { ok: true, value: fp },
                Err(e) => ProgressEvent::FingerprintResult { ok: false, value: e },
            };
            let _ = tx.send(ev);
        });
    }

    fn on_file_dropped(&mut self, path: PathBuf) {
        let is_exe = path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false);
        if !is_exe { self.toast(ToastKind::Err, t("fp.only_exe")); return; }
        if self.user_info.is_none() { self.toast(ToastKind::Err, t("fp.login_first")); return; }
        if self.badge_running { self.toast(ToastKind::Err, t("fp.stop_badge_first")); return; }
        self.set_tab(Tab::Badge);
        self.start_fingerprint(Some(path));
    }

    fn check_update(&mut self, manual: bool) {
        if self.update_checking || self.update_installing { return; }
        self.update_checking = true;
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let result = updater::check().await;
            let _ = tx.send(ProgressEvent::UpdateChecked { result, manual });
        });
    }

    fn install_update(&mut self) {
        let Some(info) = self.update_info.clone() else { return };
        if self.update_installing || self.is_busy() { return; }
        self.update_installing = true;
        self.push_log(tf("log.update_downloading", &[&info.version]));
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let res = updater::download_and_install(&info).await;
            let _ = tx.send(ProgressEvent::UpdateInstalled(res));
        });
    }

    fn start_hypesquad(&mut self) {
        let token = self.token_input.trim().to_string();
        let house = self.hs_house.clone();
        self.hs_running = true;
        self.hs_result = None;
        self.shared.stats.lock().begin("hype", &self.account_name());
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let (success, msg) = set_hypesquad_house(&token, &house).await;
            let _ = tx.send(ProgressEvent::HypeSquadResult { success, msg });
        });
    }

    fn stop_all(&mut self) {
        if let Some(c) = self.quest_cancel.take() { c.cancel(); }
        if let Some(c) = self.badge_cancel.take() { c.cancel(); }
    }

    fn stop_watcher(&mut self) {
        if let Some(c) = self.watch_cancel.take() { c.cancel(); }
    }

    fn restart_watcher(&mut self) {
        self.stop_watcher();
        if !self.cfg.watch_quests || self.user_info.is_none() || self.quest_running { return; }
        let token = self.token_input.trim().to_string();
        if token.is_empty() { return; }
        let cancel = CancellationToken::new();
        self.watch_cancel = Some(cancel.clone());
        let seen: HashSet<String> = self.cfg.seen_quests.iter().cloned().collect();
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            watch_quests(token, tx, cancel, Duration::from_secs(WATCH_INTERVAL_SECS), seen).await;
        });
    }

    fn resolve_light(&self) -> bool {
        match self.cfg.theme.as_str() {
            "light" => true,
            "system" => platform::system_prefers_light().unwrap_or(false),
            _ => false,
        }
    }

    fn menu_mode(&self) -> u8 {
        match self.cfg.theme.as_str() {
            "light" => 2,
            "system" => 0,
            _ => 1,
        }
    }

    fn refresh_theme(&mut self) {
        let light = self.resolve_light();
        theme::set_light(light);
        theme::set_accent(&self.cfg.accent);
        theme::apply_theme(&self.ctx);
        self.ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(
            if light { egui::SystemTheme::Light } else { egui::SystemTheme::Dark },
        ));
        platform::set_menu_theme(self.menu_mode());
    }

    fn apply_appearance(&mut self) {
        self.refresh_theme();
        self.persist_config();
    }

    fn set_autostart(&mut self, on: bool) {
        match platform::set_autostart(on) {
            Ok(()) => { self.cfg.autostart = on; self.persist_config(); }
            Err(_) => { self.cfg.autostart = false; self.toast(ToastKind::Err, t("settings.autostart_fail")); }
        }
    }

    fn redact(&self, input: &str) -> String {
        static TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z0-9_-]{23,28}\.[A-Za-z0-9_-]{6,7}\.[A-Za-z0-9_-]{27,}").unwrap());
        static MFA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"mfa\.[A-Za-z0-9_-]{20,}").unwrap());
        static ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{17,20}\b").unwrap());
        static EMAIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\w.+-]+@[\w-]+\.[\w.-]+").unwrap());
        static HOME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)([A-Z]:\\Users\\)[^\\/\s]+").unwrap());

        let mut out = input.to_string();
        let secrets: Vec<String> = vec![
            self.token_input.trim().to_string(),
            self.badge_cookie.trim().to_string(),
            self.badge_fingerprint.trim().to_string(),
            self.cfg.token.clone(),
            self.cfg.cookie.clone(),
            self.cfg.badge_fingerprint.clone(),
        ];
        for sec in secrets.into_iter().filter(|s| s.len() >= 8) {
            out = out.replace(&sec, "<secret>");
        }
        out = TOKEN.replace_all(&out, "<token>").into_owned();
        out = MFA.replace_all(&out, "<token>").into_owned();
        out = ID.replace_all(&out, "<id>").into_owned();
        out = EMAIL.replace_all(&out, "<email>").into_owned();
        out = HOME.replace_all(&out, "${1}<user>").into_owned();
        let names = vec![self.account_name()];
        for n in names.into_iter().filter(|n| n.chars().count() >= 3) {
            out = out.replace(&n, "<user>");
        }
        out
    }

    fn diagnostics_text(&self) -> String {
        let onoff = |b: bool| if b { "on" } else { "off" };
        let mut s = String::new();
        s.push_str(&format!("Joker Tool v{}{}\n", updater::CURRENT, if updater::is_prerelease(updater::CURRENT) { " (beta)" } else { "" }));
        s.push_str(&format!("OS: {} {} - {}\n", std::env::consts::OS, std::env::consts::ARCH, platform::os_version()));
        s.push_str(&format!(
            "Language: {} | Theme: {} | Accent: {} | Tray: {} | Notify: {} | Autostart: {} | Watch quests: {}\n",
            i18n::lang().code(),
            self.cfg.theme,
            self.cfg.accent,
            onoff(self.cfg.tray_on_close),
            onoff(self.cfg.notify),
            onoff(self.cfg.autostart),
            onoff(self.cfg.watch_quests),
        ));
        s.push_str(&format!("Logged in: {} | Remember account: {}\n", if self.user_info.is_some() { "yes" } else { "no" }, if self.remember { "yes" } else { "no" }));
        s.push_str(&format!(
            "Busy: quest={} badge={} hype={} | Update: {}\n",
            self.quest_running,
            self.badge_running,
            self.hs_running,
            match &self.update_info {
                Some(i) => format!("available {}", i.version),
                None => "none".to_string(),
            }
        ));
        s.push_str("\n--- last 40 log lines ---\n");
        let skip = self.log_entries.len().saturating_sub(40);
        for e in self.log_entries.iter().skip(skip) {
            s.push_str(&format!("[{}] {}\n", e.time, self.redact(&e.text)));
        }
        s
    }

    fn copy_diagnostics(&mut self) {
        let text = self.diagnostics_text();
        self.ctx.copy_text(text);
        self.toast(ToastKind::Ok, t("toast.diag_copied"));
    }

    fn load_textures(&mut self, ctx: &egui::Context) {
        if self.logo_tex.is_none() {
            self.logo_tex = load_logo_texture(ctx);
        }
        if let Some(rgba) = self.avatar_pending.take() {
            let n = AVATAR_SIZE as usize;
            if rgba.len() == n * n * 4 {
                self.avatar_tex = Some(ctx.load_texture(
                    "discord_avatar",
                    egui::ColorImage::from_rgba_unmultiplied([n, n], &rgba),
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
    }
}

mod onboarding;
mod palette;
mod tabs;
mod update;
