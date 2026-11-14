use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;

const REPO: &str = "2amkeyl/joker-tool";
const EXE_ASSET: &str = "JokerTool.exe";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
}

pub fn is_prerelease(v: &str) -> bool { v.trim_start_matches(['v', 'V']).contains('-') }

fn split_ver(s: &str) -> (Vec<u64>, Option<String>) {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let s = s.split('+').next().unwrap_or(s);
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p.to_string())),
        None => (s, None),
    };
    let nums = core.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect();
    (nums, pre)
}

fn cmp_pre(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut ia, mut ib) = (a.split('.'), b.split('.'));
    loop {
        match (ia.next(), ib.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let o = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(nx), Ok(ny)) => nx.cmp(&ny),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if o != Ordering::Equal { return o; }
            }
        }
    }
}

pub fn is_newer(remote: &str, local: &str) -> bool {
    let ((mut a, pa), (mut b, pb)) = (split_ver(remote), split_ver(local));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    if a != b { return a > b; }
    match (pa, pb) {
        (None, None) => false,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(x), Some(y)) => cmp_pre(&x, &y) == std::cmp::Ordering::Greater,
    }
}

fn client(timeout: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("JokerTool-updater/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(timeout))
        .build()
        .map_err(|e| e.to_string())
}

pub async fn check() -> Result<Option<UpdateInfo>, String> {
    let want_pre = is_prerelease(CURRENT);
    let http = client(15)?;
    let url = if want_pre {
        format!("https://api.github.com/repos/{}/releases?per_page=20", REPO)
    } else {
        format!("https://api.github.com/repos/{}/releases/latest", REPO)
    };
    let resp = http.get(url).header("accept", "application/vnd.github+json").send().await.map_err(|e| format!("Không kết nối được GitHub: {}", e))?;
    if resp.status().as_u16() == 404 && !want_pre { return Ok(None); }
    if !resp.status().is_success() { return Err(format!("GitHub trả về mã {}", resp.status().as_u16())); }
    
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    let releases: Vec<Value> = match body {
        Value::Array(a) => a,
        other => vec![other],
    };

    let mut best: Option<&Value> = None;
    for r in &releases {
        if r["draft"].as_bool().unwrap_or(false) { continue; }
        if !want_pre && r["prerelease"].as_bool().unwrap_or(false) { continue; }
        let tag = r["tag_name"].as_str().unwrap_or_default();
        if tag.is_empty() { continue; }
        let better = match best {
            None => true,
            Some(b) => is_newer(tag, b["tag_name"].as_str().unwrap_or_default()),
        };
        if better { best = Some(r); }
    }

    let Some(rel) = best else { return Ok(None) };
    let tag = rel["tag_name"].as_str().unwrap_or_default().to_string();
    if !is_newer(&tag, CURRENT) { return Ok(None); }
    
    let asset = rel["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(EXE_ASSET)))
        .ok_or_else(|| format!("Bản {} chưa có file {}", tag, EXE_ASSET))?;
    let url = asset["browser_download_url"].as_str().ok_or("Thiếu link tải")?.to_string();
    let sha256 = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(|s| s.to_lowercase());
    Ok(Some(UpdateInfo { version: tag, url, sha256 }))
}

pub async fn download_and_install(info: &UpdateInfo) -> Result<(), String> {
    let resp = client(300)?.get(&info.url).send().await.map_err(|e| format!("Tải thất bại: {}", e))?;
    if !resp.status().is_success() { return Err(format!("Tải thất bại (mã {})", resp.status().as_u16())); }
    let bytes = resp.bytes().await.map_err(|e| format!("Tải thất bại: {}", e))?;

    if bytes.len() < 500_000 || &bytes[..2] != b"MZ" { return Err("File tải về không phải chương trình hợp lệ.".into()); }
    if let Some(expected) = &info.sha256 {
        let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{:02x}", b)).collect();
        if &got != expected { return Err("Sai mã SHA-256, đã huỷ cập nhật để an toàn.".into()); }
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new_path = exe.with_extension("new");
    let old_path = exe.with_extension("old");
    std::fs::write(&new_path, &bytes).map_err(|e| format!("Không ghi được file mới: {}", e))?;
    let _ = std::fs::remove_file(&old_path);

    if let Err(e) = std::fs::rename(&exe, &old_path) {
        let _ = std::fs::remove_file(&new_path);
        return Err(format!("Không thay được file hiện tại: {}", e));
    }
    if let Err(e) = std::fs::rename(&new_path, &exe) {
        if let Err(rb_err) = std::fs::rename(&old_path, &exe) {
            return Err(format!("Thay file mới thất bại: {}. Rollback cũng thất bại: {}", e, rb_err));
        }
        return Err(format!("Không đặt được file mới (đã rollback): {}", e));
    }
    Ok(())
}

pub fn restart() -> ! {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg("--restarted").spawn();
    }
    std::process::exit(0);
}

pub fn cleanup_old() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(exe.with_extension("old"));
    }
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn stable_ordering() {
        assert!(is_newer("v1.3.0", "1.2.0"));
        assert!(!is_newer("v1.2.0", "1.2.0"));
        assert!(is_newer("1.2.1", "1.2.0"));
        assert!(!is_newer("1.1.9", "1.2.0"));
    }

    #[test]
    fn prerelease_ordering() {
        assert!(!is_newer("v1.3.0-beta.1", "1.3.0"));
        assert!(is_newer("v1.3.0", "1.3.0-beta.1"));
        assert!(is_newer("v1.3.0-beta.2", "1.3.0-beta.1"));
        assert!(is_newer("v1.3.0-beta.10", "1.3.0-beta.2"));
        assert!(is_newer("v1.3.0-rc.1", "1.3.0-beta.9"));
        assert!(!is_newer("v1.3.0-beta.1", "1.3.0-beta.1"));
        assert!(is_newer("v1.3.0-beta.1", "1.2.0"));
        assert!(!is_newer("v1.2.9", "1.3.0-beta.1"));
    }
}