pub mod theme;
pub mod widgets;

use crate::i18n::t;
use eframe::egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Quest,
    Badge,
    HypeSquad,
    Stats,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Quest, Tab::Badge, Tab::HypeSquad, Tab::Stats, Tab::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Quest => t("tab.quest"),
            Tab::Badge => t("tab.badge"),
            Tab::HypeSquad => t("tab.hype"),
            Tab::Stats => t("tab.stats"),
            Tab::Settings => t("tab.settings"),
        }
    }

    pub fn accent(self) -> Color32 {
        if let Some(c) = theme::accent_override() {
            return c;
        }
        match self {
            Tab::Quest => theme::ACCENT_RED,
            Tab::Badge => theme::ACCENT_AMBER,
            Tab::HypeSquad => theme::ACCENT_PURPLE,
            Tab::Stats => theme::ACCENT_TEAL,
            Tab::Settings => theme::ACCENT_BLUE,
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}
