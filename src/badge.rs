use base64::Engine;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::events::ProgressEvent;
use crate::quest::validate_token;

// ==========================================
// Constants
// ==========================================

const SCIENCE_URL: &str = "https://discord.com/api/v9/science";
const ME_URL: &str = "https://discord.com/api/v9/users/@me?with_analytics_token=true";
const GAMES_CDN_URL: &str = "https://cdn.discordapp.com/detectables/games.json";
const BATCH_SIZE: usize = 50;
const BATCH_DELAY_SECS: u64 = 1;
const GAMES_TTL_SECS: u64 = 24 * 3600;
const MAX_GAMES: usize = 200;

const USER_AGENT_BADGE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) discord/1.0.9253 Chrome/148.0.7778.280 Electron/42.7.1 Safari/537.36";

fn unix_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ==========================================
// Milestones (cho GUI dropdown)
// ==========================================

pub struct TimeMilestone {
    pub key: &'static str,
    pub name: &'static str,
    pub hours: u32,
}

pub struct GameMilestone {
    pub key: &'static str,
    pub name: &'static str,
    pub games: u32,
}

pub const TIME_MILESTONES: &[TimeMilestone] = &[
    TimeMilestone { key: "1", name: "Casual", hours: 1 },
    TimeMilestone { key: "2", name: "Recreational", hours: 5 },
    TimeMilestone { key: "3", name: "Dedicated", hours: 20 },
    TimeMilestone { key: "4", name: "Committed", hours: 75 },
    TimeMilestone { key: "5", name: "Serious", hours: 150 },
    TimeMilestone { key: "6", name: "Devoted", hours: 300 },
    TimeMilestone { key: "7", name: "Seasoned", hours: 500 },
    TimeMilestone { key: "8", name: "Ironclad", hours: 1000 },
    TimeMilestone { key: "9", name: "Unshakeable", hours: 2000 },
    TimeMilestone { key: "10", name: "Eternal", hours: 5000 },
];

pub const GAME_MILESTONES: &[GameMilestone] = &[
    GameMilestone { key: "1", name: "Sampler", games: 2 },
    GameMilestone { key: "2", name: "Dabbler", games: 5 },
    GameMilestone { key: "3", name: "Enthusiast", games: 10 },
    GameMilestone { key: "4", name: "Ranger", games: 15 },
    GameMilestone { key: "5", name: "Explorer", games: 20 },
    GameMilestone { key: "6", name: "Adventurer", games: 30 },
    GameMilestone { key: "7", name: "Voyager", games: 40 },
    GameMilestone { key: "8", name: "Maverick", games: 60 },
    GameMilestone { key: "9", name: "Polymath", games: 80 },
    GameMilestone { key: "10", name: "Universalist", games: 100 },
];

pub fn find_time(key: &str) -> Option<&'static TimeMilestone> {
    TIME_MILESTONES.iter().find(|m| m.key == key)
}

pub fn find_games(key: &str) -> Option<&'static GameMilestone> {
    GAME_MILESTONES.iter().find(|m| m.key == key)
}

pub fn is_likely_fingerprint(t: &str) -> bool {
    if t.is_empty() {
        return true;
    }
    let t = t.trim();
    if !(100..=300).contains(&t.len()) {
        return false;
    }
    t.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

// ==========================================
// Games
// ==========================================

#[derive(Clone, Debug)]
pub struct SpooferGame {
    pub id: String,
    pub name: String,
    pub exe: String,
}

struct GamesCache {
    games: Vec<SpooferGame>,
    at: f64,
}

static CACHE: Lazy<Arc<Mutex<Option<GamesCache>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

fn games_cache_path() -> PathBuf {
    crate::config::AppConfig::dir().join("games.json")
}

fn parse_games(raw: &str) -> Vec<SpooferGame> {
    let data: Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = match data.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for entry in arr {
        let obj = match entry.as_object() {
            Some(o) => o,
            None => continue,
        };
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .trim()
            .to_string();
        let name = if name.is_empty() {
            "Unknown".to_string()
        } else {
            name
        };

        let gid = obj
            .get("id")
            .and_then(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .or_else(|| v.as_u64().map(|n| n.to_string()))
            })
            .unwrap_or_default();
        if gid.is_empty() || !gid.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if seen.contains(&gid) {
            continue;
        }

        let exes = match obj.get("executables").and_then(|v| v.as_array()) {
            Some(e) => e,
            None => continue,
        };

        let mut exe: Option<String> = None;
        let mut has_win = false;
        for e in exes {
            if let Some(eo) = e.as_object() {
                if eo.get("os").and_then(|v| v.as_str()) == Some("win32") {
                    has_win = true;
                    if let Some(n) = eo.get("name").and_then(|v| v.as_str()) {
                        exe = Some(n.to_string());
                        break;
                    }
                }
            }
        }
        if !has_win {
            continue;
        }

        seen.insert(gid.clone());
        out.push(SpooferGame {
            id: gid,
            name,
            exe: exe.unwrap_or_else(|| "game.exe".into()),
        });
    }
    out
}

async fn load_games(tx: &UnboundedSender<ProgressEvent>) -> Vec<SpooferGame> {
    let now = unix_f64();

    {
        let cache = CACHE.lock().await;
        if let Some(c) = cache.as_ref() {
            if now - c.at < GAMES_TTL_SECS as f64 {
                return c.games.clone();
            }
        }
    }

    let cp = games_cache_path();
    if cp.exists() {
        if let Ok(raw) = tokio::fs::read_to_string(&cp).await {
            let games = parse_games(&raw);
            if !games.is_empty() {
                *CACHE.lock().await = Some(GamesCache {
                    games: games.clone(),
                    at: now,
                });
                return games;
            }
        }
        let _ = tokio::fs::remove_file(&cp).await;
    }

    let _ = tx.send(ProgressEvent::Log(
        "→ Đang tải danh sách game từ Discord CDN...".into(),
    ));

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    if let Ok(resp) = client.get(GAMES_CDN_URL).send().await {
        if resp.status().is_success() {
            if let Ok(raw) = resp.text().await {
                let games = parse_games(&raw);
                if !games.is_empty() {
                    if let Some(parent) = cp.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    let _ = tokio::fs::write(&cp, &raw).await;
                    *CACHE.lock().await = Some(GamesCache {
                        games: games.clone(),
                        at: now,
                    });
                    return games;
                }
            }
        }
    }
    Vec::new()
}

// ==========================================
// Super props + analytics
// ==========================================

fn build_super_props(
    build_number: u64,
    launch_signature: &str,
    heartbeat_session: &str,
) -> String {
    let obj = json!({
        "os": "Windows", "browser": "Discord Client", "release_channel": "stable",
        "client_version": "1.0.9253", "os_version": "10.0.26200", "os_arch": "x64", "app_arch": "x64",
        "system_locale": "vi-VN",
        "browser_user_agent": USER_AGENT_BADGE, "browser_version": "42.7.1",
        "client_build_number": build_number, "native_build_number": 88414,
        "client_event_source": null,
        "client_launch_id": uuid::Uuid::new_v4().to_string(),
        "launch_signature": launch_signature,
        "client_heartbeat_session_id": heartbeat_session,
    });
    base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&obj).unwrap())
}

async fn fetch_analytics_token(token: &str, super_props: &str) -> Result<String, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "authorization",
        reqwest::header::HeaderValue::from_str(token).map_err(|e| e.to_string())?,
    );
    headers.insert(
        "user-agent",
        reqwest::header::HeaderValue::from_static(USER_AGENT_BADGE),
    );
    headers.insert(
        "x-super-properties",
        reqwest::header::HeaderValue::from_str(super_props).map_err(|e| e.to_string())?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(ME_URL).send().await.map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    match status {
        401 => Err("Token bị từ chối (401).".into()),
        403 => Err("Bị Cloudflare chặn (403).".into()),
        429 => Err("Rate limit (429). Đợi 5-10 phút.".into()),
        200 => {
            let data: Value = resp.json().await.map_err(|e| e.to_string())?;
            data.get("analytics_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "Discord không trả analytics_token.".into())
        }
        s => Err(format!("Lấy analytics token lỗi (mã {s}).")),
    }
}

// ==========================================
// ScienceSpoofer
// ==========================================

struct ScienceSpoofer {
    analytics_token: String,
    heartbeat_session: String,
    launch_signature: String,
    fingerprint: String,
    seq: std::sync::atomic::AtomicU64,
    client: reqwest::Client,
}

impl ScienceSpoofer {
    fn new(
        token: &str,
        analytics_token: String,
        super_props: String,
        heartbeat_session: String,
        launch_signature: String,
        fingerprint: String,
    ) -> Result<Self, String> {
        use reqwest::header::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("*/*"));
        headers.insert(
            "accept-language",
            HeaderValue::from_static("vi-VN,vi;q=0.9,en;q=0.8"),
        );
        headers.insert(
            "authorization",
            HeaderValue::from_str(token).map_err(|e| e.to_string())?,
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("origin", HeaderValue::from_static("https://discord.com"));
        headers.insert(
            "referer",
            HeaderValue::from_static("https://discord.com/channels/@me"),
        );
        headers.insert("user-agent", HeaderValue::from_static(USER_AGENT_BADGE));
        headers.insert(
            "x-debug-options",
            HeaderValue::from_static("bugReporterEnabled"),
        );
        headers.insert("x-discord-locale", HeaderValue::from_static("vi-VN"));
        headers.insert(
            "x-discord-timezone",
            HeaderValue::from_static("Asia/Ho_Chi_Minh"),
        );
        headers.insert(
            "x-super-properties",
            HeaderValue::from_str(&super_props).map_err(|e| e.to_string())?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            analytics_token,
            heartbeat_session,
            launch_signature,
            fingerprint,
            seq: std::sync::atomic::AtomicU64::new(0),
            client,
        })
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    }

    fn build_launch(&self, game: &SpooferGame) -> Value {
        let now = now_ms();
        let mut props = json!({
            "client_track_timestamp": now,
            "client_heartbeat_session_id": self.heartbeat_session,
            "event_sequence_number": self.next_seq(),
            "game": game.name, "game_id": game.id,
            "verified": true, "elevated": false, "is_launcher": false,
            "game_platform": "desktop", "detection_method": "verified_game",
            "is_overlay_enabled": false, "is_overlay_game_enabled": true,
            "is_overlay_game_source": "OOP_DEFAULT_DATABASE",
            "fullscreen_type": "UNKNOWN", "hardware_display_count": 1,
            "overlay_method": "Disabled", "activity_status_enabled": true,
            "activity_status_shared_guilds": [], "current_user_status": "online",
            "game_detection_enabled": true, "executable_path": game.exe,
            "voice_channel_id": null, "voice_channel_type": null,
            "voice_channel_bitrate": null, "voice_channel_guild_id": null,
            "hidden_by_distributor": false, "game_metadata": null,
            "client_performance_cpu": null, "client_performance_memory": null,
            "cpu_core_count": null, "accessibility_features": 0,
            "rendered_locale": "vi-VN",
            "launch_signature": self.launch_signature,
            "client_rtc_state": null, "client_app_state": "focused",
            "client_send_timestamp": now,
        });
        if !self.fingerprint.is_empty() {
            props["executable_fingerprint"] = json!(self.fingerprint);
        }
        json!({"type": "launch_game", "properties": props})
    }

    fn build_heartbeat(
        &self,
        game: &SpooferGame,
        duration_ms: u64,
        session_id: &str,
        initial: bool,
        final_: bool,
        ts: u64,
    ) -> Value {
        json!({"type": "running_game_heartbeat", "properties": {
            "client_track_timestamp": ts,
            "client_heartbeat_session_id": self.heartbeat_session,
            "event_sequence_number": self.next_seq(),
            "game_id": game.id, "game_name": game.name,
            "game_metadata": null, "game_executable": game.exe,
            "game_detection_enabled": true,
            "initial_heartbeat": initial, "final_heartbeat": final_,
            "game_session_id": session_id, "duration_tracked_ms": duration_ms,
            "rtc_connection_id": null, "media_session_id": null,
            "launch_signature": self.launch_signature,
            "client_app_state": "focused", "client_send_timestamp": ts,
        }})
    }

    fn build_events(&self, game: &SpooferGame, duration_ms: u64) -> Vec<Value> {
        let sid = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let start = now.saturating_sub(duration_ms);
        vec![
            self.build_heartbeat(game, 0, &sid, true, false, start),
            self.build_launch(game),
            self.build_heartbeat(game, duration_ms, &sid, false, true, now),
        ]
    }

    async fn post(&self, events: Vec<Value>) -> u16 {
        let payload = json!({"token": self.analytics_token, "events": events});
        match self.client.post(SCIENCE_URL).json(&payload).send().await {
            Ok(r) => {
                let status = r.status().as_u16();
                if status == 429 {
                    let wait = r
                        .json::<Value>()
                        .await
                        .ok()
                        .and_then(|v| v.get("retry_after").and_then(|v| v.as_f64()))
                        .unwrap_or(5.0);
                    sleep(Duration::from_secs_f64(wait + 1.0)).await;
                }
                status
            }
            Err(_) => 0,
        }
    }
}

// ==========================================
// Public: run badge session
// ==========================================

pub async fn run_badge_session(
    token: String,
    fingerprint: String,
    hours_per_game: f64,
    games_count: usize,
    tx: UnboundedSender<ProgressEvent>,
    cancel: CancellationToken,
) {
    let emit = |e: ProgressEvent| {
        let _ = tx.send(e);
    };

    emit(ProgressEvent::Log("→ Đang xác thực token...".into()));

    if let Err(e) = validate_token(&token).await {
        emit(ProgressEvent::BadgeDone {
            success: false,
            done: 0,
            total: games_count,
            reason: e,
        });
        return;
    }

    match run_badge_inner(&token, &fingerprint, hours_per_game, games_count, &tx, &cancel).await {
        Ok(done) => emit(ProgressEvent::BadgeDone {
            success: true,
            done,
            total: games_count,
            reason: "Hoàn tất".into(),
        }),
        Err(e) => emit(ProgressEvent::BadgeDone {
            success: false,
            done: 0,
            total: games_count,
            reason: e,
        }),
    }
}

async fn run_badge_inner(
    token: &str,
    fingerprint: &str,
    hours_per_game: f64,
    games_count: usize,
    tx: &UnboundedSender<ProgressEvent>,
    cancel: &CancellationToken,
) -> Result<usize, String> {
    let emit = |e: ProgressEvent| {
        let _ = tx.send(e);
    };

    // Build number
    emit(ProgressEvent::Log("→ Đang lấy build number...".into()));
    let build = crate::quest::_get_build_number_public().await;

    let hb = uuid::Uuid::new_v4().to_string();
    let sig = uuid::Uuid::new_v4().to_string();
    let super_props = build_super_props(build, &sig, &hb);

    emit(ProgressEvent::Log("→ Đang lấy analytics token...".into()));
    let analytics = fetch_analytics_token(token, &super_props).await?;

    if cancel.is_cancelled() {
        return Err("Đã huỷ.".into());
    }

    let all = load_games(tx).await;
    if all.is_empty() {
        return Err("Không tải được games.json từ CDN.".into());
    }

    let count = games_count.min(all.len()).min(MAX_GAMES);
    let selected: Vec<_> = all
        .choose_multiple(&mut rand::thread_rng(), count)
        .cloned()
        .collect();

    emit(ProgressEvent::Log(format!(
        "→ Đã chọn {}/{} game, bắt đầu buff...",
        count,
        games_count
    )));

    if cancel.is_cancelled() {
        return Err("Đã huỷ.".into());
    }

    let spoofer = ScienceSpoofer::new(
        token,
        analytics,
        super_props,
        hb,
        sig,
        fingerprint.to_string(),
    )?;
    let duration_ms = (hours_per_game * 3600.0 * 1000.0) as u64;

    // Test event
    let Some(test_game) = selected.first().cloned() else {
        return Err("Không có game nào.".into());
    };
    let test_status = spoofer
        .post(spoofer.build_events(&test_game, duration_ms))
        .await;

    let mut done: usize;
    let mut failed = 0usize;

    if test_status == 204 {
        done = 1;
        let _ = tx.send(ProgressEvent::BadgeUpdate {
            done,
            failed,
            total: count,
            desc: format!("Test OK: {}", test_game.name),
        });
    } else if matches!(test_status, 401 | 403) {
        return Err(format!("Auth bị từ chối (mã {test_status})."));
    } else {
        return Err(format!("Test endpoint thất bại (mã {test_status})."));
    }

    // Batches
    for chunk in selected[1..].chunks(BATCH_SIZE) {
        if cancel.is_cancelled() {
            return Err("Đã huỷ bởi người dùng.".into());
        }

        let mut events = Vec::with_capacity(chunk.len() * 3);
        for g in chunk {
            events.extend(spoofer.build_events(g, duration_ms));
        }

        let mut success = false;
        for _ in 0..3 {
            let status = spoofer.post(events.clone()).await;
            if status == 204 {
                success = true;
                break;
            }
            if matches!(status, 401 | 403) {
                return Err(format!("Auth bị từ chối giữa chừng (mã {status})."));
            }
            sleep(Duration::from_secs(3)).await;
        }
        if success {
            done += chunk.len();
        } else {
            failed += chunk.len();
        }

        let _ = tx.send(ProgressEvent::BadgeUpdate {
            done,
            failed,
            total: count,
            desc: format!("Đang buff... {}/{}", done, count),
        });

        sleep(Duration::from_secs(BATCH_DELAY_SECS)).await;
    }

    if failed > 0 {
        Err(format!("{done}/{count} OK, {failed} thất bại."))
    } else {
        Ok(done)
    }
}
