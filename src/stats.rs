use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::time::Instant;

const MAX_SESSIONS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub ts: i64, pub kind: String, pub account: String, pub ok: u32, pub fail: u32, pub secs: u32,
}

struct Running { start: Instant, kind: String, account: String }

#[derive(Default, Serialize, Deserialize)]
pub struct Stats {
    #[serde(default)] pub quests_ok: u64,
    #[serde(default)] pub quests_fail: u64,
    #[serde(default)] pub games: u64,
    #[serde(default)] pub secs: u64,
    #[serde(default)] pub sessions: Vec<Session>,
    #[serde(skip)] running: Option<Running>,
}

impl Stats {
    pub fn load() -> Self {
        std::fs::read_to_string(AppConfig::stats_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = AppConfig::stats_path();
        if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
        if let Ok(bytes) = serde_json::to_vec(self) {
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, bytes).is_ok() { let _ = std::fs::rename(&tmp, &path); }
        }
    }

    pub fn begin(&mut self, kind: &str, account: &str) {
        self.running = Some(Running { start: Instant::now(), kind: kind.to_string(), account: account.to_string() });
    }

    pub fn finish(&mut self, kind: &str, ok: u32, fail: u32) {
        let (secs, account) = match self.running.take() {
            Some(r) if r.kind == kind => (r.start.elapsed().as_secs() as u32, r.account),
            Some(r) => { self.running = Some(r); (0, String::new()) }
            None => (0, String::new()),
        };
        match kind {
            "quest" => { self.quests_ok += ok as u64; self.quests_fail += fail as u64; }
            "badge" => self.games += ok as u64,
            _ => {}
        }
        self.secs += secs as u64;
        if ok == 0 && fail == 0 && secs < 3 { return; }
        self.sessions.push(Session { ts: chrono::Local::now().timestamp(), kind: kind.to_string(), account, ok, fail, secs });
        if self.sessions.len() > MAX_SESSIONS {
            let extra = self.sessions.len() - MAX_SESSIONS;
            self.sessions.drain(0..extra);
        }
        self.save();
    }

    pub fn daily_ok(&self, days: usize) -> Vec<(String, u32)> {
        use chrono::{Duration, Local, TimeZone};
        let today = Local::now().date_naive();
        (0..days).rev().map(|back| {
            let day = today - Duration::days(back as i64);
            let total: u32 = self.sessions.iter()
                .filter(|s| s.kind == "quest")
                .filter(|s| Local.timestamp_opt(s.ts, 0).single().map(|d| d.date_naive() == day).unwrap_or(false))
                .map(|s| s.ok)
                .sum();
            (day.format("%d/%m").to_string(), total)
        }).collect()
    }

    pub fn clear(&mut self) {
        let running = self.running.take();
        *self = Stats::default();
        self.running = running;
        self.save();
    }
}
