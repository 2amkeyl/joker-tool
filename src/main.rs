#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod avatar;
mod badge;
mod config;
mod events;
mod gui;
mod hypesquad;
mod hype_assets;
mod i18n;
mod log_writer;
mod platform;
mod quest;
mod shared;
mod stats;
mod ui;
mod updater;

use config::AppConfig;
use eframe::egui;
use gui::JokerApp;

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/BeVietnamPro-Regular.ttf")),
    );
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "my_font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "my_font".to_owned());

    fonts.font_data.insert("bold_font".to_owned(), egui::FontData::from_static(include_bytes!("../assets/fonts/BeVietnamPro-SemiBold.ttf")));
    fonts.families.insert(egui::FontFamily::Name("bold".into()), vec!["bold_font".to_owned(), "my_font".to_owned()]);

    if let Ok(emoji_data) = std::fs::read("C:\\Windows\\Fonts\\seguiemj.ttf") {
        fonts.font_data.insert("emoji".to_owned(), egui::FontData::from_owned(emoji_data));
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().push("emoji".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Name("bold".into())).unwrap().push("emoji".to_owned());
    }
    // Icon fonts (Phosphor, MIT): regular weight joins the normal fallback chain, the filled weight
    // lives in its own family so the selected sidebar item can use it.
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    fonts.font_data.insert("phosphor-fill".to_owned(), egui_phosphor::Variant::Fill.font_data());
    fonts.families.insert(egui::FontFamily::Name("ph_fill".into()), vec!["phosphor-fill".to_owned()]);
    ctx.set_fonts(fonts);
}

fn load_icon() -> eframe::egui::IconData {
    let Ok(img) = image::load_from_memory(include_bytes!("../assets/icon.png")) else {
        return eframe::egui::IconData::default();
    };
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    eframe::egui::IconData { rgba: rgba.into_raw(), width: w, height: h }
}

fn main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(AppConfig::log_path()) {
            let _ = writeln!(f, "[PANIC] {}", info);
        }
    }));
    let restarted = std::env::args().any(|a| a == "--restarted");
    if !platform::acquire_single_instance(if restarted { 8000 } else { 0 }) {
        platform::focus_existing();
        return Ok(());
    }

    platform::clear_legacy_autostart();

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let cfg = AppConfig::load();
    i18n::set_lang(i18n::Lang::from_code(&cfg.lang));
    let start_light = match cfg.theme.as_str() {
        "light" => true,
        "system" => platform::system_prefers_light().unwrap_or(false),
        _ => false,
    };
    ui::theme::set_light(start_light);
    ui::theme::set_accent(&cfg.accent);
    let log = log_writer::LogWriter::new(AppConfig::log_path());
    let shared = shared::Shared::new(cfg.notify);
    let (tx_workers, rx_workers) = tokio::sync::mpsc::unbounded_channel();
    let (tx_gui, rx_gui) = tokio::sync::mpsc::unbounded_channel();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 640.0])
            .with_min_inner_size([720.0, 480.0])
            .with_icon(load_icon())
            .with_maximize_button(false)
            .with_resizable(false)
            .with_title("Joker Tool"),
        ..Default::default()
    };

    eframe::run_native(
        "Joker Tool",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            ui::theme::apply_theme(&cc.egui_ctx);
            shared::spawn_forwarder(rt.handle(), cc.egui_ctx.clone(), shared.clone(), rx_workers, tx_gui);
            Ok(Box::new(JokerApp::new(rt, cfg, log, tx_workers, rx_gui, shared, cc.egui_ctx.clone())))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {e}"))?;

    Ok(())
}
