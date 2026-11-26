use super::*;
use crate::ui::fx;
use egui::{FontFamily, FontId, Painter};

fn f(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
fn fb(size: f32) -> FontId {
    FontId::new(size, bold())
}
fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

fn accordion(ui: &mut egui::Ui, key: &str, title: &str, sub: &str, accent: Color32, default_open: bool, body: impl FnOnce(&mut egui::Ui)) {
    let id = ui.make_persistent_id(key);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if resp.clicked() {
        state.toggle(ui);
    }
    let open = state.openness(ui.ctx());
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);
    ui.painter().add(fx::panel_shape(rect, 12.0, card_bg(), hv));
    let p = ui.painter();
    p.text(pos2(rect.left() + 18.0, rect.top() + 19.0), Align2::LEFT_CENTER, title, fb(14.5), heading());
    p.text(pos2(rect.left() + 18.0, rect.top() + 38.0), Align2::LEFT_CENTER, sub, f(12.0), muted());
    let c = pos2(rect.right() - 24.0, rect.center().y);
    let a = open * std::f32::consts::FRAC_PI_2;
    let rot = |x: f32, y: f32| pos2(c.x + x * a.cos() - y * a.sin(), c.y + x * a.sin() + y * a.cos());
    let col = muted().lerp_to_gamma(accent, 0.4 + 0.6 * open);
    p.line_segment([rot(-3.0, -5.0), rot(3.0, 0.0)], st(2.0, col));
    p.line_segment([rot(3.0, 0.0), rot(-3.0, 5.0)], st(2.0, col));
    state.show_body_unindented(ui, |ui| {
        ui.add_space(10.0);
        body(ui);
    });
    state.store(ui.ctx());
}

fn step(ui: &mut egui::Ui, n: usize, text: &str, accent: Color32, illus: impl FnOnce(&Painter, Rect, Color32)) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let (rect, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 13.0, accent.gamma_multiply(0.22));
        ui.painter().circle_stroke(rect.center(), 13.0, st(1.0, accent.gamma_multiply(0.7)));
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, n.to_string(), fb(13.0), accent);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.add(egui::Label::new(RichText::new(text).size(13.5).color(text_c())).wrap());
            let w = ui.available_width().min(520.0);
            let (area, _) = ui.allocate_exact_size(vec2(w, 96.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(area, rnd(10.0), bg1());
            p.rect_stroke(area, rnd(10.0), st(1.0, border()));
            let clipped = p.with_clip_rect(area.shrink(1.0));
            illus(&clipped, area.shrink(8.0), accent);
        });
    });
    ui.add_space(10.0);
}

fn pill(p: &Painter, rect: Rect, fill: Color32) {
    p.rect_filled(rect, rnd(rect.height() / 2.0), fill);
}

fn illus_browser(p: &Painter, r: Rect, accent: Color32) {
    let bar = Rect::from_min_size(r.min, vec2(r.width(), 20.0));
    p.rect_filled(bar, rnd(6.0), card_inner());
    for (i, c) in [RED, ACCENT_AMBER, GREEN].iter().enumerate() {
        p.circle_filled(pos2(bar.left() + 10.0 + i as f32 * 12.0, bar.center().y), 3.5, *c);
    }
    let url = Rect::from_min_max(pos2(bar.left() + 52.0, bar.top() + 4.0), pos2(bar.right() - 10.0, bar.bottom() - 4.0));
    pill(p, url, bg3());
    p.text(pos2(url.left() + 10.0, url.center().y), Align2::LEFT_CENTER, "discord.com/app", f(11.0), text_c());
    p.circle_filled(pos2(url.right() - 10.0, url.center().y), 3.0, accent);
    let body = Rect::from_min_max(pos2(r.left(), bar.bottom() + 6.0), r.max);
    p.rect_filled(Rect::from_min_size(body.min, vec2(34.0, body.height())), rnd(6.0), card_inner());
    for i in 0..3 {
        p.circle_filled(pos2(body.left() + 17.0, body.top() + 12.0 + i as f32 * 20.0), 7.0, accent.gamma_multiply(0.5 - 0.1 * i as f32));
    }
    for i in 0..3 {
        let y = body.top() + 10.0 + i as f32 * 18.0;
        p.circle_filled(pos2(body.left() + 52.0, y + 3.0), 6.0, border());
        pill(p, Rect::from_min_size(pos2(body.left() + 64.0, y - 1.0), vec2(70.0 + 40.0 * (i % 2) as f32, 5.0)), border());
        pill(p, Rect::from_min_size(pos2(body.left() + 64.0, y + 7.0), vec2(150.0 - 30.0 * (i % 2) as f32, 5.0)), border().gamma_multiply(0.6));
    }
}

fn illus_devtools_tabs(p: &Painter, r: Rect, accent: Color32) {
    let top = Rect::from_min_size(r.min, vec2(r.width(), 40.0));
    p.rect_filled(top, rnd(6.0), card_inner());
    pill(p, Rect::from_min_size(top.min + vec2(10.0, 12.0), vec2(120.0, 6.0)), border());
    pill(p, Rect::from_min_size(top.min + vec2(10.0, 24.0), vec2(80.0, 6.0)), border().gamma_multiply(0.6));
    let tabs_y = top.bottom() + 6.0;
    let tabs = ["Elements", "Console", "Sources", "Network"];
    let mut x = r.left() + 6.0;
    for (i, name) in tabs.iter().enumerate() {
        let w = 14.0 + name.len() as f32 * 6.2;
        let rc = Rect::from_min_size(pos2(x, tabs_y), vec2(w, 22.0));
        if i == 3 {
            p.rect_filled(rc, rnd(6.0), accent.gamma_multiply(0.22));
            p.rect_stroke(rc, rnd(6.0), st(1.2, accent));
        }
        p.text(rc.center(), Align2::CENTER_CENTER, *name, f(11.0), if i == 3 { heading() } else { muted() });
        x += w + 6.0;
    }
    let k = Rect::from_min_size(pos2(r.right() - 66.0, tabs_y), vec2(60.0, 22.0));
    p.rect_filled(k, rnd(6.0), card_inner());
    p.rect_stroke(k, rnd(6.0), st(1.0, border()));
    p.text(k.center(), Align2::CENTER_CENTER, "F12 · F5", mono(10.5), text_c());
}

fn illus_requests(p: &Painter, r: Rect, accent: Color32) {
    let list = Rect::from_min_size(r.min, vec2(r.width() * 0.52, r.height()));
    p.rect_filled(list, rnd(6.0), card_inner());
    let rows = ["api/v9/users/@me", "api/v9/science", "api/v9/quests/@me", "api/v9/channels"];
    for (i, name) in rows.iter().enumerate() {
        let y = list.top() + 6.0 + i as f32 * 20.0;
        let row = Rect::from_min_size(pos2(list.left() + 4.0, y), vec2(list.width() - 8.0, 18.0));
        if i == 1 {
            p.rect_filled(row, rnd(5.0), accent.gamma_multiply(0.22));
            p.rect_stroke(row, rnd(5.0), st(1.2, accent));
        }
        p.circle_filled(pos2(row.left() + 8.0, row.center().y), 3.0, if i == 1 { accent } else { GREEN.gamma_multiply(0.8) });
        p.text(pos2(row.left() + 18.0, row.center().y), Align2::LEFT_CENTER, *name, mono(10.5), if i == 1 { heading() } else { muted() });
    }
    let pane = Rect::from_min_max(pos2(list.right() + 6.0, r.top()), r.max);
    p.rect_filled(pane, rnd(6.0), card_inner());
    for (i, (name, on)) in [("Headers", true), ("Payload", false), ("Preview", false)].iter().enumerate() {
        let x = pane.left() + 8.0 + i as f32 * 52.0;
        p.text(pos2(x, pane.top() + 12.0), Align2::LEFT_CENTER, *name, f(10.5), if *on { heading() } else { faint() });
        if *on {
            p.line_segment([pos2(x, pane.top() + 22.0), pos2(x + 40.0, pane.top() + 22.0)], st(2.0, accent));
        }
    }
    for i in 0..3 {
        pill(p, Rect::from_min_size(pos2(pane.left() + 8.0, pane.top() + 32.0 + i as f32 * 14.0), vec2(80.0 - 14.0 * i as f32, 5.0)), border());
    }
}

fn illus_headers(p: &Painter, r: Rect, accent: Color32) {
    p.rect_filled(r, rnd(6.0), card_inner());
    p.text(pos2(r.left() + 10.0, r.top() + 12.0), Align2::LEFT_CENTER, "Request Headers", fb(11.0), heading());
    let rows = [("accept", "*/*"), ("authorization", "••••••••"), ("cookie", "__dcfduid=a1b2…; __sdcfduid=c3d4…"), ("user-agent", "Mozilla/5.0 …")];
    for (i, (k, v)) in rows.iter().enumerate() {
        let y = r.top() + 30.0 + i as f32 * 15.0;
        let hot = *k == "cookie";
        if hot {
            let hl = Rect::from_min_size(pos2(r.left() + 4.0, y - 7.0), vec2(r.width() - 8.0, 15.0));
            p.rect_filled(hl, rnd(4.0), accent.gamma_multiply(0.22));
            p.rect_stroke(hl, rnd(4.0), st(1.2, accent));
        }
        p.text(pos2(r.left() + 10.0, y), Align2::LEFT_CENTER, format!("{}:", k), mono(10.5), if hot { accent } else { muted() });
        p.text(pos2(r.left() + 104.0, y), Align2::LEFT_CENTER, *v, mono(10.0), if hot { heading() } else { faint() });
    }
    let c = pos2(r.right() - 14.0, r.top() + 14.0);
    p.rect_stroke(Rect::from_center_size(c + vec2(2.0, 2.0), vec2(10.0, 12.0)), rnd(2.0), st(1.3, muted()));
    p.rect_filled(Rect::from_center_size(c - vec2(1.0, 1.0), vec2(10.0, 12.0)), rnd(2.0), accent.gamma_multiply(0.5));
}

fn illus_login(p: &Painter, r: Rect, accent: Color32) {
    let field = Rect::from_min_size(r.min + vec2(0.0, 18.0), vec2(r.width() * 0.62, 30.0));
    p.rect_filled(field, rnd(8.0), card_inner());
    p.rect_stroke(field, rnd(8.0), st(1.0, border()));
    for i in 0..14 {
        p.circle_filled(pos2(field.left() + 14.0 + i as f32 * 9.0, field.center().y), 2.4, muted());
    }
    let btn = Rect::from_min_size(pos2(field.right() + 10.0, field.top()), vec2(r.right() - field.right() - 10.0, 30.0));
    p.rect_filled(btn, rnd(8.0), accent);
    p.text(btn.center(), Align2::CENTER_CENTER, "✓", fb(14.0), Color32::WHITE);
    p.circle_filled(pos2(r.left() + 12.0, r.bottom() - 14.0), 5.0, GREEN);
    p.text(pos2(r.left() + 24.0, r.bottom() - 14.0), Align2::LEFT_CENTER, "Đã đăng nhập / Signed in", f(11.0), GREEN);
}

fn illus_drop(p: &Painter, r: Rect, accent: Color32) {
    let win = Rect::from_min_max(pos2(r.left() + r.width() * 0.46, r.top()), r.max);
    p.rect_filled(win, rnd(8.0), card_inner());
    p.rect_stroke(win, rnd(8.0), st(1.6, accent.gamma_multiply(0.8)));
    p.text(win.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, ".exe", fb(15.0), accent);
    p.text(win.center() + vec2(0.0, 10.0), Align2::CENTER_CENTER, "drop", f(11.0), muted());
    let file = Rect::from_center_size(pos2(r.left() + 34.0, r.center().y), vec2(36.0, 46.0));
    p.rect_filled(file, rnd(5.0), card_bg());
    p.rect_stroke(file, rnd(5.0), st(1.2, muted()));
    p.text(file.center(), Align2::CENTER_CENTER, "game\n.exe", mono(9.5), text_c());
    let (a, b) = (pos2(file.right() + 8.0, r.center().y), pos2(win.left() - 10.0, r.center().y));
    p.line_segment([a, b], st(2.0, accent));
    p.line_segment([b, b + vec2(-6.0, -5.0)], st(2.0, accent));
    p.line_segment([b, b + vec2(-6.0, 5.0)], st(2.0, accent));
}

fn illus_result(p: &Painter, r: Rect, accent: Color32) {
    p.text(pos2(r.left() + 2.0, r.top() + 12.0), Align2::LEFT_CENTER, "Fingerprint", f(11.5), text_c());
    let field = Rect::from_min_size(r.min + vec2(0.0, 26.0), vec2(r.width() * 0.7, 28.0));
    p.rect_filled(field, rnd(8.0), card_inner());
    p.rect_stroke(field, rnd(8.0), st(1.2, accent));
    p.text(pos2(field.left() + 10.0, field.center().y), Align2::LEFT_CENTER, "9f3a1c7e5b02d84a…", mono(11.0), heading());
    let ok = pos2(field.right() + 22.0, field.center().y);
    p.circle_filled(ok, 11.0, GREEN.gamma_multiply(0.22));
    p.line_segment([ok + vec2(-4.0, 0.0), ok + vec2(-1.0, 3.5)], st(2.0, GREEN));
    p.line_segment([ok + vec2(-1.0, 3.5), ok + vec2(5.0, -3.5)], st(2.0, GREEN));
}

impl JokerApp {
    pub(super) fn render_badge_guides(&mut self, ui: &mut egui::Ui, accent: Color32, cookie_missing: bool) {
        accordion(ui, "guide_cookie", t("badge.guide_cookie"), t("badge.guide_cookie_sub"), accent, cookie_missing, |ui| {
            step(ui, 1, t("guide.ck1"), accent, illus_browser);
            step(ui, 2, t("guide.ck2"), accent, illus_devtools_tabs);
            step(ui, 3, t("guide.ck3"), accent, illus_requests);
            step(ui, 4, t("guide.ck4"), accent, illus_headers);
            egui::Frame::default().fill(RED.gamma_multiply(0.10)).stroke(st(1.0, RED.gamma_multiply(0.5))).rounding(rnd(10.0)).inner_margin(mgs(12.0, 8.0)).show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(t("guide.ck_warn")).size(12.5).color(text_c())).wrap());
            });
        });
        ui.add_space(8.0);
        accordion(ui, "guide_fp", t("badge.guide_fp"), t("badge.guide_fp_sub"), accent, false, |ui| {
            step(ui, 1, t("guide.fp1"), accent, illus_login);
            step(ui, 2, t("guide.fp2"), accent, illus_drop);
            step(ui, 3, t("guide.fp3"), accent, illus_result);
        });
    }
}
