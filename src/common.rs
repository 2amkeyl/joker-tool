//! Helpers format thời gian, cắt chuỗi.

pub fn fmt_dur(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{}g {:02}p", h, m)
    } else if m > 0 {
        format!("{}p {:02}s", m, sec)
    } else {
        format!("{}s", sec)
    }
}

pub fn short(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}