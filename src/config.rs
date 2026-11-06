//! Lưu/đọc config trong %APPDATA%\JokerTool\config.json

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use base64::{engine::general_purpose::STANDARD, Engine as _};

const XOR_KEY: &[u8] = b"JokerPhantomThieves";

fn obfuscate(data: &str) -> String {
    if data.is_empty() { return String::new(); }
    let bytes = data.as_bytes();
    let obf: Vec<u8> = bytes.iter().enumerate().map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()]).collect();
    STANDARD.encode(obf)
}

fn deobfuscate(data: &str) -> String {
    if data.is_empty() { return String::new(); }
    if let Ok(decoded) = STANDARD.decode(data) {
        let deobf: Vec<u8> = decoded.iter().enumerate().map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()]).collect();
        if let Ok(s) = String::from_utf8(deobf) {
            return s;
        }
    }
    String::new()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token_enc: String,
    #[serde(skip)]
    pub token: String, // Token thực tế lưu trong RAM
    #[serde(default)]
    pub badge_time_key: String,
    #[serde(default)]
    pub badge_game_key: String,
}

impl AppConfig {
    pub fn dir() -> PathBuf {
        directories::ProjectDirs::from("", "", "JokerTool")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn config_path() -> PathBuf {
        Self::dir().join("config.json")
    }

    pub fn log_path() -> PathBuf {
        Self::dir().join("log.txt")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() { return Self::default(); }
        
        let mut cfg: Self = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
            
        cfg.token = deobfuscate(&cfg.token_enc);
        cfg
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::dir();
        std::fs::create_dir_all(&dir)?;
        let path = Self::config_path();
        let tmp = path.with_extension("tmp");
        
        let mut to_save = self.clone();
        to_save.token_enc = obfuscate(&self.token);
        
        std::fs::write(&tmp, serde_json::to_vec_pretty(&to_save)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}
