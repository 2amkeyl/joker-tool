
use super::theme::{self, rnd};
use eframe::egui::{pos2, vec2, Color32, Mesh, Painter, Pos2, Rect, Shape};
pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn panel_shape(rect: Rect, radius: f32, base: Color32, lift: f32) -> Shape {
    let border = theme::border().lerp_to_gamma(theme::muted(), (0.55 * lift).clamp(0.0, 1.0));
    Shape::Vec(vec![
        Shape::rect_filled(rect, rnd(radius), base),
        Shape::rect_stroke(rect, rnd(radius), theme::st(1.0, border)),
    ])
}

fn tint(rgb: (u8, u8, u8), a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied((rgb.0 as f32 * a) as u8, (rgb.1 as f32 * a) as u8, (rgb.2 as f32 * a) as u8, (a * 255.0) as u8)
}

fn blob(p: &Painter, center: Pos2, radius: f32, rgb: (u8, u8, u8), alpha: f32) {
    const SEG: usize = 48;
    const RINGS: [(f32, f32); 5] = [(0.0, 1.0), (0.3, 0.62), (0.58, 0.27), (0.82, 0.07), (1.0, 0.0)];
    let mut m = Mesh::default();
    let color = |k: f32| tint(rgb, alpha * k);
    m.colored_vertex(center, color(RINGS[0].1));
    for (rf, af) in RINGS.iter().skip(1) {
        for s in 0..SEG {
            let a = s as f32 / SEG as f32 * std::f32::consts::TAU;
            m.colored_vertex(center + vec2(a.cos(), a.sin()) * radius * *rf, color(*af));
        }
    }
    for s in 0..SEG {
        m.add_triangle(0, 1 + s as u32, 1 + ((s + 1) % SEG) as u32);
    }
    for ring in 0..(RINGS.len() - 2) {
        let b0 = 1 + (ring * SEG) as u32;
        let b1 = b0 + SEG as u32;
        for s in 0..SEG as u32 {
            let n = (s + 1) % SEG as u32;
            m.add_triangle(b0 + s, b1 + s, b1 + n);
            m.add_triangle(b0 + s, b1 + n, b0 + n);
        }
    }
    p.add(Shape::mesh(m));
}

const INDIGO: (u8, u8, u8) = (104, 118, 255);
const TEAL: (u8, u8, u8) = (45, 212, 191);

pub fn backdrop(p: &Painter, rect: Rect, accent: Color32, light: bool) {
    let k = if light { 0.6 } else { 1.0 };
    let acc = (accent.r(), accent.g(), accent.b());

    let mut m = Mesh::default();
    m.colored_vertex(rect.left_top(), tint(acc, 0.15 * k));
    m.colored_vertex(rect.right_top(), tint(INDIGO, 0.10 * k));
    m.colored_vertex(rect.right_bottom(), tint(TEAL, 0.07 * k));
    m.colored_vertex(rect.left_bottom(), tint(acc, 0.0));
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));

    let span = rect.width().max(rect.height());
    blob(p, pos2(rect.left() + rect.width() * 0.12, rect.top() - rect.height() * 0.04), span * 0.6, acc, 0.22 * k);
    blob(p, pos2(rect.right() - rect.width() * 0.05, rect.bottom() - rect.height() * 0.02), span * 0.55, INDIGO, 0.14 * k);

    let step = 24.0;
    let base = if light { 0.10 } else { 0.075 };
    let dot = if light { (40, 44, 60) } else { (255, 255, 255) };
    let mut dots = Mesh::default();
    let mut y = rect.top() + step * 0.5;
    while y < rect.bottom() {
        let fade = 1.0 - 0.85 * ((y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        let c = tint(dot, base * fade);
        let mut x = rect.left() + step * 0.5;
        while x < rect.right() {
            dots.add_colored_rect(Rect::from_center_size(pos2(x, y), vec2(1.8, 1.8)), c);
            x += step;
        }
        y += step;
    }
    p.add(Shape::mesh(dots));
}

pub fn sidebar_wash(p: &Painter, rect: Rect, accent: Color32, light: bool) {
    let k = if light { 0.6 } else { 1.0 };
    let acc = (accent.r(), accent.g(), accent.b());
    let mut m = Mesh::default();
    m.colored_vertex(rect.left_top(), tint(acc, 0.0));
    m.colored_vertex(rect.right_top(), tint(acc, 0.0));
    m.colored_vertex(rect.right_bottom(), tint(acc, 0.11 * k));
    m.colored_vertex(rect.left_bottom(), tint(acc, 0.11 * k));
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
    blob(p, pos2(rect.left() + rect.width() * 0.2, rect.bottom() + 20.0), rect.height() * 0.5, acc, 0.14 * k);
}
