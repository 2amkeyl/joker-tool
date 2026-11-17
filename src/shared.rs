use crate::events::ProgressEvent;
use crate::i18n::{t, tf};
use crate::platform;
use crate::stats::Stats;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct Shared {
    pub notify: AtomicBool,
    pub focused: AtomicBool,
    pub stats: Mutex<Stats>,
}

impl Shared {
    pub fn new(notify: bool) -> Arc<Self> {
        Arc::new(Self { notify: AtomicBool::new(notify), focused: AtomicBool::new(true), stats: Mutex::new(Stats::load()) })
    }

    fn toast(&self, body: String) {
        if self.notify.load(Ordering::Relaxed) && !self.focused.load(Ordering::Relaxed) {
            platform::notify("Joker Tool", &body);
        }
    }

    fn on_event(&self, ev: &ProgressEvent) {
        match ev {
            ProgressEvent::QuestAllDone { completed, failed, .. } => {
                self.stats.lock().finish("quest", *completed as u32, *failed as u32);
                if *completed > 0 || *failed > 0 {
                    self.toast(tf("notify.quest_done", &[&completed.to_string(), &failed.to_string()]));
                }
            }
            ProgressEvent::QuestsFound { new, .. } => {
                if let Some(first) = new.first() {
                    self.toast(tf("notify.new_quests", &[&new.len().to_string(), &first.name]));
                }
            }
            ProgressEvent::BadgeDone { success, done, failed, total, .. } => {
                self.stats.lock().finish("badge", *done as u32, *failed as u32);
                if *success {
                    self.toast(tf("notify.badge_done", &[&done.to_string(), &total.to_string()]));
                } else {
                    self.toast(t("notify.badge_fail").to_string());
                }
            }
            ProgressEvent::HypeSquadResult { success, .. } => {
                self.stats.lock().finish("hype", *success as u32, (!*success) as u32);
                if *success { self.toast(t("notify.hype_done").to_string()); }
            }
            _ => {}
        }
    }
}

pub fn spawn_forwarder(
    rt: &tokio::runtime::Handle, ctx: eframe::egui::Context,
    shared: Arc<Shared>, mut rx: UnboundedReceiver<ProgressEvent>, tx: UnboundedSender<ProgressEvent>,
) {
    rt.spawn(async move {
        while let Some(ev) = rx.recv().await {
            shared.on_event(&ev);
            if tx.send(ev).is_err() { break; }
            ctx.request_repaint();
        }
    });
}