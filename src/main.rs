mod avatar;
mod badge;
mod common;
mod config;
mod events;
mod gui;
mod hypesquad;
mod log_writer;
mod quest;

use config::AppConfig;
use gui::JokerApp;

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../Inter-Regular.ttf")),
    );
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "my_font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "my_font".to_owned());

    fonts.font_data.insert("bold_font".to_owned(), egui::FontData::from_static(include_bytes!("../Inter-SemiBold.ttf")));
    fonts.families.insert(egui::FontFamily::Name("bold".into()), vec!["bold_font".to_owned(), "my_font".to_owned()]);

    if let Ok(emoji_data) = std::fs::read("C:\\Windows\\Fonts\\seguiemj.ttf") {
        fonts.font_data.insert("emoji".to_owned(), egui::FontData::from_owned(emoji_data));
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().push("emoji".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Name("bold".into())).unwrap().push("emoji".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn load_icon() -> eframe::egui::IconData {
    let icon_bytes = include_bytes!("../icon.png");
    
    let img = match image::load_from_memory(icon_bytes) {
        Ok(img) => img,
        Err(_) => return eframe::egui::IconData::default(),
    };

    let resized = img.resize_exact(256, 256, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();
    let (w, h) = rgba.dimensions();
    
    eframe::egui::IconData {
        rgba: rgba.into_raw(),
        width: w,
        height: h,
    }
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let cfg = AppConfig::load();
    let log = log_writer::LogWriter::new(AppConfig::log_path());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 680.0])
            .with_min_inner_size([760.0, 540.0])
            .with_icon(load_icon()) // Gắn icon
            .with_title("Joker Tool"),
        ..Default::default()
    };

    let app = JokerApp::new(rt, cfg, log, tx, rx);

    eframe::run_native(
        "Joker Tool",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            gui::apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ).map_err(|e| anyhow::anyhow!("GUI lỗi: {e}"))?;

    Ok(())
}
