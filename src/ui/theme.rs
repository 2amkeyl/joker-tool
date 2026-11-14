use eframe::egui::{self, Color32, FontFamily, FontId, Margin, Rounding, Stroke, TextStyle, Vec2};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const GREEN: Color32 = Color32::from_rgb(52, 199, 120);
pub const RED: Color32 = Color32::from_rgb(240, 70, 80);
pub const STOP: Color32 = Color32::from_rgb(168, 40, 52);
pub const ACCENT_RED: Color32 = Color32::from_rgb(232, 32, 54);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(255, 160, 40);
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(157, 104, 248);
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(96, 165, 250);
pub const ACCENT_TEAL: Color32 = Color32::from_rgb(45, 212, 191);

struct Palette {
    bg1: Color32,
    bg2: Color32,
    bg3: Color32,
    card: Color32,
    card_inner: Color32,
    border: Color32,
    text: Color32,
    muted: Color32,
    faint: Color32,
    disabled: Color32,
    heading: Color32,
    edge: Color32,
    hover: Color32,
}

const DARK: Palette = Palette {
    bg1: Color32::from_rgb(14, 14, 17),
    bg2: Color32::from_rgb(21, 21, 25),
    bg3: Color32::from_rgb(27, 27, 32),
    card: Color32::from_rgb(35, 35, 41),
    card_inner: Color32::from_rgb(24, 24, 29),
    border: Color32::from_rgb(56, 56, 66),
    text: Color32::from_rgb(238, 238, 244),
    muted: Color32::from_rgb(170, 172, 188),
    faint: Color32::from_rgb(120, 122, 138),
    disabled: Color32::from_rgb(46, 46, 54),
    heading: Color32::WHITE,
    edge: Color32::from_rgb(10, 10, 12),
    hover: Color32::from_rgb(46, 46, 56),
};

const LIGHT: Palette = Palette {
    bg1: Color32::from_rgb(234, 236, 241),
    bg2: Color32::from_rgb(247, 248, 250),
    bg3: Color32::from_rgb(240, 242, 246),
    card: Color32::from_rgb(255, 255, 255),
    card_inner: Color32::from_rgb(245, 246, 249),
    border: Color32::from_rgb(214, 217, 226),
    text: Color32::from_rgb(34, 36, 46),
    muted: Color32::from_rgb(98, 102, 120),
    faint: Color32::from_rgb(140, 144, 160),
    disabled: Color32::from_rgb(226, 229, 236),
    heading: Color32::from_rgb(16, 17, 24),
    edge: Color32::from_rgb(222, 225, 232),
    hover: Color32::from_rgb(232, 235, 242),
};

static LIGHT_MODE: AtomicBool = AtomicBool::new(false);
static ACCENT_OVERRIDE: AtomicU32 = AtomicU32::new(0);

fn pal() -> &'static Palette {
    if LIGHT_MODE.load(Ordering::Relaxed) { &LIGHT } else { &DARK }
}

pub fn set_light(light: bool) {
    LIGHT_MODE.store(light, Ordering::Relaxed);
}

pub fn is_light() -> bool {
    LIGHT_MODE.load(Ordering::Relaxed)
}

pub const ACCENT_CHOICES: [(&str, Color32); 6] = [
    ("red", ACCENT_RED),
    ("amber", ACCENT_AMBER),
    ("teal", ACCENT_TEAL),
    ("blue", ACCENT_BLUE),
    ("purple", ACCENT_PURPLE),
    ("pink", Color32::from_rgb(244, 114, 182)),
];

pub fn set_accent(name: &str) {
    let v = ACCENT_CHOICES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| 0xFF00_0000 | ((c.r() as u32) << 16) | ((c.g() as u32) << 8) | c.b() as u32)
        .unwrap_or(0);
    ACCENT_OVERRIDE.store(v, Ordering::Relaxed);
}

pub fn accent_override() -> Option<Color32> {
    let v = ACCENT_OVERRIDE.load(Ordering::Relaxed);
    if v == 0 {
        None
    } else {
        Some(Color32::from_rgb(((v >> 16) & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8))
    }
}

pub fn brand() -> Color32 {
    accent_override().unwrap_or(ACCENT_RED)
}

pub fn bg1() -> Color32 {
    pal().bg1
}
pub fn bg2() -> Color32 {
    pal().bg2
}
pub fn bg3() -> Color32 {
    pal().bg3
}
pub fn card_bg() -> Color32 {
    pal().card
}
pub fn card_inner() -> Color32 {
    pal().card_inner
}
pub fn border() -> Color32 {
    pal().border
}
pub fn text_c() -> Color32 {
    pal().text
}
pub fn muted() -> Color32 {
    pal().muted
}
pub fn faint() -> Color32 {
    pal().faint
}
pub fn disabled_bg() -> Color32 {
    pal().disabled
}
pub fn heading() -> Color32 {
    pal().heading
}
pub fn edge() -> Color32 {
    pal().edge
}
pub fn hover_bg() -> Color32 {
    pal().hover
}

pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn rnd(r: f32) -> Rounding {
    Rounding::same(r)
}

pub fn st(w: f32, c: Color32) -> Stroke {
    Stroke::new(w, c)
}

pub fn mg(v: f32) -> Margin {
    Margin::same(v)
}

pub fn mgs(x: f32, y: f32) -> Margin {
    Margin::symmetric(x, y)
}

pub fn apply_theme(ctx: &egui::Context) {
    let p = pal();
    let light = is_light();
    let mut style = (*ctx.style()).clone();
    let mut v = if light { egui::Visuals::light() } else { egui::Visuals::dark() };
    v.window_fill = p.card;
    v.panel_fill = p.bg3;
    v.extreme_bg_color = p.bg1;
    v.selection.bg_fill = if light { brand().gamma_multiply(0.35) } else { brand().gamma_multiply(0.62) };
    v.selection.stroke = st(1.0, if light { p.heading } else { Color32::WHITE });
    v.window_rounding = rnd(14.0);
    v.window_stroke = st(1.0, p.border);
    v.menu_rounding = rnd(10.0);
    v.popup_shadow.color = Color32::from_black_alpha(if light { 60 } else { 140 });
    v.window_shadow.color = Color32::from_black_alpha(if light { 70 } else { 160 });
    v.override_text_color = Some(p.text);

    let r = rnd(10.0);
    v.widgets.noninteractive.bg_fill = p.bg2;
    v.widgets.noninteractive.bg_stroke = st(1.0, p.border);
    v.widgets.noninteractive.rounding = r;

    let (inactive, hovered) = if light {
        (Color32::from_rgb(230, 233, 240), Color32::from_rgb(216, 220, 230))
    } else {
        (Color32::from_rgb(48, 48, 57), Color32::from_rgb(66, 66, 78))
    };
    for (w, fill, fg) in [
        (&mut v.widgets.inactive, inactive, p.text),
        (&mut v.widgets.hovered, hovered, p.heading),
        (&mut v.widgets.active, brand(), Color32::WHITE),
        (&mut v.widgets.open, hovered, p.heading),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = st(1.0, p.border);
        w.fg_stroke = st(1.0, fg);
        w.rounding = r;
        w.expansion = 0.0;
    }
    v.widgets.hovered.bg_stroke = st(1.0, brand().gamma_multiply(0.6));

    style.visuals = v;
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.combo_height = 260.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    style.text_styles.insert(TextStyle::Body, FontId::new(14.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(14.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));
    ctx.set_style(style);
}
