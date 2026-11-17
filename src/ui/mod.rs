pub mod fx;
pub mod theme;
pub mod widgets;

use crate::i18n::t;
use eframe::egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Quest,
    Badge,
    HypeSquad,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Quest, Tab::Badge, Tab::HypeSquad, Tab::Settings];
    pub const NAV: [Tab; 3] = [Tab::Quest, Tab::Badge, Tab::HypeSquad];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Quest => t("tab.quest"),
            Tab::Badge => t("tab.badge"),
            Tab::HypeSquad => t("tab.hype"),
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
            Tab::Settings => theme::ACCENT_BLUE,
        }
    }

    pub fn icon(self) -> &'static str {
        use egui_phosphor::regular as ph;
        match self {
            Tab::Quest => ph::TROPHY,
            Tab::Badge => ph::SEAL_CHECK,
            Tab::HypeSquad => ph::USERS_THREE,
            Tab::Settings => ph::GEAR_SIX,
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavFlag {
    #[default]
    None,
    New,
    Error,
}
