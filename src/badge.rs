use base64::Engine;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::events::ProgressEvent;
use crate::quest::{get_build_number, random_proxy, validate_token};

const SCIENCE_URL: &str = "https://discord.com/api/v9/science";
const ME_URL: &str = "https://discord.com/api/v9/users/@me?with_analytics_token=true";
const GAMES_CDN_URL: &str = "https://cdn.discordapp.com/detectables/games.json";
const PROPERTIES_URL: &str = "https://cordapi.dolfi.es/api/v2/properties/windows";

const BATCH_SIZE: usize = 50;
const BATCH_DELAY_MS: u64 = 300;
const GAMES_TTL_SECS: u64 = 24 * 3600;
const MAX_GAMES: usize = 200;

const USER_AGENT_BADGE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) discord/1.0.9253 Chrome/148.0.7778.280 Electron/42.7.1 Safari/537.36";

fn unix_f64() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub struct TimeMilestone { pub key: &'static str, pub name: &'static str, pub hours: u32 }
pub struct GameMilestone { pub key: &'static str, pub name: &'static str, pub games: u32 }

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

pub fn find_time(key: &str) -> Option<&'static TimeMilestone> { TIME_MILESTONES.iter().find(|m| m.key == key) }
pub fn find_games(key: &str) -> Option<&'static GameMilestone> { GAME_MILESTONES.iter().find(|m| m.key == key) }

pub fn is_likely_fingerprint(t: &str) -> bool {
    if t.is_empty() { return true; }
    let t = t.trim();
    if !(100..=300).contains(&t.len()) { return false; }
    t.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

pub fn get_user_id_from_token(token: &str) -> String {
    if let Some(first) = token.split('.').next() {
        let mut padded = first.to_string();
        while padded.len() % 4 != 0 { padded.push('='); }
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(padded) {
            if let Ok(s) = String::from_utf8(decoded) { return s; }
        }
    }
    String::new()
}

pub fn generate_fingerprint(token: &str, exe_path: &str) -> Result<String, String> {
    let uid = get_user_id_from_token(token);
    if uid.is_empty() { return Err("Token không hợp lệ để lấy User ID.".into()); }

    let ps_script = format!(r#"
param(
  [string]$Path,
  [Parameter(Mandatory = $true)][string]$UserId
)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class CertPubKeyHash {{
  const uint X509_ASN_ENCODING = 0x00000001;
  const uint PKCS_7_ASN_ENCODING = 0x00010000;
  const uint CALG_SHA_256 = 0x0000800C;
  [StructLayout(LayoutKind.Sequential)] struct Blob {{ public uint cbData; public IntPtr pbData; }}
  [StructLayout(LayoutKind.Sequential)] struct AlgId {{ public IntPtr pszObjId; public Blob Parameters; }}
  [StructLayout(LayoutKind.Sequential)] struct BitBlob {{ public uint cbData; public IntPtr pbData; public uint cUnusedBits; }}
  [StructLayout(LayoutKind.Sequential)] struct PubKeyInfo {{ public AlgId Algorithm; public BitBlob PublicKey; }}
  [StructLayout(LayoutKind.Sequential)] struct FileTime {{ public uint Low; public uint High; }}
  [StructLayout(LayoutKind.Sequential)] struct CertInfo {{ public uint Version; public Blob SerialNumber; public AlgId SignatureAlgorithm; public Blob Issuer; public FileTime NotBefore; public FileTime NotAfter; public Blob Subject; public PubKeyInfo SubjectPublicKeyInfo; public BitBlob IssuerUniqueId; public BitBlob SubjectUniqueId; public uint ExtensionCount; public IntPtr Extensions; }}
  [StructLayout(LayoutKind.Sequential)] struct CertContext {{ public uint EncodingType; public IntPtr Encoded; public uint EncodedSize; public IntPtr CertInfo; public IntPtr Store; }}
  [DllImport("crypt32.dll", SetLastError = true)] static extern bool CryptHashPublicKeyInfo(IntPtr provider, uint algid, uint flags, uint encoding, ref PubKeyInfo info, byte[] hash, ref uint hashSize);
  public static byte[] Hash(IntPtr certContext) {{
    var ctx = Marshal.PtrToStructure<CertContext>(certContext);
    var info = Marshal.PtrToStructure<CertInfo>(ctx.CertInfo);
    var hash = new byte[32]; uint hashSize = 32;
    if (!CryptHashPublicKeyInfo(IntPtr.Zero, CALG_SHA_256, 0, X509_ASN_ENCODING | PKCS_7_ASN_ENCODING, ref info.SubjectPublicKeyInfo, hash, ref hashSize) || hashSize != 32)
      throw new InvalidOperationException("CryptHashPublicKeyInfo failed");
    return hash;
  }}
}}
"@
if (!$Path) {{ throw "Không tìm thấy đường dẫn executable." }}
$fs = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete)
try {{
  $fileSize = [UInt64]$fs.Length
  $readSize = [int][Math]::Min($fs.Length, 0x10000)
  $buf = [byte[]]::new($readSize)
  $read = $fs.Read($buf, 0, $readSize)
  if ($read -lt 0x40) {{ throw "Kích thước file quá nhỏ." }}
  if ($read -ne $readSize) {{ [Array]::Resize([ref]$buf, $read) }}
}} finally {{ $fs.Dispose() }}
if ([BitConverter]::ToUInt16($buf, 0) -ne 0x5A4D) {{ throw "Không phải file MZ executable." }}
$pe = [BitConverter]::ToInt32($buf, 0x3C)
if ($pe -lt 0 -or [BitConverter]::ToUInt32($buf, $pe) -ne 0x4550) {{ throw "Không phải file PE executable." }}
$fh = $pe + 4
$sections = [BitConverter]::ToUInt16($buf, $fh + 2)
$timestamp = [BitConverter]::ToUInt32($buf, $fh + 4)
$optSize = [BitConverter]::ToUInt16($buf, $fh + 16)
$opt = $fh + 20
$isPe32Plus = [BitConverter]::ToUInt16($buf, $opt) -eq 0x20B
if (!$isPe32Plus) {{ throw "Discord chỉ tạo fingerprint cho file PE32+." }}
$sizeOfImage = [BitConverter]::ToUInt32($buf, $opt + 56)
$sectionTable = $opt + $optSize
$sha256 = [Security.Cryptography.SHA256]::Create()
$headerHash = $sha256.ComputeHash([byte[]]$buf[0..($sectionTable - 1)])
$sectionHash = $sha256.ComputeHash([byte[]]$buf[$sectionTable..($sectionTable + $sections * 40 - 1)])
$machineGuid = (Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Cryptography" -Name MachineGuid).MachineGuid
$machineUserHash = $sha256.ComputeHash([Text.Encoding]::UTF8.GetBytes($machineGuid + $UserId))
$sig = Get-AuthenticodeSignature -LiteralPath $Path
$signed = $sig.Status -eq "Valid" -and $sig.SignerCertificate
if ($signed) {{
  $name = $sig.SignerCertificate.GetNameInfo([Security.Cryptography.X509Certificates.X509NameType]::SimpleName, $false)
  $certHash = $sha256.ComputeHash([Text.Encoding]::UTF8.GetBytes($name) + [CertPubKeyHash]::Hash($sig.SignerCertificate.Handle))
}} else {{
  $certHash = [byte[]]::new(32)
}}
$flags = 0
if ($signed) {{ $flags += 1 }}
if ($isPe32Plus) {{ $flags += 2 }}
$out = [byte[]](@(1, [byte]$flags) + [BitConverter]::GetBytes($fileSize) + [BitConverter]::GetBytes($timestamp) + [BitConverter]::GetBytes($sizeOfImage) + $headerHash + $machineUserHash + $sectionHash + $certHash)
$key = $sha256.ComputeHash([Text.Encoding]::UTF8.GetBytes($UserId + "https://discord.com/careers"))
for ($i = 1; $i -lt $out.Length; $i++) {{
  $out[$i] = $out[$i] -bxor $key[($i - 1) -band 31]
}}
[Convert]::ToBase64String($out)
"#);

    let script_path = std::env::temp_dir().join(format!("fp_{}.ps1", uuid::Uuid::new_v4()));
    std::fs::write(&script_path, ps_script).map_err(|e| e.to_string())?;

    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", script_path.to_str().unwrap(),
        "-Path", exe_path,
        "-UserId", &uid,
    ]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    use std::process::Stdio;
    use std::io::Read;
    use std::time::Instant;
    
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file(script_path);
            return Err(format!("Không khởi chạy được PowerShell: {}", e));
        }
    };

    let start = Instant::now();
    let timeout = Duration::from_secs(60);
    let mut output = String::new();
    let mut err_output = String::new();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                if let Some(mut out) = child.stdout.take() { let _ = out.read_to_string(&mut output); }
                if let Some(mut err) = child.stderr.take() { let _ = err.read_to_string(&mut err_output); }
                break;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&script_path);
                    return Err("Quá thời gian 60 giây khi tạo Fingerprint.".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&script_path);
                return Err(format!("Lỗi theo dõi tiến trình: {}", e));
            }
        }
    }

    let _ = std::fs::remove_file(&script_path);

    if !output.is_empty() {
        if let Some(last_line) = output.lines().last() {
            if !last_line.trim().is_empty() { return Ok(last_line.trim().to_string()); }
        }
    }
    let last_err = err_output.lines().last().unwrap_or("Lỗi không xác định").trim();
    Err(format!("Lỗi tạo Fingerprint: {}", last_err))
}

#[derive(Clone, Debug)]
pub struct SpooferGame { pub id: String, pub name: String, pub exe: String }

struct GamesCache { games: Vec<SpooferGame>, at: f64 }

static CACHE: Lazy<Arc<Mutex<Option<GamesCache>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

fn games_cache_path() -> PathBuf {
    crate::config::AppConfig::dir().join("games.json")
}

fn parse_games(raw: &str) -> Vec<SpooferGame> {
    let data: Value = match serde_json::from_str(raw) { Ok(v) => v, Err(_) => return Vec::new() };
    let arr = match data.as_array() { Some(a) => a, None => return Vec::new() };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for entry in arr {
        let obj = match entry.as_object() { Some(o) => o, None => continue };
        let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").trim().to_string();
        let name = if name.is_empty() { "Unknown".to_string() } else { name };
        let gid = obj.get("id").and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_u64().map(|n| n.to_string()))).unwrap_or_default();
        if gid.is_empty() || !gid.chars().all(|c| c.is_ascii_digit()) || seen.contains(&gid) { continue; }
        let exes = match obj.get("executables").and_then(|v| v.as_array()) { Some(e) => e, None => continue };
        let mut exe: Option<String> = None;
        let mut has_win = false;
        for e in exes {
            if let Some(eo) = e.as_object() {
                if eo.get("os").and_then(|v| v.as_str()) == Some("win32") {
                    has_win = true;
                    if let Some(n) = eo.get("name").and_then(|v| v.as_str()) { exe = Some(n.to_string()); break; }
                }
            }
        }
        if !has_win { continue; }
        seen.insert(gid.clone());
        out.push(SpooferGame { id: gid, name, exe: exe.unwrap_or_else(|| "game.exe".into()) });
    }
    out
}

pub async fn load_games(tx: &tokio::sync::mpsc::UnboundedSender<ProgressEvent>) -> Vec<SpooferGame> {
    let now = unix_f64();
    {
        let cache = CACHE.lock().await;
        if let Some(c) = cache.as_ref() {
            if now - c.at < GAMES_TTL_SECS as f64 { return c.games.clone(); }
        }
    }

    let cp = games_cache_path();
    let mut old_games = Vec::new();
    
    if cp.exists() {
        let is_fresh = tokio::fs::metadata(&cp).await.ok().and_then(|m| m.modified().ok())
            .and_then(|mtime| SystemTime::now().duration_since(mtime).ok())
            .map(|age| age.as_secs() < GAMES_TTL_SECS).unwrap_or(false);

        if is_fresh {
            if let Ok(raw) = tokio::fs::read_to_string(&cp).await {
                let games = parse_games(&raw);
                if !games.is_empty() {
                    *CACHE.lock().await = Some(GamesCache { games: games.clone(), at: now });
                    return games;
                }
            }
        }
        if let Ok(raw) = tokio::fs::read_to_string(&cp).await {
            old_games = parse_games(&raw);
        }
    }

    let _ = tx.send(ProgressEvent::Log("→ Đang tải danh sách game từ Discord CDN...".into()));
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() { Ok(c) => c, Err(_) => return old_games };

    if let Ok(resp) = client.get(GAMES_CDN_URL).send().await {
        if resp.status().is_success() {
            if let Ok(raw) = resp.text().await {
                let games = parse_games(&raw);
                if !games.is_empty() {
                    if let Some(parent) = cp.parent() { let _ = tokio::fs::create_dir_all(parent).await; }
                    let _ = tokio::fs::write(&cp, &raw).await;
                    *CACHE.lock().await = Some(GamesCache { games: games.clone(), at: now });
                    return games;
                }
            }
        }
    }
    old_games
}

async fn build_super_props(build_number: u64, launch_signature: &str, heartbeat_session: &str) -> String {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap_or_default();
    let mut props = match client.post(PROPERTIES_URL)
        .header("content-type", "application/json")
        .header("user-agent", USER_AGENT_BADGE)
        .body("{}").send().await {
        Ok(res) => res.json::<Value>().await.unwrap_or_else(|_| json!({}))["properties"].clone(),
        Err(_) => json!({}),
    };

    if props.is_null() || !props.is_object() {
        props = json!({
            "os": "Windows", "browser": "Discord Client", "release_channel": "stable",
            "client_version": "1.0.9253", "os_version": "10.0.26200", "os_arch": "x64", "app_arch": "x64",
            "system_locale": "en-GB", "browser_user_agent": USER_AGENT_BADGE, "browser_version": "42.7.1",
            "client_build_number": build_number, "native_build_number": 88414, "client_event_source": null,
        });
    }
    props["client_launch_id"] = json!(uuid::Uuid::new_v4().to_string());
    props["launch_signature"] = json!(launch_signature);
    props["client_heartbeat_session_id"] = json!(heartbeat_session);
    base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&props).unwrap())
}

async fn fetch_analytics_token(token: &str, super_props: &str) -> Result<String, String> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().map_err(|e| e.to_string())?;
    let res = client.get(ME_URL)
        .header("authorization", token)
        .header("user-agent", USER_AGENT_BADGE)
        .header("x-super-properties", super_props)
        .send().await.map_err(|e| format!("{:?}", e))?;
    if !res.status().is_success() { return Err(format!("Lỗi xác thực: {}", res.status())); }
    let data: Value = res.json().await.map_err(|e| e.to_string())?;
    data.get("analytics_token").and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| "Không tìm thấy analytics_token".into())
}

struct ScienceSpoofer {
    analytics_token: String,
    fingerprint: String,
    heartbeat_session: String,
    launch_signature: String,
    seq: std::sync::atomic::AtomicU64,
    client: reqwest::Client,
    notify: Option<tokio::sync::mpsc::UnboundedSender<ProgressEvent>>,
}

impl ScienceSpoofer {
    fn new(token: &str, cookie: &str, analytics_token: String, super_props: String, heartbeat_session: String, launch_signature: String, fingerprint: String, proxy: Option<String>) -> Result<Self, String> {
        use reqwest::header::{HeaderMap, HeaderValue};
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("*/*"));
        headers.insert("accept-language", HeaderValue::from_static("en-GB"));
        headers.insert("authorization", HeaderValue::from_str(token).map_err(|e| e.to_string())?);
        headers.insert("cookie", HeaderValue::from_str(cookie).map_err(|e| e.to_string())?);
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("origin", HeaderValue::from_static("https://discord.com"));
        headers.insert("referer", HeaderValue::from_static("https://discord.com/channels/@me"));
        headers.insert("user-agent", HeaderValue::from_static(USER_AGENT_BADGE));
        headers.insert("x-debug-options", HeaderValue::from_static("bugReporterEnabled"));
        headers.insert("x-discord-locale", HeaderValue::from_static("en-GB"));
        headers.insert("x-discord-timezone", HeaderValue::from_static("Europe/Oslo"));
        headers.insert("x-super-properties", HeaderValue::from_str(&super_props).map_err(|e| e.to_string())?);

        let mut builder = reqwest::Client::builder().default_headers(headers).timeout(Duration::from_secs(20));
        if let Some(p) = proxy { if let Ok(px) = reqwest::Proxy::all(&p) { builder = builder.proxy(px); } }
        let client = builder.build().map_err(|e| e.to_string())?;

        Ok(Self { analytics_token, fingerprint, heartbeat_session, launch_signature, seq: std::sync::atomic::AtomicU64::new(0), client, notify: None })
    }

    fn next_seq(&self) -> u64 { self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1 }

    fn build_launch(&self, game: &SpooferGame) -> Value {
        let now = now_ms();
        let mut props = json!({
            "client_track_timestamp": now,
            "client_heartbeat_session_id": self.heartbeat_session,
            "event_sequence_number": self.next_seq(),
            "game": game.name, "game_id": game.id, "verified": true, "elevated": false,
            "is_launcher": false, "game_platform": "desktop", "detection_method": "verified_game",
            "is_overlay_enabled": false, "is_overlay_game_enabled": true,
            "is_overlay_game_source": "OOP_DEFAULT_DATABASE",
            "fullscreen_type": "UNKNOWN", "hardware_display_count": 1,
            "overlay_method": "Disabled", "activity_status_enabled": true,
            "activity_status_shared_guilds": [], "current_user_status": "online",
            "game_detection_enabled": true, "executable_path": game.exe,
            "voice_channel_id": null, "voice_channel_type": null, "voice_channel_bitrate": null,
            "voice_channel_guild_id": null, "hidden_by_distributor": false, "game_metadata": null,
            "client_performance_cpu": null, "client_performance_memory": null,
            "cpu_core_count": null, "accessibility_features": 0,
            "rendered_locale": "en-GB",
            "client_rtc_state": null, "client_app_state": "focused", "client_send_timestamp": now,
        });
        if !self.fingerprint.is_empty() { props["executable_fingerprint"] = json!(self.fingerprint); }
        json!({"type": "launch_game", "properties": props})
    }

    fn build_heartbeat(&self, game: &SpooferGame, duration_ms: u64, session_id: &str, initial: bool, final_: bool, ts: u64) -> Value {
        json!({"type": "running_game_heartbeat", "properties": {
            "client_track_timestamp": ts,
            "client_heartbeat_session_id": self.heartbeat_session,
            "event_sequence_number": self.next_seq(),
            "game_id": game.id, "game_name": game.name, "game_metadata": null,
            "game_executable": game.exe, "game_detection_enabled": true,
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
        let start = if now > duration_ms { now - duration_ms } else { now };
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
                    if let Some(tx) = &self.notify { let _ = tx.send(ProgressEvent::RateLimit { secs: 6 }); }
                    sleep(Duration::from_secs(6)).await;
                }
                status
            }
            Err(_) => 0,
        }
    }
}

pub struct BadgeJob {
    pub token: String,
    pub cookie: String,
    pub fingerprint: String,
    pub hours_per_game: f64,
    pub games_count: usize,
}

pub async fn run_badge_session(job: BadgeJob, tx: tokio::sync::mpsc::UnboundedSender<ProgressEvent>, cancel: CancellationToken) {
    let emit = |e: ProgressEvent| { let _ = tx.send(e); };
    emit(ProgressEvent::Log("→ Đang xác thực token...".into()));

    if let Err(e) = validate_token(&job.token).await {
        emit(ProgressEvent::BadgeDone { success: false, done: 0, failed: 0, total: job.games_count, reason: e });
        return;
    }

    match execute(&job, &tx, &cancel).await {
        Ok((done, failed, total)) => emit(ProgressEvent::BadgeDone { success: true, done, failed, total, reason: "Hoàn tất".into() }),
        Err((done, failed, total, reason)) => emit(ProgressEvent::BadgeDone { success: false, done, failed, total, reason }),
    }
}

async fn execute(job: &BadgeJob, tx: &tokio::sync::mpsc::UnboundedSender<ProgressEvent>, cancel: &CancellationToken) -> Result<(usize, usize, usize), (usize, usize, usize, String)> {
    let emit = |e: ProgressEvent| { let _ = tx.send(e); };
    emit(ProgressEvent::Log("→ Đang lấy build number...".into()));

    let build = get_build_number().await;
    let hb = uuid::Uuid::new_v4().to_string();
    let sig = uuid::Uuid::new_v4().to_string();
    let super_props = build_super_props(build, &sig, &hb).await;

    emit(ProgressEvent::Log("→ Đang lấy analytics token...".into()));
    let analytics = match fetch_analytics_token(&job.token, &super_props).await {
        Ok(t) => t,
        Err(e) => return Err((0, 0, job.games_count, e)),
    };

    if cancel.is_cancelled() { return Err((0, 0, job.games_count, "Đã huỷ.".into())); }

    let all = load_games(tx).await;
    if all.is_empty() { return Err((0, 0, job.games_count, "Không tải được games.json từ CDN.".into())); }

    let count = job.games_count.min(all.len()).min(MAX_GAMES);
    let selected: Vec<_> = all.choose_multiple(&mut rand::thread_rng(), count).cloned().collect();

    emit(ProgressEvent::Log(format!("→ Đã chọn {}/{} game, bắt đầu buff...", count, count)));

    if cancel.is_cancelled() { return Err((0, 0, count, "Đã huỷ.".into())); }

    let mut spoofer = match ScienceSpoofer::new(
        &job.token, &job.cookie, analytics, super_props, hb, sig, job.fingerprint.clone(), random_proxy(),
    ) {
        Ok(s) => s,
        Err(e) => return Err((0, 0, count, e)),
    };
    spoofer.notify = Some(tx.clone());
    let duration_ms = (job.hours_per_game * 3600.0 * 1000.0) as u64;

    let Some(test_game) = selected.first().cloned() else { return Err((0, 0, count, "Không có game nào.".into())); };
    let test_status = spoofer.post(spoofer.build_events(&test_game, duration_ms)).await;

    // The test request must succeed to reach the batching loop below.
    let mut done = 1usize;
    let mut failed = 0usize;

    if test_status == 204 {
        let _ = tx.send(ProgressEvent::BadgeUpdate { done, failed, total: count, desc: format!("Test OK: {}", test_game.name) });
    } else if matches!(test_status, 401 | 403) {
        return Err((0, 0, count, format!("Auth bị từ chối (mã {test_status}). Hãy kiểm tra Cookie!")));
    } else {
        return Err((0, 0, count, format!("Test endpoint thất bại (mã {test_status}).")));
    }

    for chunk in selected[1..].chunks(BATCH_SIZE) {
        if cancel.is_cancelled() { return Err((done, failed, count, "Đã huỷ bởi người dùng.".into())); }

        let mut events = Vec::with_capacity(chunk.len() * 3);
        for g in chunk { events.extend(spoofer.build_events(g, duration_ms)); }

        let mut success = false;
        for _ in 0..3 {
            let status = spoofer.post(events.clone()).await;
            if status == 204 { success = true; break; }
            if matches!(status, 401 | 403) { return Err((done, failed, count, format!("Auth bị từ chối giữa chừng (mã {status})."))); }
            sleep(Duration::from_secs(3)).await;
        }
        if success { done += chunk.len(); } else { failed += chunk.len(); }

        let _ = tx.send(ProgressEvent::BadgeUpdate { done, failed, total: count, desc: format!("Đang buff... {}/{}", done, count) });
        sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
    }

    if failed > 0 { Err((done, failed, count, format!("{done}/{count} OK, {failed} thất bại."))) } else { Ok((done, failed, count)) }
}
