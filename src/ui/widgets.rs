use super::theme::*;
use super::Tab;
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
    let out = egui::Frame::default()
        .fill(card_bg())
        .rounding(rnd(14.0))
        .inner_margin(mg(20.0))
        .stroke(st(1.0, border()))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    let r = out.response.rect;
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

pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str, accent: Color32) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(30.0).color(heading()).family(bold()));
    let (bar, _) = ui.allocate_exact_size(Vec2::new(64.0, 4.0), Sense::hover());
    ui.painter().rect_filled(bar, rnd(2.0), accent);
    ui.add_space(4.0);
    ui.label(RichText::new(subtitle).color(muted()).size(14.0));
    ui.add_space(18.0);
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
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 48.0), Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let frac = frac.clamp(0.0, 1.0);
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

pub fn tier_picker(ui: &mut Ui, tiers: &[Tier], selected: &mut String, enabled: bool, accent: Color32) -> bool {
    let avail = ui.available_width();
    let gap = 10.0;
    let cols: usize = if avail >= 540.0 { 5 } else if avail >= 400.0 { 4 } else { 3 };
    let tile_w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    let mut changed = false;

    for row in tiers.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for tier in row {
                let sense = if enabled { Sense::click() } else { Sense::hover() };
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(tile_w, 66.0), sense);
                let sel = selected.as_str() == tier.key;
                let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), enabled && resp.hovered(), 0.1);
                let dim = if enabled { 1.0 } else { 0.55 };

                let fill = if sel {
                    accent.gamma_multiply(0.20)
                } else {
                    card_inner().lerp_to_gamma(hover_bg(), hv)
                };
                let stroke = if sel { st(1.6, accent) } else { st(1.0, border().lerp_to_gamma(accent, 0.5 * hv)) };
                let p = ui.painter();
                p.rect_filled(rect, rnd(12.0), fill);
                p.rect_stroke(rect, rnd(12.0), stroke);
                let head_col = if sel { accent } else { faint() };
                p.text(pos2(rect.left() + 12.0, rect.top() + 14.0), Align2::LEFT_CENTER, &tier.head, font(11.5), head_col.gamma_multiply(dim));
                p.text(pos2(rect.left() + 12.0, rect.top() + 33.0), Align2::LEFT_CENTER, tier.title, font_bold(14.0), heading().gamma_multiply(dim));
                p.text(pos2(rect.left() + 12.0, rect.top() + 52.0), Align2::LEFT_CENTER, &tier.sub, font(12.0), muted().gamma_multiply(dim));

                if enabled && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel {
                    *selected = tier.key.to_string();
                    changed = true;
                }
            }
        });
    }
    changed
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

pub fn draw_tab_icon(p: &Painter, tab: Tab, c: Pos2, color: Color32) {
    let at = |dx: f32, dy: f32| pos2(c.x + dx, c.y + dy);
    match tab {
        Tab::Quest => {
            p.add(egui::Shape::convex_polygon(vec![at(0.0, -9.0), at(8.0, 0.0), at(0.0, 9.0), at(-8.0, 0.0)], color, egui::Stroke::NONE));
        }
        Tab::Badge => {
            p.circle_stroke(c, 8.0, st(2.0, color));
            p.circle_filled(c, 3.0, color);
        }
        Tab::HypeSquad => {
            p.add(egui::Shape::convex_polygon(vec![at(0.0, -9.0), at(9.0, 7.0), at(-9.0, 7.0)], color, egui::Stroke::NONE));
        }
        Tab::Stats => {
            for (i, h) in [6.0_f32, 12.0, 9.0].iter().enumerate() {
                let x = c.x - 7.0 + i as f32 * 7.0;
                p.rect_filled(Rect::from_min_max(pos2(x - 2.5, c.y + 8.0 - h), pos2(x + 2.5, c.y + 8.0)), rnd(1.5), color);
            }
        }
        Tab::Settings => {
            for (i, knob) in [(-6.0, -3.0), (0.0, 4.0), (6.0, -1.0)].iter().enumerate() {
                let y = c.y + knob.0;
                p.line_segment([pos2(c.x - 9.0, y), pos2(c.x + 9.0, y)], st(1.8, color));
                let _ = i;
                p.circle_filled(pos2(c.x + knob.1, y), 2.8, color);
            }
        }
    }
}

pub const NAV_H: f32 = 44.0;
pub const NAV_GAP: f32 = 4.0;

pub fn nav_item(ui: &mut Ui, tab: Tab, selected: bool, busy: bool) -> Response {
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

    let color = if selected { heading() } else { muted().lerp_to_gamma(heading(), hv) };
    let icon_color = if selected { accent } else { muted().lerp_to_gamma(accent, hv) };
    draw_tab_icon(ui.painter(), tab, pos2(body.left() + 26.0, body.center().y), icon_color);
    ui.painter().text(
        pos2(body.left() + 48.0, body.center().y),
        Align2::LEFT_CENTER,
        tab.label(),
        FontId::new(15.0, if selected { bold() } else { FontFamily::Proportional }),
        color,
    );
    if busy {
        let t = ui.input(|i| i.time) as f32;
        let k = 0.5 + 0.5 * (t * 4.0).sin();
        let c = pos2(body.right() - 16.0, body.center().y);
        ui.painter().circle_filled(c, 3.5 + 3.0 * k, GREEN.gamma_multiply(0.22 * (1.0 - k)));
        ui.painter().circle_filled(c, 3.5, GREEN);
    }
    resp
}

pub fn logo_block(ui: &mut Ui, compact: bool, tex: Option<TextureId>) {
    let h = if compact { 66.0 } else { 76.0 };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let p = ui.painter();
    let icon = if compact { 46.0 } else { 54.0 };
    let icon_rect = Rect::from_min_size(pos2(rect.left() + 20.0, rect.center().y - icon / 2.0), Vec2::splat(icon));
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

pub fn account_card(ui: &mut Ui, name: Option<String>, tex: Option<TextureId>, accent: Color32) -> Response {
    let w = (ui.available_width() - 20.0).max(60.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 58.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), resp.hovered(), 0.12);
    let p = ui.painter();
    p.rect_filled(rect, rnd(12.0), card_inner().lerp_to_gamma(hover_bg(), hv));
    p.rect_stroke(rect, rnd(12.0), st(1.0, border().lerp_to_gamma(accent, 0.5 * hv)));
    let c = pos2(rect.left() + 28.0, rect.center().y);
    paint_avatar(p, c, 36.0, tex, name.as_deref(), accent);

    let logged = name.is_some();
    let shown = match name {
        Some(n) if n.chars().count() > 14 => n.chars().take(13).collect::<String>() + "…",
        Some(n) => n,
        None => "Khách".to_string(),
    };
    p.text(pos2(rect.left() + 54.0, rect.center().y - 8.0), Align2::LEFT_CENTER, shown, font_bold(13.5), heading());
    let (txt, col) = if logged { ("Đã đăng nhập", GREEN) } else { ("Chưa đăng nhập", muted()) };
    p.text(pos2(rect.left() + 54.0, rect.center().y + 10.0), Align2::LEFT_CENTER, txt, font(11.5), col);
    resp
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

pub fn bar_chart(ui: &mut Ui, data: &[(String, u32)], accent: Color32) {
    let w = ui.available_width();
    let h = 150.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let p = ui.painter();
    let plot = Rect::from_min_max(pos2(rect.left() + 28.0, rect.top() + 18.0), pos2(rect.right(), rect.bottom() - 22.0));
    let max = data.iter().map(|d| d.1).max().unwrap_or(0).max(4);
    for i in 0..=2 {
        let y = plot.bottom() - plot.height() * (i as f32 / 2.0);
        p.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], st(1.0, border().gamma_multiply(0.6)));
        let label = ((max as f32) * (i as f32 / 2.0)).round() as u32;
        p.text(pos2(rect.left(), y), Align2::LEFT_CENTER, label.to_string(), font(10.5), faint());
    }
    let n = data.len().max(1) as f32;
    let slot = plot.width() / n;
    let bar_w = (slot * 0.56).min(26.0);
    for (i, (label, v)) in data.iter().enumerate() {
        let cx = plot.left() + slot * (i as f32 + 0.5);
        let bh = plot.height() * (*v as f32 / max as f32);
        let bar = Rect::from_min_max(pos2(cx - bar_w / 2.0, plot.bottom() - bh.max(2.0)), pos2(cx + bar_w / 2.0, plot.bottom()));
        let is_today = i + 1 == data.len();
        let col = if *v == 0 { border() } else if is_today { accent } else { accent.gamma_multiply(0.62) };
        p.rect_filled(bar, rnd(4.0), col);
        if *v > 0 {
            p.text(pos2(cx, bar.top() - 7.0), Align2::CENTER_CENTER, v.to_string(), font_bold(11.0), text_c());
        }
        if i % 2 == (data.len() + 1) % 2 || data.len() <= 7 {
            p.text(pos2(cx, rect.bottom() - 8.0), Align2::CENTER_CENTER, label, font(10.5), if is_today { text_c() } else { faint() });
        }
    }
}
