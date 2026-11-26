use crate::quest::UserInfo;
use std::time::Duration;

pub const AVATAR_SIZE: u32 = 96;

pub async fn fetch_avatar(user: &UserInfo) -> Option<Vec<u8>> {
    let url = match &user.avatar {
        Some(h) if !h.is_empty() => format!("https://cdn.discordapp.com/avatars/{}/{}.png?size=128", user.id, h),
        _ => {
            let idx = user.id.parse::<u64>().map(|v| (v >> 22) % 6).unwrap_or(0);
            format!("https://cdn.discordapp.com/embed/avatars/{}.png", idx)
        }
    };
    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let bytes = resp.bytes().await.ok()?;

    let img = image::load_from_memory(&bytes).ok()?;
    let img = img.resize_exact(AVATAR_SIZE, AVATAR_SIZE, image::imageops::FilterType::Lanczos3);
    let mut rgba = img.to_rgba8();

    let c = AVATAR_SIZE as f32 / 2.0;
    for (x, y, px) in rgba.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - c;
        let dy = y as f32 + 0.5 - c;
        let a = (c - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
        px.0[3] = (px.0[3] as f32 * a) as u8;
    }
    Some(rgba.into_raw())
}
