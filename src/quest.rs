use tokio::time::sleep;
use anyhow::Result;
use base64::Engine;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{json, Value};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use crate::events::ProgressEvent;

// ==========================================
// Constants
// ==========================================

const API_BASE: &str = "https://discord.com/api/v9";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) discord/1.0.9175 Chrome/128.0.6613.186 Electron/32.2.7 Safari/537.36";

pub const HEARTBEAT_INTERVAL: u64 = 20;
pub const MAX_FAILS: u32 = 5;
pub const MAX_ATTEMPTS: u32 = 3;
pub const FALLBACK_BUILD: u64 = 504_649;

pub const SUPPORTED_TASKS: &[&str] = &[
    "WATCH_VIDEO",
    "PLAY_ON_DESKTOP",
    "STREAM_ON_DESKTOP",
    "PLAY_ACTIVITY",
    "WATCH_VIDEO_ON_MOBILE",
];

static BUILD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:buildNumber|build_number|BUILD_NUMBER)["\s:=]+["\s]*(\d{5,7})"#)
        .expect("valid regex")
});

static BUILD_CACHE: Lazy<parking_lot::Mutex<(u64, f64)>> =
    Lazy::new(|| parking_lot::Mutex::new((FALLBACK_BUILD, 0.0)));

fn unix_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

// ==========================================
// Token validation
// ==========================================

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
}

impl UserInfo {
    pub fn display(&self) -> String {
        if let Some(gn) = &self.global_name {
            if !gn.is_empty() { return gn.clone(); }
        }
        self.username.clone()
    }
}

pub async fn validate_token(token: &str) -> Result<UserInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Lỗi HTTP: {e}"))?;

    let resp = client.get(format!("{}/users/@me", API_BASE)).header("Authorization", token).send().await.map_err(|e| format!("Không kết nối được Discord: {e}"))?;
    let status = resp.status().as_u16();
    
    if status == 401 { return Err("Token không hợp lệ hoặc đã hết hạn.".into()); }
    if status == 403 { return Err("Bị Discord chặn (403). Thử tắt VPN hoặc đợi vài phút.".into()); }
    if status == 429 { return Err("Bị rate limit (429). Đợi 5-10 phút rồi thử lại.".into()); }
    if status != 200 { return Err(format!("Discord trả mã {status}.")); }

    let data: Value = resp.json().await.map_err(|e| format!("Không đọc được response: {e}"))?;
    Ok(UserInfo {
        id: data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        username: data.get("username").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        global_name: data.get("global_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
        avatar: data.get("avatar").and_then(|v| v.as_str()).map(|s| s.to_string()),
    })
}

// ==========================================
// Build number
// ==========================================

async fn fetch_latest_build_number() -> u64 {
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";
    let client = match reqwest::Client::builder().user_agent(ua).build() { Ok(c) => c, Err(_) => return FALLBACK_BUILD };
    let html = match client.get("https://discord.com/app").timeout(Duration::from_secs(15)).send().await {
        Ok(r) if r.status().is_success() => match r.text().await { Ok(t) => t, Err(_) => return FALLBACK_BUILD },
        _ => return FALLBACK_BUILD,
    };
    if let Some(m) = BUILD_RE.captures(&html) { if let Ok(n) = m[1].parse() { return n; } }
    FALLBACK_BUILD
}

async fn get_build_number() -> u64 {
    let now = unix_f64();
    { let cache = BUILD_CACHE.lock(); if cache.1 > 0.0 && now - cache.1 < 21600.0 { return cache.0; } }
    let v = fetch_latest_build_number().await;
    { let mut cache = BUILD_CACHE.lock(); cache.0 = v; cache.1 = now; }
    v
}

#[doc(hidden)]
pub async fn _get_build_number_public() -> u64 { get_build_number().await }

// ==========================================
// Quest JSON helpers
// ==========================================

pub fn _get<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let obj = v.as_object()?;
    for k in keys { if let Some(val) = obj.get(*k) { return Some(val); } }
    None
}

pub fn user_status(q: &Value) -> Option<&Value> { _get(q, &["userStatus", "user_status"]) }

pub fn task_config(q: &Value) -> Option<&Value> {
    q.get("config").and_then(|c| _get(c, &["taskConfig", "task_config", "taskConfigV2", "task_config_v2"]))
}

pub fn quest_name(q: &Value) -> String {
    let cfg = q.get("config");
    let msgs = cfg.and_then(|c| c.get("messages"));
    let app = cfg.and_then(|c| c.get("application"));
    let name = msgs.and_then(|m| _get(m, &["questName", "quest_name"]))
        .or_else(|| msgs.and_then(|m| _get(m, &["gameTitle", "game_title"])))
        .or_else(|| app.and_then(|a| a.get("name")))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()); // Fix: fallback correctly if empty
    name.unwrap_or_else(|| format!("Quest#{}", q.get("id").and_then(|v| v.as_str()).unwrap_or("?")))
}

pub fn is_valid_task_config(t: &Value) -> bool {
    let val = match t.get("target") { Some(v) => v, None => return false };
    if val.is_boolean() { return false; }
    matches!(val.as_f64(), Some(f) if f.is_finite() && f >= 1.0)
}

pub fn is_completable(q: &Value) -> bool {
    if let Some(exp) = q.get("config").and_then(|c| _get(c, &["expiresAt", "expires_at"])).and_then(|v| v.as_str()) {
        match DateTime::parse_from_rfc3339(exp) {
            Ok(dt) if dt.with_timezone(&Utc) > Utc::now() => {}
            _ => return false,
        }
    }
    let tasks = match task_config(q).and_then(|c| c.get("tasks")) { Some(t) if t.is_object() => t, _ => return false };
    SUPPORTED_TASKS.iter().any(|t| tasks.get(*t).map(is_valid_task_config).unwrap_or(false))
}

pub fn is_enrolled(q: &Value) -> bool {
    user_status(q).and_then(|u| _get(u, &["enrolledAt", "enrolled_at"])).map(|v| !v.is_null()).unwrap_or(false)
}

pub fn is_completed(q: &Value) -> bool {
    user_status(q).and_then(|u| _get(u, &["completedAt", "completed_at"])).map(|v| !v.is_null()).unwrap_or(false)
}

pub fn get_task_type(q: &Value) -> Option<String> {
    let tasks = task_config(q)?.get("tasks")?;
    SUPPORTED_TASKS.iter().find(|t| tasks.get(**t).map(is_valid_task_config).unwrap_or(false)).map(|s| s.to_string())
}

pub fn get_seconds_needed(q: &Value) -> i64 {
    let t = match get_task_type(q) { Some(t) => t, None => return 0 };
    let task = match task_config(q).and_then(|c| c.get("tasks")).and_then(|tasks| tasks.get(&t)) {
        Some(v) if is_valid_task_config(v) => v, _ => return 0,
    };
    task.get("target").and_then(|v| v.as_f64()).filter(|f| f.is_finite() && *f >= 1.0).map(|f| f as i64).unwrap_or(0)
}

pub fn get_seconds_done(q: &Value, needed: i64) -> f64 {
    let t = match get_task_type(q) { Some(t) => t, None => return 0.0 };
    let entry = match user_status(q).and_then(|u| u.get("progress")).and_then(|p| p.get(&t)) {
        Some(e) if e.is_object() => e, _ => return 0.0,
    };
    entry.get("value").and_then(|v| v.as_f64()).filter(|f| f.is_finite() && *f >= 0.0).map(|f| f.min(needed as f64)).unwrap_or(0.0)
}

// ==========================================
// DiscordApi
// ==========================================

struct DiscordApi { http: reqwest::Client }

impl DiscordApi {
    async fn new(token: &str) -> Result<Self> {
        let build = get_build_number().await;
        let super_props = make_super_properties(build);

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_str(token)?);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
        headers.insert("X-Super-Properties", HeaderValue::from_str(&super_props)?);
        headers.insert("X-Discord-Locale", HeaderValue::from_static("en-US"));
        headers.insert("Origin", HeaderValue::from_static("https://discord.com"));

        let http = reqwest::Client::builder().default_headers(headers).timeout(Duration::from_secs(20)).build()?;
        Ok(Self { http })
    }

    async fn request(&self, method: reqwest::Method, path: &str, payload: Option<Value>, cancel: &CancellationToken) -> (u16, Option<Value>) {
        let url = format!("{}{}", API_BASE, path);
        
        for attempt in 0..3u32 {
            if cancel.is_cancelled() { return (0, None); }
            let mut req = self.http.request(method.clone(), &url);
            if let Some(p) = &payload { req = req.json(p); }

            let send_fut = req.send();
            let res = tokio::select! {
                _ = cancel.cancelled() => return (0, None),
                r = send_fut => r
            };

            match res {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let body: Value = resp.json().await.unwrap_or(Value::Null);

                    // Trả về 429 nếu hết lượt thử
                    if status == 429 {
                        let retry_after = body.get("retry_after").and_then(|v| v.as_f64()).unwrap_or(5.0);
                        if attempt < 2 {
                            let wait = retry_after + rand::random::<f64>() + 0.5;
                            tokio::select! {
                                _ = cancel.cancelled() => return (0, None),
                                _ = sleep(Duration::from_secs_f64(wait)) => {}
                            }
                            continue;
                        } else {
                            return (429, Some(body));
                        }
                    }

                    if matches!(status, 500..=504) && method == reqwest::Method::POST {
                        return (status, Some(body)); // Không retry POST để tránh duplicate
                    } else if matches!(status, 500..=504) && attempt < 2 {
                        tokio::select! { _ = cancel.cancelled() => return (0, None), _ = sleep(Duration::from_secs(2)) => {} }
                        continue;
                    }
                    return (status, Some(body));
                }
                Err(_) => {
                    if method == reqwest::Method::POST { return (0, None); }
                    if attempt < 2 {
                        tokio::select! { _ = cancel.cancelled() => return (0, None), _ = sleep(Duration::from_secs(2)) => {} }
                    }
                }
            }
        }
        (0, None)
    }

    async fn get(&self, path: &str, cancel: &CancellationToken) -> (u16, Option<Value>) { self.request(reqwest::Method::GET, path, None, cancel).await }
    async fn post(&self, path: &str, payload: Option<Value>, cancel: &CancellationToken) -> (u16, Option<Value>) { self.request(reqwest::Method::POST, path, payload, cancel).await }
}

fn make_super_properties(build_number: u64) -> String {
    let obj = json!({
        "os": "Windows", "browser": "Discord Client", "release_channel": "stable",
        "client_version": "1.0.9175", "os_arch": "x64", "app_arch": "x64", "system_locale": "en-US",
        "browser_user_agent": USER_AGENT, "browser_version": "32.2.7",
        "client_build_number": build_number, "native_build_number": 59498, "client_event_source": null,
    });
    base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&obj).unwrap())
}

// ==========================================
// Public: run quest session
// ==========================================

pub async fn run_quest_session(token: String, tx: UnboundedSender<ProgressEvent>, cancel: CancellationToken) {
    let emit = |e: ProgressEvent| { let _ = tx.send(e); };
    emit(ProgressEvent::Log("→ Đang khởi động phiên cày quest...".into()));

    let api = match DiscordApi::new(&token).await {
        Ok(a) => a,
        Err(e) => { emit(ProgressEvent::QuestAllDone { completed: 0, failed: 0, reason: format!("Không khởi tạo được API client: {e}") }); return; }
    };

    emit(ProgressEvent::Log("→ Đang quét quest...".into()));
    
    // Giữ lại stats khi văng lỗi hoặc cancel
    match run_quest_inner(&api, &tx, &cancel).await {
        Ok(_) => {
            // Hàm inner đã emit AllDone khi hoàn thành
        }
        Err((completed, failed, reason)) => {
            emit(ProgressEvent::QuestAllDone { completed, failed, reason });
        }
    }
}

async fn run_quest_inner(api: &DiscordApi, tx: &UnboundedSender<ProgressEvent>, cancel: &CancellationToken) -> Result<(), (usize, usize, String)> {
    let mut attempts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut total_completed = 0usize;
    let mut total_failed = 0usize;
    let mut worker_fails = 0u32;

    // Tránh lặp vô hạn khi enroll fail
    let mut enroll_fails: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    loop {
        if cancel.is_cancelled() { return Err((total_completed, total_failed, "Đã huỷ bởi người dùng.".into())); }

        let (status, data) = api.get("/quests/@me", cancel).await;
        if status == 401 { return Err((total_completed, total_failed, "Token hết hạn hoặc bị từ chối.".into())); }
        if status != 200 {
            worker_fails += 1;
            if worker_fails >= MAX_FAILS { return Err((total_completed, total_failed, format!("Lỗi lấy quest liên tục (HTTP {status})."))); }
            let _ = tx.send(ProgressEvent::Log(format!("⚠ Lấy quest lỗi HTTP {status}, thử lại sau 5s...")));
            tokio::select! { _ = cancel.cancelled() => return Err((total_completed, total_failed, "Đã huỷ.".into())), _ = sleep(Duration::from_secs(5)) => {} }
            continue;
        }

        let data = data.unwrap_or(Value::Null);
        let arr = if data.is_object() { data.get("quests").cloned().unwrap_or(Value::Array(vec![])) } else { data };
        let mut quests: Vec<Value> = Vec::new();
        
        if let Some(arr) = arr.as_array() {
            for q in arr {
                // Chấp nhận cả ID dạng số
                let id_val = q.get("id");
                let id_str = match id_val {
                    Some(v) if v.is_string() => v.as_str().unwrap().trim().to_string(),
                    Some(v) if v.is_number() => v.to_string(),
                    _ => continue,
                };
                if id_str.parse::<u64>().is_ok() {
                    let mut q = q.clone();
                    q.as_object_mut().unwrap().insert("id".into(), Value::String(id_str));
                    quests.push(q);
                }
            }
        }

        let pending: Vec<&Value> = quests.iter().filter(|q| {
            !is_enrolled(q) && !is_completable(q) == false && !is_completed(q) // is_completable == true
        }).collect();

        if !pending.is_empty() {
            let mut enrolled_any = false;
            for q in &pending {
                if cancel.is_cancelled() { return Err((total_completed, total_failed, "Đã huỷ.".into())); }
                let qid = q.get("id").and_then(|v| v.as_str()).unwrap_or("");
                
                // Nếu quest này đã fail enroll 3 lần, bỏ qua
                if *enroll_fails.get(qid).unwrap_or(&0) >= MAX_ATTEMPTS { continue; }

                let _ = tx.send(ProgressEvent::Log(format!("→ Đang nhận quest: {}", quest_name(q))));
                if enroll_quest(api, q, cancel).await {
                    enrolled_any = true;
                } else {
                    *enroll_fails.entry(qid.to_string()).or_insert(0) += 1;
                }
                tokio::select! { _ = cancel.cancelled() => return Err((total_completed, total_failed, "Đã huỷ.".into())), _ = sleep(Duration::from_secs(3)) => {} }
            }
            if enrolled_any { tokio::select! { _ = cancel.cancelled() => return Err((0,0,"".into())), _ = sleep(Duration::from_secs(2)) => {} }; }
            continue;
        }

        let actionable: Vec<Value> = quests.iter().filter(|q| {
            is_enrolled(q) && !is_completed(q) && is_completable(q)
        }).filter(|q| {
            let qid = q.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            attempts.get(&qid).copied().unwrap_or(0) < MAX_ATTEMPTS
        }).cloned().collect();

        if actionable.is_empty() {
            let reason = if total_completed > 0 { format!("Đã cày xong {} quest!", total_completed) } else { "Không tìm thấy quest nào cần cày.".into() };
            let _ = tx.send(ProgressEvent::QuestAllDone { completed: total_completed, failed: total_failed, reason });
            return Ok(());
        }

        for q in &actionable {
            if cancel.is_cancelled() { return Err((total_completed, total_failed, "Đã huỷ.".into())); }
            let qid = q.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let name = quest_name(q);
            *attempts.entry(qid).or_insert(0) += 1;

            match process_quest(api, q, tx, cancel).await {
                Ok(()) => {
                    total_completed += 1;
                    let _ = tx.send(ProgressEvent::QuestCompleted { name, success: true, reason: None });
                }
                Err(QuestError::Abort(e)) => {
                    let _ = tx.send(ProgressEvent::QuestCompleted { name, success: false, reason: Some(e.clone()) });
                    return Err((total_completed, total_failed, format!("Dừng vì lỗi: {e}")));
                }
                Err(QuestError::Fail(e)) => {
                    total_failed += 1;
                    let _ = tx.send(ProgressEvent::QuestCompleted { name, success: false, reason: Some(e) });
                }
            }
        }

        tokio::select! { _ = cancel.cancelled() => return Err((total_completed, total_failed, "Đã huỷ.".into())), _ = sleep(Duration::from_secs(3)) => {} }
        worker_fails = 0;
    }
}

#[derive(Debug)]
enum QuestError { Abort(String), Fail(String) }

async fn enroll_quest(api: &DiscordApi, quest: &Value, cancel: &CancellationToken) -> bool {
    let qid = match quest.get("id").and_then(|v| v.as_str()) { Some(s) => s, None => return false };
    let payload = json!({ "location": 11, "is_targeted": false, "metadata_raw": null, "metadata_sealed": null });
    let (status, _) = api.post(&format!("/quests/{}/enroll", qid), Some(payload), cancel).await;
    matches!(status, 200 | 201 | 204)
}

async fn process_quest(api: &DiscordApi, quest: &Value, tx: &UnboundedSender<ProgressEvent>, cancel: &CancellationToken) -> Result<(), QuestError> {
    let task_type = match get_task_type(quest) { Some(t) => t, None => return Err(QuestError::Fail("Không xác định được task.".into())) };
    let name = quest_name(quest);
    let needed = get_seconds_needed(quest) as f64;
    let done = get_seconds_done(quest, needed as i64);

    let _ = tx.send(ProgressEvent::Log(format!("→ Cày quest: {} ({} · {:.0}/{:.0}s)", name, task_type, done, needed)));

    if task_type == "WATCH_VIDEO" || task_type == "WATCH_VIDEO_ON_MOBILE" {
        complete_video(api, quest, &name, &task_type, tx, cancel).await
    } else {
        complete_heartbeat(api, quest, &name, &task_type, tx, cancel).await
    }
}

async fn complete_video(api: &DiscordApi, quest: &Value, name: &str, task_type: &str, tx: &UnboundedSender<ProgressEvent>, cancel: &CancellationToken) -> Result<(), QuestError> {
    let qid = quest.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let sec_needed = get_seconds_needed(quest) as f64;
    let mut sec_done = get_seconds_done(quest, sec_needed as i64);
    let enrolled_ts = unix_f64(); // Simplified for brevity
    let speed = 7.0;
    let max_future = 10.0;
    let mut fails = 0u32;
    let mut last_emit = Instant::now() - Duration::from_secs(10);

    while sec_done < sec_needed {
        if cancel.is_cancelled() { return Err(QuestError::Abort("Đã huỷ.".into())); }
        let max_allowed = (unix_f64() - enrolled_ts) + max_future;
        let diff = max_allowed - sec_done;
        let timestamp = sec_done + speed;

        if diff >= speed {
            // Cộng random trước khi min
            let payload = json!({ "timestamp": (timestamp + rand::random::<f64>()).min(sec_needed) });
            let (status, body) = api.post(&format!("/quests/{}/video-progress", qid), Some(payload), cancel).await;
            
            if status == 200 {
                fails = 0;
                if body.as_ref().and_then(|b| b.get("completed_at")).map(|v| !v.is_null()).unwrap_or(false) { return Ok(()); }
                sec_done = timestamp.min(sec_needed);
                if last_emit.elapsed().as_millis() > 500 {
                    let _ = tx.send(ProgressEvent::QuestUpdate { name: name.to_string(), task_type: task_type.to_string(), sec_done, sec_needed });
                    last_emit = Instant::now();
                }
            } else {
                fails += 1;
                if fails >= MAX_FAILS { return Err(QuestError::Fail(format!("Lỗi liên tục (HTTP {}).", status))); }
                tokio::select! { _ = cancel.cancelled() => return Err(QuestError::Abort("Đã huỷ.".into())), _ = sleep(Duration::from_secs(2)) => {} }
            }
        }
        if timestamp >= sec_needed { break; }
        tokio::select! { _ = cancel.cancelled() => return Err(QuestError::Abort("Đã huỷ.".into())), _ = sleep(Duration::from_secs(1)) => {} }
    }

    let (status, body) = api.post(&format!("/quests/{}/video-progress", qid), Some(json!({"timestamp": sec_needed})), cancel).await;
    if status == 200 && body.as_ref().and_then(|b| b.get("completed_at")).map(|v| !v.is_null()).unwrap_or(false) { return Ok(()); }
    
    // Xác nhận lại trạng thái quest
    tokio::select! { _ = cancel.cancelled() => return Err(QuestError::Abort("Đã huỷ.".into())), _ = sleep(Duration::from_secs(2)) => {} }
    if verify_completed(api, &qid, cancel).await { return Ok(()); }
    Err(QuestError::Fail("Gửi xong nhưng Discord chưa báo hoàn thành.".into()))
}

async fn complete_heartbeat(api: &DiscordApi, quest: &Value, name: &str, task_type: &str, tx: &UnboundedSender<ProgressEvent>, cancel: &CancellationToken) -> Result<(), QuestError> {
    let qid = quest.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let sec_needed = get_seconds_needed(quest) as f64;
    let mut sec_done = get_seconds_done(quest, sec_needed as i64);
    let stream_key = if task_type == "PLAY_ACTIVITY" { "call:0:1".to_string() } else { format!("call:0:{}", rand::random::<u32>() % 29000 + 1000) };
    let mut fails = 0u32;

    while sec_done < sec_needed {
        if cancel.is_cancelled() { return Err(QuestError::Abort("Đã huỷ.".into())); }
        let payload = json!({"stream_key": stream_key, "terminal": false});
        let (status, body) = api.post(&format!("/quests/{}/heartbeat", qid), Some(payload), cancel).await;

        if status == 200 {
            fails = 0;
            if let Some(progress) = body.as_ref().and_then(|b| b.get("progress")) {
                if let Some(v) = progress.get(task_type).and_then(|e| e.get("value")).and_then(|v| v.as_f64()) {
                    if v.is_finite() && v >= 0.0 {
                        sec_done = v.min(sec_needed);
                        let _ = tx.send(ProgressEvent::QuestUpdate { name: name.to_string(), task_type: task_type.to_string(), sec_done, sec_needed });
                    }
                }
            }
            if body.as_ref().and_then(|b| b.get("completed_at")).map(|v| !v.is_null()).unwrap_or(false) || sec_done >= sec_needed { break; }
        } else {
            fails += 1;
            if fails >= MAX_FAILS { return Err(QuestError::Fail(format!("Lỗi liên tục (HTTP {}).", status))); }
        }
        // Có thể ngắt lệnh sleep 20s ngay lập tức
        tokio::select! { _ = cancel.cancelled() => return Err(QuestError::Abort("Đã huỷ.".into())), _ = sleep(Duration::from_secs(HEARTBEAT_INTERVAL)) => {} }
    }

    let (status, body) = api.post(&format!("/quests/{}/heartbeat", qid), Some(json!({"stream_key": stream_key, "terminal": true})), cancel).await;
    if status == 200 && body.as_ref().and_then(|b| b.get("completed_at")).map(|v| !v.is_null()).unwrap_or(false) { return Ok(()); }
    
    // Xác nhận lại trạng thái quest
    tokio::select! { _ = cancel.cancelled() => return Err(QuestError::Abort("Đã huỷ.".into())), _ = sleep(Duration::from_secs(2)) => {} }
    if verify_completed(api, &qid, cancel).await { return Ok(()); }
    Err(QuestError::Fail("Heartbeat xong nhưng Discord chưa báo hoàn thành.".into()))
}

async fn verify_completed(api: &DiscordApi, qid: &str, cancel: &CancellationToken) -> bool {
    let (status, data) = api.get("/quests/@me", cancel).await;
    if status == 200 {
        if let Some(arr) = data.and_then(|d| d.get("quests").cloned()).and_then(|v| v.as_array().cloned()) {
            if let Some(q) = arr.iter().find(|q| q.get("id").and_then(|v| v.as_str()) == Some(qid)) {
                return is_completed(q);
            }
        }
    }
    false
}
