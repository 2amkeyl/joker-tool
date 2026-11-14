use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use base64::{engine::general_purpose::STANDARD, Engine as _};

#[cfg(windows)]
fn encrypt(s: &str) -> String {
    if s.is_empty() { return String::new(); }
    match windows_dpapi::encrypt_data(s.as_bytes(), windows_dpapi::Scope::User, None) {
        Ok(data) => STANDARD.encode(data),
        Err(_) => String::new(),
    }
}

#[cfg(windows)]
fn decrypt(s: &str) -> String {
    if s.is_empty() { return String::new(); }
    if let Ok(data) = STANDARD.decode(s) {
        if let Ok(bytes) = windows_dpapi::decrypt_data(&data, windows_dpapi::Scope::User, None) {
            if let Ok(res) = String::from_utf8(bytes) { return res; }
        }
    }
    String::new()
}

#[cfg(not(windows))]
fn encrypt(s: &str) -> String { STANDARD.encode(s.as_bytes()) }
#[cfg(not(windows))]
fn decrypt(s: &str) -> String { STANDARD.decode(s).ok().and_then(|b| String::from_utf8(b).ok()).unwrap_or_default() }

fn yes() -> bool { true }
fn auto() -> String { "auto".into() }
fn default_lang() -> String { "vi".into() }
fn default_theme() -> String { "dark".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LegacyProfile {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token_enc: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cookie_enc: String,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(skip)]
    pub token: String,
    #[serde(skip)]
    pub cookie: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token_enc: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cookie_enc: String,
    #[serde(skip)]
    pub token: String,
    #[serde(skip)]
    pub cookie: String,
    #[serde(default)]
    pub badge_time_key: String,
    #[serde(default)]
    pub badge_game_key: String,
    // Read old multi-profile configs once, migrate the active profile to the
    // single-account fields below, and never write the legacy list again.
    #[serde(default, rename = "profiles", skip_serializing)]
    legacy_profiles: Vec<LegacyProfile>,
    #[serde(default, rename = "active_id", skip_serializing)]
    legacy_active_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub badge_fingerprint: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub onboarded: bool,
    #[serde(default)]
    pub tray_on_close: bool,
    #[serde(default = "yes")]
    pub notify: bool,
    #[serde(default)]
    pub light: bool,
    #[serde(default)]
    pub theme: String,
    #[serde(default = "auto")]
    pub accent: String,
    #[serde(default)]
    pub watch_quests: bool,
    #[serde(default)]
    pub seen_quests: Vec<String>,
    #[serde(default)]
    pub autostart: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            token_enc: String::new(), cookie_enc: String::new(),
            token: String::new(), cookie: String::new(),
            badge_time_key: String::new(), badge_game_key: String::new(),
            legacy_profiles: Vec::new(), legacy_active_id: String::new(), badge_fingerprint: String::new(),
            lang: default_lang(), onboarded: false, tray_on_close: false, notify: true, light: false,
            theme: default_theme(), accent: auto(), watch_quests: false, seen_quests: Vec::new(), autostart: false,
        }
    }
}

impl AppConfig {
    pub fn dir() -> PathBuf {
        directories::ProjectDirs::from("", "", "JokerTool").map(|d| d.config_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
    }
    pub fn config_path() -> PathBuf { Self::dir().join("config.json") }
    pub fn log_path() -> PathBuf { Self::dir().join("log.txt") }
    pub fn stats_path() -> PathBuf { Self::dir().join("stats.json") }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() { return Self::default(); }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(), 
        };
        
        match serde_json::from_str::<Self>(&content) {
            Ok(mut cfg) => {
                cfg.token = decrypt(&cfg.token_enc);
                cfg.cookie = decrypt(&cfg.cookie_enc);

                // Migrate legacy multi-account configs to one remembered account.
                // Prefer the previously active profile; fall back to the first valid one.
                for p in &mut cfg.legacy_profiles {
                    p.token = decrypt(&p.token_enc);
                    p.cookie = decrypt(&p.cookie_enc);
                }
                let mut migrated_profiles = false;
                if cfg.token.is_empty() {
                    let selected = cfg.legacy_profiles.iter().find(|p| {
                        p.id == cfg.legacy_active_id && !p.token.is_empty()
                    }).or_else(|| cfg.legacy_profiles.iter().find(|p| !p.token.is_empty()));
                    if let Some(p) = selected {
                        cfg.token = p.token.clone();
                        cfg.cookie = p.cookie.clone();
                        cfg.badge_fingerprint = p.fingerprint.clone();
                        migrated_profiles = true;
                    }
                }
                if !cfg.legacy_profiles.is_empty() || !cfg.legacy_active_id.is_empty() {
                    cfg.legacy_profiles.clear();
                    cfg.legacy_active_id.clear();
                    migrated_profiles = true;
                }

                // Migrate the previous two-state `light` setting to the explicit theme mode.
                if cfg.theme.is_empty() {
                    cfg.theme = if cfg.light { "light" } else { "dark" }.into();
                }
                if !matches!(cfg.theme.as_str(), "light" | "dark" | "system") {
                    cfg.theme = "dark".into();
                }
                cfg.light = cfg.theme == "light";
                if migrated_profiles {
                    // Persist the one-account format immediately; credentials remain DPAPI-encrypted.
                    let _ = cfg.save();
                }
                cfg
            }
            Err(_) => {
                let _ = std::fs::rename(&path, path.with_extension("corrupt"));
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::dir();
        std::fs::create_dir_all(&dir)?;
        let path = Self::config_path();
        let tmp = path.with_extension("tmp");

        let mut to_save = self.clone();
        // Keep the legacy boolean synchronized for older builds.
        to_save.light = to_save.theme == "light";
        to_save.token_enc = encrypt(&self.token);
        to_save.cookie_enc = encrypt(&self.cookie);
        std::fs::write(&tmp, serde_json::to_vec_pretty(&to_save)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }


}
