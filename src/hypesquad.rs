use serde_json::json;

pub async fn set_hypesquad_house(user_token: &str, house_name: &str) -> (bool, String) {
    let houses = [
        ("bravery", 1u32),
        ("brilliance", 2),
        ("balance", 3),
        ("leave", 0),
    ];
    let key = house_name.to_lowercase();
    let house_id = match houses.iter().find(|(k, _)| *k == key) {
        Some((_, id)) => *id,
        None => return (false, "Nhà HypeSquad không hợp lệ!".into()),
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (false, format!("Lỗi HTTP: {e}")),
    };

    let url = "https://discord.com/api/v9/hypesquad/online";
    let result = if house_id == 0 {
        let r = client
            .post(url)
            .header("Authorization", user_token)
            .json(&json!({}))
            .send()
            .await;
        let ok = r
            .as_ref()
            .map(|r| matches!(r.status().as_u16(), 200 | 204))
            .unwrap_or(false);
        if !ok {
            let r2 = client
                .delete(url)
                .header("Authorization", user_token)
                .send()
                .await;
            r2.as_ref()
                .map(|r| matches!(r.status().as_u16(), 200 | 204))
                .unwrap_or(false)
        } else {
            true
        }
    } else {
        let r = client
            .post(url)
            .header("Authorization", user_token)
            .json(&json!({"house_id": house_id}))
            .send()
            .await;
        r.as_ref()
            .map(|r| matches!(r.status().as_u16(), 200 | 204))
            .unwrap_or(false)
    };

    if result {
        let msg = if house_id == 0 {
            "Đã gỡ huy hiệu HypeSquad.".to_string()
        } else {
            format!(
                "Đã gán thành công huy hiệu HypeSquad {}",
                capitalize(house_name)
            )
        };
        (true, msg)
    } else {
        (false, "Token lỗi hoặc hết hạn rồi nhé!".into())
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}