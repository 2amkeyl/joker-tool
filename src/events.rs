use crate::quest::QuestInfo;
use crate::updater::UpdateInfo;

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Log(String),
    QuestList(Result<Vec<QuestInfo>, String>),
    QuestsFound { new: Vec<QuestInfo>, all_ids: Vec<String> },
    RateLimit { secs: u64 },
    TokenStatus { valid: bool, msg: String, user: Option<crate::quest::UserInfo> },
    QuestUpdate { name: String, task_type: String, sec_done: f64, sec_needed: f64 },
    QuestCompleted { name: String, success: bool, reason: Option<String> },
    QuestAllDone { completed: usize, failed: usize, reason: String },
    BadgeUpdate { done: usize, failed: usize, total: usize, desc: String },
    BadgeDone { success: bool, done: usize, failed: usize, total: usize, reason: String },
    HypeSquadResult { success: bool, msg: String },
    UpdateChecked { result: Result<Option<UpdateInfo>, String>, manual: bool },
    UpdateInstalled(Result<(), String>),
    FingerprintResult { ok: bool, value: String },
    Avatar { user_id: String, rgba: Vec<u8> },
}
