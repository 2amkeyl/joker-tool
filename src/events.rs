//! Sự kiện giữa worker thread và GUI thread.

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Log message — vừa hiện GUI vừa ghi file log.txt
    Log(String),

    /// Token được kiểm tra
    TokenStatus  {
        valid: bool,
        msg: String,
        user: Option<crate::quest::UserInfo>,
    },

    /// Đang cày 1 quest, cập nhật tiến độ
    QuestUpdate {
        name: String,
        task_type: String,
        sec_done: f64,
        sec_needed: f64,
    },

    /// 1 quest xong (thành công hoặc thất bại)
    QuestCompleted {
        name: String,
        success: bool,
        reason: Option<String>,
    },

    /// Toàn bộ phiên quest kết thúc
    QuestAllDone {
        completed: usize,
        failed: usize,
        reason: String,
    },

    /// Đang buff badge, cập nhật tiến độ
    BadgeUpdate {
        done: usize,
        failed: usize,
        total: usize,
        desc: String,
    },

    /// Badge xong
    BadgeDone {
        success: bool,
        done: usize,
        total: usize,
        reason: String,
    },

    /// HypeSquad kết quả
    HypeSquadResult { success: bool, msg: String },

    /// Avatar Discord đã tải xong
    Avatar { user_id: String, rgba: Vec<u8> },
}