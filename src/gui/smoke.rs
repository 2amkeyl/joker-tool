
use super::*;
use crate::config::AppConfig;
use crate::log_writer::LogWriter;
use crate::shared::Shared;
use std::collections::HashMap as Map;

struct Tex {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

fn apply_deltas(texs: &mut Map<egui::TextureId, Tex>, delta: &egui::TexturesDelta) {
    for (id, d) in &delta.set {
        let (w, h) = (d.image.width(), d.image.height());
        let rgba: Vec<u8> = match &d.image {
            egui::ImageData::Color(c) => c.pixels.iter().flat_map(|p| p.to_array()).collect(),
            egui::ImageData::Font(f) => f.srgba_pixels(None).flat_map(|p| p.to_array()).collect(),
        };
        match d.pos {
            None => {
                texs.insert(*id, Tex { w, h, px: rgba });
            }
            Some([x0, y0]) => {
                if let Some(t) = texs.get_mut(id) {
                    for row in 0..h {
                        let dst = ((y0 + row) * t.w + x0) * 4;
                        let src = row * w * 4;
                        t.px[dst..dst + w * 4].copy_from_slice(&rgba[src..src + w * 4]);
                    }
                }
            }
        }
    }
}

fn quest(id: &str, name: &str, task: &str, reward: &str, hours: i64, done: f64, completed: bool, completable: bool) -> QuestInfo {
    QuestInfo {
        id: id.into(),
        name: name.into(),
        task_type: task.into(),
        needed: 900,
        done,
        enrolled: done > 0.0,
        completed,
        completable,
        expires: Some((chrono::Utc::now() + chrono::Duration::hours(hours)).to_rfc3339()),
        reward: if reward.is_empty() { None } else { Some(reward.into()) },
    }
}

fn run_frames(app: &mut JokerApp, ctx: &egui::Context, texs: &mut Map<egui::TextureId, Tex>, n: usize, size: egui::Vec2, t: &mut f64) -> egui::FullOutput {
    let mut last = None;
    for _ in 0..n {
        *t += 0.05;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            time: Some(*t),
            focused: true,
            ..Default::default()
        };
        app.tab_changed = Instant::now() - Duration::from_secs(5);
        let out = ctx.run(input, |ctx| app.frame(ctx));
        apply_deltas(texs, &out.textures_delta);
        last = Some(out);
    }
    last.unwrap()
}

fn dump(dir: &std::path::Path, name: &str, ctx: &egui::Context, out: egui::FullOutput, texs: &Map<egui::TextureId, Tex>, size: egui::Vec2) {
    let ppp = out.pixels_per_point;
    let prims = ctx.tessellate(out.shapes, ppp);
    let mut meshes = Vec::new();
    let mut count = 0usize;
    for cp in prims {
        if let egui::epaint::Primitive::Mesh(m) = cp.primitive {
            let verts: Vec<[f32; 8]> = m
                .vertices
                .iter()
                .map(|v| [v.pos.x, v.pos.y, v.uv.x, v.uv.y, v.color.r() as f32, v.color.g() as f32, v.color.b() as f32, v.color.a() as f32])
                .collect();
            count += m.indices.len() / 3;
            let tex = match m.texture_id {
                egui::TextureId::Managed(i) => format!("m{}", i),
                egui::TextureId::User(i) => format!("u{}", i),
            };
            let r = cp.clip_rect;
            meshes.push(serde_json::json!({"clip":[r.min.x, r.min.y, r.max.x, r.max.y], "tex": tex, "v": verts, "i": m.indices}));
        }
    }
    let mut tex_meta = serde_json::Map::new();
    for (id, t) in texs {
        let key = match id {
            egui::TextureId::Managed(i) => format!("m{}", i),
            egui::TextureId::User(i) => format!("u{}", i),
        };
        std::fs::write(dir.join(format!("{}_{}.rgba", name, key)), &t.px).unwrap();
        tex_meta.insert(key, serde_json::json!({"w": t.w, "h": t.h}));
    }
    let doc = serde_json::json!({"size":[size.x, size.y], "ppp": ppp, "textures": tex_meta, "meshes": meshes});
    std::fs::write(dir.join(format!("{}.json", name)), serde_json::to_vec(&doc).unwrap()).unwrap();
    eprintln!("dumped {} ({} triangles)", name, count);
}

#[test]
fn render_all_tabs() {
    let dump_dir = std::env::var("JOKER_DUMP").ok().map(std::path::PathBuf::from);
    if let Some(d) = &dump_dir {
        std::fs::create_dir_all(d).unwrap();
    }
    let ctx = egui::Context::default();
    if dump_dir.is_some() {
        ctx.set_pixels_per_point(2.0);
    }
    crate::setup_custom_fonts(&ctx);
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let shared = Shared::new(false);
    {
        let mut s = shared.stats.lock();
        s.quests_ok = 14;
        s.secs = 5 * 3600 + 20 * 60;
    }
    let mut cfg = AppConfig::default();
    cfg.onboarded = true;
    let log = LogWriter::new(std::env::temp_dir().join("joker_smoke.log"));
    let mut app = JokerApp::new(rt, cfg, log, tx, rx, shared, ctx.clone());
    app.onboard_step = usize::MAX;
    app.user_info = Some(UserInfo { id: "123456789012345678".into(), username: "demo".into(), global_name: Some("Demo User".into()), avatar: None });

    let list = vec![
        quest("1001", "Tham gia sự kiện Orbs trong Fortnite", "PLAY_ON_DESKTOP", "Orbs x700", 52, 300.0, false, true),
        quest("1002", "Xem video giới thiệu bản cập nhật mới", "WATCH_VIDEO", "Avatar decoration", 7, 0.0, false, true),
        quest("1003", "Stream một game bất kỳ trong 15 phút", "STREAM_ON_DESKTOP", "Profile effect", 130, 0.0, false, true),
        quest("1004", "Chơi trên thiết bị di động", "PLAY_ACTIVITY", "", 300, 0.0, false, false),
        quest("1005", "Quest đã hoàn thành tuần trước", "WATCH_VIDEO", "Badge", 20, 900.0, true, true),
    ];
    app.quest_selected = list.iter().filter(|q| q.selectable()).map(|q| q.id.clone()).collect();
    app.quest_queue = list.iter().filter(|q| q.selectable()).map(|q| q.id.clone()).collect();
    app.recent_new = vec![list[1].clone(), list[0].clone()];
    app.quest_list = Some(list);
    app.current_quest = Some(("Tham gia sự kiện Orbs trong Fortnite".into(), "PLAY_ON_DESKTOP".into(), 412.0, 900.0));
    app.quest_running = false;
    app.badge_progress = (3, 0, 10);
    app.badge_running = true;
    app.badge_desc = "Đang giữ phiên chơi ảo...".into();
    app.nav_flags[Tab::Quest.index()] = NavFlag::New;
    app.nav_flags[Tab::HypeSquad.index()] = NavFlag::Error;
    app.badge_running = false;

    let size = egui::vec2(900.0, 640.0);
    let mut texs = Map::new();
    let mut t = 0.0;
    let cases: [(&str, Tab, bool, f32, bool); 6] = [
        ("quest", Tab::Quest, false, 1500.0, false),
        ("badge", Tab::Badge, false, 1900.0, false),
        ("hype", Tab::HypeSquad, false, 900.0, false),
        ("settings", Tab::Settings, false, 1500.0, false),
        ("quest_light", Tab::Quest, true, 1500.0, false),
        ("menu_top", Tab::Badge, false, 760.0, true),
    ];
    app.log_open = false;
    for (name, tab, light, height, menu) in cases {
        app.current_tab = tab;
        app.account_menu_open = menu;
        app.cfg.theme = if light { "light".into() } else { "dark".into() };
        app.cfg.light = light;
        theme::set_light(light);
        theme::apply_theme(&ctx);
        let case_size = egui::vec2(size.x, height);
        let out = run_frames(&mut app, &ctx, &mut texs, 24, case_size, &mut t);
        if let Some(d) = &dump_dir {
            dump(d, name, &ctx, out, &texs, case_size);
        }
    }
    app.account_menu_open = false;
    app.user_info = None;
    app.avatar_tex = None;
    app.current_tab = Tab::Quest;
    app.cfg.theme = "dark".into();
    app.cfg.light = false;
    theme::set_light(false);
    theme::apply_theme(&ctx);
    {
        let case_size = egui::vec2(size.x, 760.0);
        let out = run_frames(&mut app, &ctx, &mut texs, 24, case_size, &mut t);
        if let Some(d) = &dump_dir {
            dump(d, "quest_out", &ctx, out, &texs, case_size);
        }
    }
    app.cfg.motion_fps = 30;
    app.current_tab = Tab::Quest;
    let _ = run_frames(&mut app, &ctx, &mut texs, 6, size, &mut t);
    let _ = run_frames(&mut app, &ctx, &mut texs, 4, egui::vec2(640.0, 420.0), &mut t);
}

#[test]
fn queue_drag_reorders() {
    use egui::{Event, PointerButton, Pos2};
    let ctx = egui::Context::default();
    crate::setup_custom_fonts(&ctx);
    let list: Vec<QuestInfo> = ["a", "b", "c"].iter().map(|id| quest(id, &format!("Quest {id}"), "WATCH_VIDEO", "", 10, 0.0, false, true)).collect();
    let mut queue: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
    let mut drag: Option<usize> = None;
    let mut origin = Pos2::ZERO;
    let mut width = 0.0f32;
    let mut t = 0.0f64;

    let frame = |events: Vec<Event>, queue: &mut Vec<String>, drag: &mut Option<usize>, origin: &mut Pos2, width: &mut f32, t: &mut f64| {
        *t += 0.016;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(700.0, 400.0))),
            time: Some(*t),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                *origin = ui.cursor().min;
                *width = ui.available_width();
                super::tabs::queue_list(ui, queue, drag, &list, Color32::RED);
            });
        });
    };

    frame(vec![], &mut queue, &mut drag, &mut origin, &mut width, &mut t);
    let row_h = 46.0 + 6.0;
    let (ox, oy, ow) = (origin.x, origin.y, width);
    let row_center = move |i: f32| Pos2::new(ox + ow * 0.5, oy + i * row_h + 23.0);

    frame(vec![Event::PointerMoved(row_center(2.0))], &mut queue, &mut drag, &mut origin, &mut width, &mut t);
    frame(vec![Event::PointerButton { pos: row_center(2.0), button: PointerButton::Primary, pressed: true, modifiers: Default::default() }], &mut queue, &mut drag, &mut origin, &mut width, &mut t);
    for step in 1..=12 {
        let y = row_center(2.0).y + (row_center(0.0).y - row_center(2.0).y) * step as f32 / 12.0;
        frame(vec![Event::PointerMoved(Pos2::new(row_center(2.0).x, y))], &mut queue, &mut drag, &mut origin, &mut width, &mut t);
    }
    assert_eq!(drag, Some(2), "dragging the third row");
    frame(vec![Event::PointerButton { pos: row_center(0.0), button: PointerButton::Primary, pressed: false, modifiers: Default::default() }], &mut queue, &mut drag, &mut origin, &mut width, &mut t);
    frame(vec![], &mut queue, &mut drag, &mut origin, &mut width, &mut t);
    assert_eq!(queue, vec!["c".to_string(), "a".to_string(), "b".to_string()]);
    assert_eq!(drag, None);
}

#[test]
fn quest_expiry_parser_accepts_rfc3339_and_unix_timestamps() {
    let now = chrono::Utc::now().timestamp();
    assert!(crate::quest::parse_expiry_datetime(&(now - 60).to_string()).unwrap() < chrono::Utc::now());
    assert!(crate::quest::parse_expiry_datetime(&((now - 60) * 1000).to_string()).unwrap() < chrono::Utc::now());
    assert!(crate::quest::parse_expiry_datetime("not-a-date").is_none());
}

#[test]
fn expired_and_unsupported_quests_stay_at_the_bottom_for_each_sort() {
    let active_later = quest("active-later", "Quest A", "WATCH_VIDEO", "", 48, 0.0, false, true);
    let active_soon = quest("active-soon", "Quest Z", "WATCH_VIDEO", "", 12, 0.0, false, true);
    let completed = quest("completed", "Quest completed", "WATCH_VIDEO", "", 24, 900.0, true, true);
    let expired = quest("expired", "Quest expired", "WATCH_VIDEO", "", -24, 0.0, false, false);
    let completed_expired = quest("completed-expired", "Quest completed but expired", "WATCH_VIDEO", "", -12, 900.0, true, false);
    let unsupported = quest("unsupported", "Quest unsupported", "UNSUPPORTED", "", 24, 0.0, false, false);

    assert!(!expired.selectable(), "expired quests must not be selectable");
    assert!(!completed_expired.selectable(), "completed-but-expired quests must not be selectable");
    assert!(!unsupported.selectable(), "unsupported quests must not be selectable");

    for (sort, expected_active) in [
        (QuestSort::Expiry, vec!["active-soon", "active-later"]),
        (QuestSort::Name, vec!["active-later", "active-soon"]),
    ] {
        let mut quests = [
            unsupported.clone(),
            completed_expired.clone(),
            expired.clone(),
            completed.clone(),
            active_later.clone(),
            active_soon.clone(),
        ];
        quests.sort_by(|a, b| super::compare_quests(a, b, sort));

        let ordered_ids: Vec<&str> = quests.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(
            ordered_ids,
            vec![
                expected_active[0],
                expected_active[1],
                "completed",
                "expired",
                "completed-expired",
                "unsupported",
            ]
        );
    }
}
