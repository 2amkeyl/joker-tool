use super::fx;
use super::theme::*;
use super::{NavFlag, Tab};
use eframe::egui::{
    self, pos2, Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Response, RichText, Sense, TextureId, Ui, Vec2,
};

fn uv() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
}

fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

fn font_bold(size: f32) -> FontId {
    FontId::new(size, bold())
}

pub fn card<R>(ui: &mut Ui, accent: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let slot = ui.painter().add(egui::Shape::Noop);
    let out = egui::Frame::default().inner_margin(mg(20.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
    let r = out.response.rect;
    ui.painter().set(slot, fx::panel_shape(r, 14.0, card_bg(), 0.0));
    ui.painter().rect_filled(Rect::from_min_size(r.min + Vec2::new(20.0, 0.0), Vec2::new(48.0, 3.0)), rnd(2.0), accent);
    out.inner
}

pub fn card_title(ui: &mut Ui, title: &str, sub: &str) {
    ui.label(RichText::new(title).size(15.0).color(heading()).family(bold()));
    if !sub.is_empty() {
        ui.label(RichText::new(sub).size(12.5).color(muted()));
    }
    ui.add_space(8.0);
}

pub fn page_header(ui: &mut Ui, subtitle: &str, accent: Color32) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let (bar, _) = ui.allocate_exact_size(Vec2::new(4.0, 18.0), Sense::hover());
        ui.painter().rect_filled(bar, rnd(2.0), accent);
        ui.label(RichText::new(subtitle).color(text_c().gamma_multiply(0.85)).size(14.0));
    });
    ui.add_space(16.0);
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(muted()));
}

pub fn primary_button(ui: &mut Ui, label: &str, fill: Color32, enabled: bool) -> Response {
    let w = ui.available_width();
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 48.0), sense);
    let hover_t = ui.ctx().animate_bool_with_time(resp.id.with("hv"), enabled && resp.hovered(), 0.12);
    let pressed = enabled && resp.is_pointer_button_down_on();

    let base = if enabled { fill } else { disabled_bg() };
    let mut col = base.lerp_to_gamma(Color32::WHITE, 0.14 * hover_t);
    if pressed {
        col = base.lerp_to_gamma(Color32::BLACK, 0.18);
    }
    let fg = if enabled { Color32::WHITE } else { muted() };

    let p = ui.painter();
    if enabled && hover_t > 0.0 {
        p.rect_filled(rect.expand(3.0 * hover_t), rnd(14.0), fill.gamma_multiply(0.22 * hover_t));
    }
    p.rect_filled(rect, rnd(12.0), col);
    p.text(rect.center(), Align2::CENTER_CENTER, label, font_bold(15.5), fg);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

pub fn pulse_dot(ui: &mut Ui, color: Color32) {
    let t = ui.input(|i| i.time) as f32;
    let k = 0.5 + 0.5 * (t * 4.0).sin();
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
    let c = rect.center();
    ui.painter().circle_filled(c, 3.5 + 3.0 * k, color.gamma_multiply(0.25 * (1.0 - k)));
    ui.painter().circle_filled(c, 3.5, color);
}

pub fn stat_chip(ui: &mut Ui, w: f32, label: &str, value: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 62.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, rnd(12.0), card_inner());
    p.rect_stroke(rect, rnd(12.0), st(1.0, border()));
    p.rect_filled(Rect::from_min_size(rect.min + Vec2::new(0.0, 14.0), Vec2::new(3.0, 34.0)), rnd(2.0), color);
    p.text(pos2(rect.left() + 16.0, rect.top() + 20.0), Align2::LEFT_CENTER, label, font(11.5), muted());
    p.text(pos2(rect.left() + 16.0, rect.top() + 42.0), Align2::LEFT_CENTER, value, font_bold(19.0), color);
}

fn gradient_pill(painter: &Painter, rect: Rect, c0: Color32, c1: Color32) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let r = (rect.height() / 2.0).min(rect.width() / 2.0);
    let seg = 10;
    let cy = rect.center().y;
    let mut pts: Vec<Pos2> = Vec::new();
    for i in 0..=seg {
        let a = -FRAC_PI_2 + PI * (i as f32 / seg as f32);
        pts.push(pos2(rect.right() - r + r * a.cos(), cy + r * a.sin()));
    }
    for i in 0..=seg {
        let a = FRAC_PI_2 + PI * (i as f32 / seg as f32);
        pts.push(pos2(rect.left() + r + r * a.cos(), cy + r * a.sin()));
    }
    let color_at = |x: f32| c0.lerp_to_gamma(c1, ((x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0));
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.center(), color_at(rect.center().x));
    for p in &pts {
        mesh.colored_vertex(*p, color_at(p.x));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + ((i + 1) % n));
    }
    painter.add(egui::Shape::mesh(mesh));
    painter.add(egui::Shape::closed_line(pts, st(1.0, c1.gamma_multiply(0.55))));
}

pub fn progress_bar(ui: &mut Ui, frac: f32, text: &str, accent: Color32, animate: bool) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 48.0), Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let target = frac.clamp(0.0, 1.0);
    let frac = ui.ctx().animate_value_with_time(resp.id.with("pb"), target, 0.3);
    let p = ui.painter();

    p.text(pos2(rect.left(), rect.top() + 10.0), Align2::LEFT_CENTER, text, font(13.0), text_c());
    p.text(pos2(rect.right(), rect.top() + 10.0), Align2::RIGHT_CENTER, format!("{:.0}%", frac * 100.0), font_bold(15.0), accent);

    let h = 18.0;
    let track = Rect::from_min_max(pos2(rect.left(), rect.bottom() - h), pos2(rect.right(), rect.bottom()));
    p.rect_filled(track, rnd(h / 2.0), bg1());
    p.rect_stroke(track, rnd(h / 2.0), st(1.0, border()));

    if frac > 0.0 {
        let inner = track.shrink(2.5);
        let fw = (inner.width() * frac).max(inner.height());
        let fill = Rect::from_min_size(inner.min, Vec2::new(fw, inner.height()));
        let pulse = if animate { 0.5 + 0.5 * (t * 3.0).sin() } else { 0.0 };

        let r = fill.height() / 2.0;
        let tip = pos2(fill.right() - r, fill.center().y);
        p.circle_filled(tip, r * 2.4, accent.gamma_multiply(0.10 + 0.10 * pulse));
        p.circle_filled(tip, r * 1.6, accent.gamma_multiply(0.14 + 0.10 * pulse));

        let c0 = accent.lerp_to_gamma(Color32::BLACK, 0.28);
        let c1 = accent.lerp_to_gamma(Color32::WHITE, 0.22);
        gradient_pill(p, fill, c0, c1);

        if animate && fill.width() > 24.0 {
            let sp = p.with_clip_rect(fill.shrink2(Vec2::new(r * 0.5, 0.0)));
            let band = fill.height() * 4.0 + 28.0;
            let cycle = (t % 1.8) / 1.8;
            let x = fill.left() - band + (fill.width() + band) * cycle;
            let (top, bot) = (fill.top(), fill.bottom());
            let clear = Color32::from_white_alpha(0);
            let glow = Color32::from_white_alpha(62);
            let mut mesh = egui::Mesh::default();
            for (px, py, c) in [(x, top, clear), (x, bot, clear), (x + band / 2.0, top, glow), (x + band / 2.0, bot, glow), (x + band, top, clear), (x + band, bot, clear)] {
                mesh.colored_vertex(pos2(px, py), c);
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(1, 3, 2);
            mesh.add_triangle(2, 3, 4);
            mesh.add_triangle(3, 5, 4);
            sp.add(egui::Shape::mesh(mesh));
        }

        let sheen = Rect::from_min_size(fill.min + Vec2::new(r * 0.7, 2.0), Vec2::new((fw - r * 1.4).max(0.0), 3.0));
        p.rect_filled(sheen, rnd(1.5), Color32::from_white_alpha(38));
    }
}

pub struct Tier {
    pub key: &'static str,
    pub head: String,
    pub title: &'static str,
    pub sub: String,
}

pub fn empty_state(ui: &mut Ui, accent: Color32, title: &str, body: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(14.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 64.0), Sense::hover());
        let c = rect.center();
        let p = ui.painter();
        p.circle_filled(c, 30.0, accent.gamma_multiply(0.10));
        p.circle_stroke(c, 30.0, st(1.0, accent.gamma_multiply(0.45)));
        p.add(egui::Shape::convex_polygon(
            vec![pos2(c.x, c.y - 13.0), pos2(c.x + 11.0, c.y), pos2(c.x, c.y + 13.0), pos2(c.x - 11.0, c.y)],
            accent.gamma_multiply(0.85),
            egui::Stroke::NONE,
        ));
        ui.add_space(8.0);
        ui.label(RichText::new(title).size(15.0).color(heading()).family(bold()));
        ui.label(RichText::new(body).size(12.5).color(muted()));
        ui.add_space(10.0);
    });
}

pub const NAV_H: f32 = 44.0;
pub const NAV_GAP: f32 = 4.0;

pub fn nav_item(ui: &mut Ui, tab: Tab, selected: bool, busy: bool, flag: NavFlag) -> Response {
    let size = Vec2::new(ui.available_width(), NAV_H);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let body = rect.shrink2(Vec2::new(10.0, 0.0));
    let accent = tab.accent();

    let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), resp.hovered() && !selected, 0.12);
    if hv > 0.0 {
        let tint = if is_light() { Color32::from_black_alpha((20.0 * hv) as u8) } else { Color32::from_white_alpha((30.0 * hv) as u8) };
        ui.painter().rect_filled(body, rnd(10.0), tint);
        ui.painter().rect_filled(body, rnd(10.0), accent.gamma_multiply(0.10 * hv));
    }

    let color = if selected { heading() } else { muted().lerp_to_gamma(heading(), 0.35 + 0.65 * hv) };
    let icon_color = if selected { accent } else { muted().lerp_to_gamma(heading(), 0.35).lerp_to_gamma(accent, hv) };
    let icon_x = body.left() + 27.0;
    let tile = Rect::from_center_size(pos2(icon_x, body.center().y), Vec2::splat(36.0));
    let tile_fill = if selected {
        accent.gamma_multiply(0.26)
    } else if is_light() {
        Color32::from_black_alpha((12.0 + 14.0 * hv) as u8)
    } else {
        Color32::from_white_alpha((14.0 + 14.0 * hv) as u8)
    };
    ui.painter().rect_filled(tile, rnd(11.0), tile_fill);
    let family = if selected { FontFamily::Name("ph_fill".into()) } else { FontFamily::Proportional };
    ui.painter().text(tile.center(), Align2::CENTER_CENTER, tab.icon(), FontId::new(22.0, family), icon_color);
    ui.painter().text(
        pos2(body.left() + 54.0, body.center().y),
        Align2::LEFT_CENTER,
        tab.label(),
        FontId::new(15.0, if selected { bold() } else { FontFamily::Proportional }),
        color,
    );

    let marker = if busy {
        Some((GREEN, true))
    } else {
        match flag {
            NavFlag::Error => Some((RED, false)),
            NavFlag::New => Some((ACCENT_AMBER, false)),
            NavFlag::None => None,
        }
    };
    if let Some((col, pulse)) = marker {
        let c = pos2(body.right() - 16.0, body.center().y);
        let k = if pulse { 0.5 + 0.5 * (ui.input(|i| i.time) as f32 * 4.0).sin() } else { 0.0 };
        if pulse {
            ui.painter().circle_filled(c, 3.5 + 3.0 * k, col.gamma_multiply(0.22 * (1.0 - k)));
        } else {
            ui.painter().circle_filled(c, 6.0, col.gamma_multiply(0.20));
        }
        ui.painter().circle_filled(c, 3.6, col);
        ui.painter().circle_stroke(c, 3.6, st(1.2, bg2()));
    }
    resp
}

pub fn logo_block(ui: &mut Ui, compact: bool, tex: Option<TextureId>) {
    let h = if compact { 66.0 } else { 76.0 };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    let icon = if compact { 46.0 } else { 54.0 };
    let icon_left = rect.left() + 20.0;
    let icon_rect = Rect::from_min_size(pos2(icon_left, rect.center().y - icon / 2.0), Vec2::splat(icon));
    let round = rnd(icon * 0.24);

    p.rect_filled(icon_rect.expand(4.0), rnd(icon * 0.30), brand().gamma_multiply(0.28));
    match tex {
        Some(id) => {
            p.image(id, icon_rect, uv(), Color32::WHITE);
        }
        None => {
            p.rect_filled(icon_rect, round, brand());
        }
    }
    p.rect_stroke(icon_rect, round, st(1.5, brand()));

    let tx = icon_rect.right() + 14.0;
    p.text(pos2(tx, rect.center().y - 11.0), Align2::LEFT_CENTER, "JOKER", font_bold(if compact { 21.0 } else { 24.0 }), heading());
    p.text(pos2(tx + 1.0, rect.center().y + 13.0), Align2::LEFT_CENTER, "T O O L", font_bold(12.0), brand());
}

pub fn paint_avatar(p: &Painter, c: Pos2, size: f32, tex: Option<TextureId>, name: Option<&str>, accent: Color32) {
    let r = size / 2.0;
    match (tex, name) {
        (Some(id), Some(_)) => {
            p.image(id, Rect::from_center_size(c, Vec2::splat(size - 2.0)), uv(), Color32::WHITE);
            p.circle_stroke(c, r, st(1.5, accent));
        }
        (_, Some(n)) => {
            p.circle_filled(c, r, accent.gamma_multiply(0.30));
            p.circle_stroke(c, r, st(1.5, accent));
            let initial = n.chars().next().map(|ch| ch.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
            p.text(c, Align2::CENTER_CENTER, initial, font_bold(size * 0.44), heading());
        }
        _ => {
            p.circle_filled(c, r, disabled_bg());
            p.circle_stroke(c, r, st(1.5, border()));
            p.text(c, Align2::CENTER_CENTER, "?", font_bold(size * 0.44), muted());
        }
    }
}

pub fn avatar(ui: &mut Ui, size: f32, tex: Option<TextureId>, name: Option<&str>, accent: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_avatar(ui.painter(), rect.center(), size, tex, name, accent);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Ok,
    Err,
    Info,
}

pub fn toast_card(ui: &mut Ui, kind: ToastKind, text: &str, alpha: f32) {
    let color = match kind {
        ToastKind::Ok => GREEN,
        ToastKind::Err => RED,
        ToastKind::Info => ACCENT_BLUE,
    };
    ui.set_opacity(alpha);
    egui::Frame::default()
        .fill(card_bg())
        .rounding(rnd(12.0))
        .stroke(st(1.0, color.gamma_multiply(0.7)))
        .inner_margin(mgs(14.0, 10.0))
        .shadow(egui::Shadow { offset: Vec2::new(0.0, 6.0), blur: 18.0, spread: 0.0, color: Color32::from_black_alpha(120) })
        .show(ui, |ui| {
            ui.set_max_width(320.0);
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                let c = rect.center();
                let p = ui.painter();
                p.circle_filled(c, 9.0, color.gamma_multiply(0.22));
                match kind {
                    ToastKind::Ok => {
                        p.line_segment([pos2(c.x - 4.0, c.y), pos2(c.x - 1.0, c.y + 3.0)], st(1.8, color));
                        p.line_segment([pos2(c.x - 1.0, c.y + 3.0), pos2(c.x + 4.5, c.y - 3.5)], st(1.8, color));
                    }
                    ToastKind::Err => {
                        p.line_segment([pos2(c.x - 3.5, c.y - 3.5), pos2(c.x + 3.5, c.y + 3.5)], st(1.8, color));
                        p.line_segment([pos2(c.x + 3.5, c.y - 3.5), pos2(c.x - 3.5, c.y + 3.5)], st(1.8, color));
                    }
                    ToastKind::Info => {
                        p.circle_filled(c, 2.0, color);
                    }
                }
                ui.label(RichText::new(text).size(13.5).color(text_c()));
            });
        });
}

pub fn tier_slider(ui: &mut Ui, tiers: &[Tier], selected: &mut String, enabled: bool, accent: Color32) -> bool {
    let n = tiers.len();
    if n < 2 {
        return false;
    }
    let cur = tiers.iter().position(|t| t.key == selected.as_str()).unwrap_or(0);
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 104.0), if enabled { Sense::click_and_drag() } else { Sense::hover() });
    let dim = if enabled { 1.0 } else { 0.55 };

    let track_l = rect.left() + 12.0;
    let track_r = rect.right() - 12.0;
    let track_y = rect.top() + 62.0;
    let step = (track_r - track_l) / (n as f32 - 1.0);
    let mut changed = false;

    if enabled && (resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on()) {
        if let Some(pos) = resp.interact_pointer_pos() {
            let idx = (((pos.x - track_l) / step).round().clamp(0.0, n as f32 - 1.0)) as usize;
            if idx != cur {
                *selected = tiers[idx].key.to_string();
                changed = true;
            }
        }
    }
    let cur = tiers.iter().position(|t| t.key == selected.as_str()).unwrap_or(0);
    let knob_t = ui.ctx().animate_value_with_time(resp.id.with("knob"), cur as f32, 0.16);
    let knob_x = track_l + knob_t * step;
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), enabled && (resp.hovered() || resp.dragged()), 0.12);

    let p = ui.painter();
    let tier = &tiers[cur];
    p.text(pos2(rect.left() + 2.0, rect.top() + 14.0), Align2::LEFT_CENTER, &tier.head, font(11.5), accent.gamma_multiply(dim));
    p.text(pos2(rect.left() + 2.0, rect.top() + 36.0), Align2::LEFT_CENTER, tier.title, font_bold(21.0), heading().gamma_multiply(dim));
    p.text(pos2(rect.right() - 2.0, rect.top() + 32.0), Align2::RIGHT_CENTER, &tier.sub, font_bold(18.0), accent.gamma_multiply(dim));

    let th = 6.0;
    let track = Rect::from_min_max(pos2(track_l - 4.0, track_y - th / 2.0), pos2(track_r + 4.0, track_y + th / 2.0));
    p.rect_filled(track, rnd(th / 2.0), card_inner());
    p.rect_stroke(track, rnd(th / 2.0), st(1.0, border()));
    let fill = Rect::from_min_max(track.min, pos2(knob_x.max(track.left() + th), track.max.y));
    p.rect_filled(fill, rnd(th / 2.0), accent.gamma_multiply(0.85 * dim));

    for i in 0..n {
        let x = track_l + i as f32 * step;
        let passed = (i as f32) <= knob_t + 0.01;
        p.circle_filled(pos2(x, track_y), 3.2, if passed { accent.gamma_multiply(dim) } else { border() });
        let is_cur = i == cur;
        let label = (i + 1).to_string();
        p.text(
            pos2(x, track_y + 20.0),
            Align2::CENTER_CENTER,
            label,
            if is_cur { font_bold(12.0) } else { font(11.0) },
            if is_cur { heading().gamma_multiply(dim) } else { faint().gamma_multiply(dim) },
        );
    }
    p.text(pos2(track_l, rect.bottom() - 2.0), Align2::LEFT_BOTTOM, tiers[0].title, font(10.5), faint().gamma_multiply(dim));
    p.text(pos2(track_r, rect.bottom() - 2.0), Align2::RIGHT_BOTTOM, tiers[n - 1].title, font(10.5), faint().gamma_multiply(dim));

    let kc = pos2(knob_x, track_y);
    p.circle_filled(kc, 15.0 + 3.0 * hv, accent.gamma_multiply((0.16 + 0.12 * hv) * dim));
    p.circle_filled(kc, 10.0, Color32::WHITE.gamma_multiply(dim));
    p.circle_filled(kc, 6.5, accent.gamma_multiply(dim));
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    changed
}

pub fn status_pill(ui: &mut Ui, text: &str, color: Color32) {
    let g = ui.painter().layout_no_wrap(text.to_string(), FontId::new(12.0, bold()), color);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(g.size().x + 30.0, 24.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, rnd(12.0), color.gamma_multiply(0.16));
    p.circle_filled(pos2(rect.left() + 12.0, rect.center().y), 3.5, color);
    p.galley(pos2(rect.left() + 22.0, rect.center().y - g.size().y / 2.0), g, color);
}

pub fn account_chip(ui: &mut Ui, name: &str, tex: Option<TextureId>, accent: Color32, open: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(210.0, 44.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), resp.hovered() || open, 0.12);
    let p = ui.painter();
    p.rect_filled(rect, rnd(22.0), card_inner().lerp_to_gamma(hover_bg(), hv));
    p.rect_stroke(rect, rnd(22.0), st(1.0, border().lerp_to_gamma(accent, if open { 0.8 } else { 0.35 * hv })));
    paint_avatar(p, pos2(rect.left() + 24.0, rect.center().y), 32.0, tex, Some(name), accent);
    let shown: String = if name.chars().count() > 14 { name.chars().take(13).collect::<String>() + "…" } else { name.to_string() };
    p.text(pos2(rect.left() + 48.0, rect.center().y - 8.0), Align2::LEFT_CENTER, shown, font_bold(13.5), heading());
    p.circle_filled(pos2(rect.left() + 52.0, rect.center().y + 9.0), 3.0, GREEN);
    p.text(pos2(rect.left() + 60.0, rect.center().y + 9.0), Align2::LEFT_CENTER, "Đã đăng nhập", font(11.0), GREEN);
    let caret = if open { egui_phosphor::regular::CARET_UP } else { egui_phosphor::regular::CARET_DOWN };
    p.text(pos2(rect.right() - 20.0, rect.center().y), Align2::CENTER_CENTER, caret, FontId::new(15.0, FontFamily::Proportional), muted().lerp_to_gamma(heading(), hv));
    resp
}

pub fn menu_row(ui: &mut Ui, icon: &str, label: &str, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.1);
    let p = ui.painter();
    p.rect_filled(rect, rnd(10.0), color.gamma_multiply(0.14 * hv).lerp_to_gamma(hover_bg(), 0.0));
    if hv > 0.0 {
        p.rect_filled(rect, rnd(10.0), hover_bg().gamma_multiply(hv));
    }
    p.text(pos2(rect.left() + 22.0, rect.center().y), Align2::CENTER_CENTER, icon, FontId::new(19.0, FontFamily::Proportional), color);
    p.text(pos2(rect.left() + 46.0, rect.center().y), Align2::LEFT_CENTER, label, font(14.0), if color == text_c() { text_c() } else { color });
    resp
}
