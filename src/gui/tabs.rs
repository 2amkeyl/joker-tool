use super::*;
use chrono::TimeZone;

fn result_icon(ui: &mut egui::Ui, ok: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    let c = rect.center();
    let color = if ok { GREEN } else { RED };
    let p = ui.painter();
    p.circle_filled(c, 8.0, color.gamma_multiply(0.2));
    if ok {
        p.line_segment([pos2(c.x - 3.5, c.y), pos2(c.x - 1.0, c.y + 2.8)], st(1.7, color));
        p.line_segment([pos2(c.x - 1.0, c.y + 2.8), pos2(c.x + 3.8, c.y - 3.0)], st(1.7, color));
    } else {
        p.line_segment([pos2(c.x - 3.0, c.y - 3.0), pos2(c.x + 3.0, c.y + 3.0)], st(1.7, color));
        p.line_segment([pos2(c.x + 3.0, c.y - 3.0), pos2(c.x - 3.0, c.y + 3.0)], st(1.7, color));
    }
}

fn kbd(ui: &mut egui::Ui, text: &str) {
    egui::Frame::default().fill(card_inner()).stroke(st(1.0, border())).rounding(rnd(6.0)).inner_margin(mgs(7.0, 2.0)).show(ui, |ui| {
        ui.label(RichText::new(text).monospace().size(12.0).color(text_c()));
    });
}

fn setting_row(ui: &mut egui::Ui, title: &str, sub: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(title).size(14.0).color(text_c()));
            if !sub.is_empty() {
                ui.label(RichText::new(sub).size(12.0).color(muted()));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), add);
    });
}

fn tag(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::default().fill(color.gamma_multiply(0.16)).rounding(rnd(6.0)).inner_margin(mgs(7.0, 2.0)).show(ui, |ui| {
        ui.label(RichText::new(text).size(11.5).color(color).family(bold()));
    });
}

fn fmt_expiry(s: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&chrono::Local).format("%d/%m").to_string())
}

fn quest_row(ui: &mut egui::Ui, q: &QuestInfo, selected: &mut HashSet<String>, accent: Color32) {
    let selectable = q.selectable();
    ui.horizontal(|ui| {
        let mut checked = selected.contains(&q.id);
        if ui.add_enabled(selectable, egui::Checkbox::without_text(&mut checked)).changed() {
            if checked {
                selected.insert(q.id.clone());
            } else {
                selected.remove(&q.id);
            }
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let name_col = if selectable { text_c() } else { faint() };
            ui.add(egui::Label::new(RichText::new(&q.name).size(14.0).color(name_col).family(bold())).wrap());
            let mut sub = task_label(&q.task_type).to_string();
            if let Some(r) = &q.reward {
                sub.push_str(&format!("  ·  {}", r));
            }
            if let Some(e) = q.expires.as_deref().and_then(fmt_expiry) {
                sub.push_str(&format!("  ·  {}", tf("quest.expires", &[&e])));
            }
            ui.add(egui::Label::new(RichText::new(sub).size(12.0).color(muted())).wrap());
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if q.completed {
                tag(ui, t("quest.st_done"), GREEN);
            } else if !q.completable {
                tag(ui, t("quest.st_unsupported"), faint());
            } else {
                let remain = (q.needed as f64 - q.done).max(0.0);
                ui.label(RichText::new(fmt_mmss(remain)).size(12.5).color(muted()));
                if q.enrolled && q.done > 0.0 {
                    tag(ui, t("quest.st_partial"), ACCENT_AMBER);
                } else {
                    tag(ui, t("quest.st_new"), accent);
                }
            }
        });
    });
}

fn draw_hype_emblem(ui: &mut egui::Ui, kind: &str, color: Color32) {
    // Keep the house marks deliberately minimal so they remain crisp at small sizes.
    let (rect, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_filled(center, 18.0, color.gamma_multiply(0.16));
    painter.circle_stroke(center, 18.0, st(1.0, color.gamma_multiply(0.75)));

    let stroke = st(2.0, color);
    match kind {
        "bravery" => {
            // Minimal shield outline.
            let points = [
                pos2(center.x, center.y - 10.0),
                pos2(center.x + 8.0, center.y - 6.0),
                pos2(center.x + 7.0, center.y + 3.0),
                pos2(center.x, center.y + 10.0),
                pos2(center.x - 7.0, center.y + 3.0),
                pos2(center.x - 8.0, center.y - 6.0),
            ];
            for i in 0..points.len() {
                painter.line_segment([points[i], points[(i + 1) % points.len()]], stroke);
            }
        }
        "brilliance" => {
            // Four clean rays and a center point.
            painter.line_segment([pos2(center.x, center.y - 10.0), pos2(center.x, center.y + 10.0)], stroke);
            painter.line_segment([pos2(center.x - 10.0, center.y), pos2(center.x + 10.0, center.y)], stroke);
            painter.line_segment([pos2(center.x - 6.0, center.y - 6.0), pos2(center.x + 6.0, center.y + 6.0)], st(1.5, color));
            painter.line_segment([pos2(center.x + 6.0, center.y - 6.0), pos2(center.x - 6.0, center.y + 6.0)], st(1.5, color));
            painter.circle_filled(center, 2.2, color);
        }
        "balance" => {
            // A simple balance beam and center support.
            painter.line_segment([pos2(center.x - 10.0, center.y - 5.0), pos2(center.x + 10.0, center.y - 5.0)], stroke);
            painter.line_segment([pos2(center.x, center.y - 10.0), pos2(center.x, center.y + 9.0)], stroke);
            painter.line_segment([pos2(center.x - 7.0, center.y + 9.0), pos2(center.x + 7.0, center.y + 9.0)], stroke);
            painter.circle_filled(pos2(center.x - 9.0, center.y - 5.0), 2.0, color);
            painter.circle_filled(pos2(center.x + 9.0, center.y - 5.0), 2.0, color);
        }
        _ => {
            painter.line_segment(
                [pos2(center.x - 7.0, center.y - 7.0), pos2(center.x + 7.0, center.y + 7.0)],
                st(2.2, color),
            );
            painter.line_segment(
                [pos2(center.x + 7.0, center.y - 7.0), pos2(center.x - 7.0, center.y + 7.0)],
                st(2.2, color),
            );
        }
    }
}

fn hype_house_card(
    ui: &mut egui::Ui,
    key: &str,
    title: &str,
    description: &str,
    color: Color32,
    selected: bool,
    enabled: bool,
) -> bool {
    let fill = if selected {
        color.gamma_multiply(if theme::is_light() { 0.12 } else { 0.20 })
    } else {
        card_bg()
    };
    let outline = if selected { color } else { border() };
    let frame = egui::Frame::default()
        .fill(fill)
        .stroke(st(if selected { 2.0 } else { 1.0 }, outline))
        .rounding(rnd(14.0))
        .inner_margin(mgs(14.0, 11.0));

    let output = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            draw_hype_emblem(ui, key, color);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(title).size(15.5).color(heading()).family(bold()));
                    if selected {
                        tag(ui, t("hype.selected"), color);
                    }
                });
                ui.add(egui::Label::new(
                    RichText::new(description).size(12.5).color(muted()),
                ).wrap());
            });
        });
    });

    let response = ui.interact(
        output.response.rect,
        ui.id().with(("hype_house_card", key)),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    } else {
        response.clicked()
    }
}

fn kind_label(kind: &str) -> (&'static str, Color32) {
    match kind {
        "quest" => (t("tab.quest"), brand()),
        "badge" => (t("tab.badge"), ACCENT_AMBER),
        _ => (t("tab.hype"), ACCENT_PURPLE),
    }
}

impl JokerApp {
    pub(super) fn render_quest_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::Quest.accent();
        page_header(ui, t("quest.title"), t("quest.subtitle"), accent);

        let logged_in = self.user_info.is_some();
        let mut do_scan = false;
        let mut do_run_all = false;
        let mut do_run_selected = false;

        if self.quest_running {
            if primary_button(ui, t("common.stop"), STOP, true).clicked() {
                if let Some(c) = self.quest_cancel.take() {
                    c.cancel();
                }
            }
        } else if let Some(list) = &self.quest_list {
            let n = self.quest_selected.len();
            let label = tf("quest.start_n", &[&n.to_string()]);
            let can_run = n > 0 && !list.is_empty() && !self.is_busy();
            if primary_button(ui, &label, accent, can_run).clicked() {
                do_run_selected = true;
            }
            if !can_run && self.is_busy() {
                ui.add_space(2.0);
                hint(ui, t("toast.stop_first"));
            }
        } else {
            let label = if self.quest_scanning { t("quest.scanning") } else { t("quest.scan") };
            let can_scan = logged_in && !self.is_busy();
            if primary_button(ui, label, accent, can_scan).clicked() {
                do_scan = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(logged_in && !self.is_busy(), egui::Button::new(RichText::new(t("quest.run_all")).size(12.5).color(muted())).frame(false)).clicked() {
                    do_run_all = true;
                }
            });
        }
        if !logged_in && !self.quest_running {
            hint(ui, t("hint.login_first"));
        }
        if let Some(err) = &self.quest_scan_err {
            ui.add_space(4.0);
            ui.label(RichText::new(tf("quest.scan_err", &[err])).size(12.5).color(RED));
        }
        ui.add_space(14.0);

        let done_n = self.quest_rows.iter().filter(|r| r.ok).count();
        let fail_n = self.quest_rows.len() - done_n;
        let cw = (ui.available_width() - 12.0) / 2.0;
        ui.horizontal(|ui| {
            stat_chip(ui, cw, t("quest.chip_done"), &done_n.to_string(), GREEN);
            stat_chip(ui, cw, t("quest.chip_fail"), &fail_n.to_string(), if fail_n > 0 { RED } else { muted() });
        });
        ui.add_space(14.0);

        let mut rescan = false;
        let mut select_all = false;
        let mut select_none = false;
        if !self.quest_running {
            if let Some(list) = &self.quest_list {
                card(ui, accent, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tf("quest.found", &[&list.len().to_string()])).size(15.0).color(heading()).family(bold()));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button(RichText::new(t("quest.rescan")).size(12.0)).clicked() {
                                rescan = true;
                            }
                            if ui.small_button(RichText::new(t("quest.select_none")).size(12.0)).clicked() {
                                select_none = true;
                            }
                            if ui.small_button(RichText::new(t("quest.select_all")).size(12.0)).clicked() {
                                select_all = true;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    if list.is_empty() {
                        empty_state(ui, accent, t("quest.none_title"), t("quest.none_body"));
                    }
                    ui.spacing_mut().item_spacing.y = 10.0;
                    let actionable: Vec<&QuestInfo> = list.iter().filter(|q| q.selectable()).collect();
                    let unsupported: Vec<&QuestInfo> = list.iter().filter(|q| !q.completed && !q.completable).collect();
                    let finished: Vec<&QuestInfo> = list.iter().filter(|q| q.completed).collect();
                    for q in &actionable {
                        quest_row(ui, q, &mut self.quest_selected, accent);
                    }
                    if !unsupported.is_empty() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(tf("quest.group_unsupported", &[&unsupported.len().to_string()])).size(12.0).color(faint()));
                        for q in &unsupported {
                            quest_row(ui, q, &mut self.quest_selected, accent);
                        }
                    }
                    if !finished.is_empty() {
                        ui.add_space(4.0);
                        egui::CollapsingHeader::new(RichText::new(tf("quest.group_done", &[&finished.len().to_string()])).size(13.0).color(muted()))
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 10.0;
                                for q in &finished {
                                    quest_row(ui, q, &mut self.quest_selected, accent);
                                }
                            });
                    }
                });
                ui.add_space(14.0);
            }
        }

        if self.quest_running || self.quest_list.is_none() {
            card(ui, accent, |ui| {
                if let Some((name, task, done, needed)) = &self.current_quest {
                    ui.horizontal(|ui| {
                        pulse_dot(ui, accent);
                        ui.add(egui::Label::new(RichText::new(name).size(17.0).color(heading()).family(bold())).wrap());
                    });
                    ui.label(RichText::new(task_label(task)).color(muted()).size(12.5));
                    ui.add_space(14.0);
                    let pct = if *needed > 0.0 { (*done / *needed).min(1.0) as f32 } else { 0.0 };
                    progress_bar(ui, pct, &format!("{} / {}", fmt_mmss(*done), fmt_mmss(*needed)), accent, true);
                    ui.add_space(6.0);
                    ui.label(RichText::new(tf("quest.remaining", &[&fmt_mmss(*needed - *done)])).size(12.0).color(muted()));
                } else if self.quest_running || self.quest_scanning {
                    ui.vertical_centered(|ui| {
                        ui.add_space(26.0);
                        ui.spinner();
                        ui.add_space(8.0);
                        ui.label(RichText::new(if self.quest_scanning { t("quest.scanning_list") } else { t("quest.scanning_run") }).italics().color(muted()));
                        ui.add_space(26.0);
                    });
                } else {
                    empty_state(ui, accent, t("quest.empty_title"), t("quest.empty_body"));
                }
            });
        }

        if !self.quest_rows.is_empty() {
            ui.add_space(14.0);
            card(ui, GREEN, |ui| {
                card_title(ui, t("quest.results"), "");
                egui::ScrollArea::vertical().max_height(220.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    for row in self.quest_rows.iter().rev() {
                        ui.horizontal_top(|ui| {
                            result_icon(ui, row.ok);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.add(egui::Label::new(RichText::new(&row.name).size(13.5).color(text_c())).wrap());
                                if !row.ok && !row.reason.is_empty() {
                                    ui.add(egui::Label::new(RichText::new(&row.reason).size(12.0).color(RED)).wrap());
                                }
                            });
                        });
                    }
                });
            });
        }

        if rescan {
            self.quest_list = None;
            self.scan_quests();
        }
        if select_all {
            if let Some(list) = &self.quest_list {
                self.quest_selected = list.iter().filter(|q| q.selectable()).map(|q| q.id.clone()).collect();
            }
        }
        if select_none {
            self.quest_selected.clear();
        }
        if do_scan {
            self.scan_quests();
        }
        if do_run_all {
            self.start_quest(None);
        }
        if do_run_selected {
            let sel = self.quest_selected.clone();
            self.start_quest(Some(sel));
        }
    }

    pub(super) fn render_badge_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::Badge.accent();
        page_header(ui, t("badge.title"), t("badge.subtitle"), accent);

        let running = self.badge_running;
        let logged_in = self.user_info.is_some();
        let fp_busy = self.badge_fp_busy;

        let time_tiers: Vec<Tier> = TIME_MILESTONES
            .iter()
            .map(|m| Tier { key: m.key, head: tf("badge.tier", &[m.key]), title: m.name, sub: tf("badge.hours", &[&m.hours.to_string()]) })
            .collect();
        let game_tiers: Vec<Tier> = GAME_MILESTONES
            .iter()
            .map(|m| Tier { key: m.key, head: tf("badge.tier", &[m.key]), title: m.name, sub: tf("badge.games", &[&m.games.to_string()]) })
            .collect();

        card(ui, accent, |ui| {
            card_title(ui, t("badge.time_title"), t("badge.time_sub"));
            tier_picker(ui, &time_tiers, &mut self.badge_time_key, !running, accent);
        });
        ui.add_space(14.0);
        card(ui, accent, |ui| {
            card_title(ui, t("badge.games_title"), t("badge.games_sub"));
            tier_picker(ui, &game_tiers, &mut self.badge_game_key, !running, accent);
        });
        ui.add_space(14.0);

        if let (Some(tm), Some(g)) = (find_time(&self.badge_time_key), find_games(&self.badge_game_key)) {
            let hpg = tm.hours as f64 / g.games as f64;
            let cw = (ui.available_width() - 24.0) / 3.0;
            ui.horizontal(|ui| {
                stat_chip(ui, cw, t("badge.chip_games"), &g.games.to_string(), accent);
                stat_chip(ui, cw, t("badge.chip_hours"), &tm.hours.to_string(), Color32::from_rgb(255, 205, 120));
                stat_chip(ui, cw, t("badge.chip_per"), &format!("{:.1}h", hpg), text_c());
            });
            ui.add_space(14.0);
        }

        let cookie_missing = self.badge_cookie.trim().is_empty();
        card(ui, accent, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("badge.cookie_title")).size(15.0).color(heading()).family(bold()));
                tag(ui, t("common.required"), RED);
            });
            ui.label(RichText::new(t("badge.cookie_sub")).size(12.5).color(muted()));
            ui.add_space(8.0);
            ui.add_enabled(
                !running,
                egui::TextEdit::singleline(&mut self.badge_cookie).password(true).hint_text(t("badge.cookie_hint")).margin(mgs(10.0, 7.0)).desired_width(ui.available_width()),
            );
        });
        ui.add_space(14.0);

        let has_adv = !self.badge_fingerprint.is_empty();
        let mut pick = false;
        card(ui, accent, |ui| {
            egui::CollapsingHeader::new(RichText::new(t("badge.adv_title")).size(14.5).color(heading()).family(bold()))
                .default_open(has_adv)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new(t("badge.adv_sub")).size(12.0).color(muted()));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Fingerprint").size(13.0).color(text_c()));
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            !running,
                            egui::TextEdit::singleline(&mut self.badge_fingerprint).hint_text(t("badge.optional")).margin(mgs(10.0, 7.0)).desired_width((ui.available_width() - 150.0).max(80.0)),
                        );
                        let can_pick = !self.is_busy() && logged_in;
                        let label = if fp_busy { t("badge.fp_busy") } else { t("badge.fp_pick") };
                        if ui.add_enabled(can_pick, egui::Button::new(label)).on_hover_text(t("badge.fp_tip")).clicked() {
                            pick = true;
                        }
                    });
                    if !self.badge_fingerprint.is_empty() && !is_likely_fingerprint(&self.badge_fingerprint) {
                        ui.label(RichText::new(t("badge.fp_bad")).size(12.0).color(RED));
                    } else {
                        ui.label(RichText::new(t("badge.fp_drop")).size(12.0).color(faint()));
                    }
                });
        });
        if pick {
            self.start_fingerprint(None);
        }
        ui.add_space(16.0);

        let fp_ok = self.badge_fingerprint.is_empty() || is_likely_fingerprint(&self.badge_fingerprint);
        let can_start = !self.is_busy() && logged_in && !cookie_missing && find_time(&self.badge_time_key).is_some() && find_games(&self.badge_game_key).is_some() && fp_ok;
        
        if !running {
            if primary_button(ui, t("common.start"), accent, can_start).clicked() {
                self.start_badge();
            }
            if !logged_in {
                ui.add_space(2.0);
                hint(ui, t("hint.login_first"));
            } else if cookie_missing {
                ui.add_space(2.0);
                hint(ui, t("badge.cookie_required"));
            } else if !fp_ok {
                ui.add_space(2.0);
                hint(ui, t("badge.fix_fp"));
            } else if self.quest_running || self.hs_running || self.quest_scanning {
                ui.add_space(2.0);
                hint(ui, t("toast.stop_first"));
            }
        } else if primary_button(ui, t("common.stop"), STOP, true).clicked() {
            if let Some(c) = self.badge_cancel.take() {
                c.cancel();
            }
        }

        if running || self.badge_progress.2 > 0 {
            ui.add_space(16.0);
            let (done, failed, total) = self.badge_progress;
            let pct = if total > 0 { done as f32 / total as f32 } else { 0.0 };
            progress_bar(ui, pct, &tf("badge.progress", &[&done.to_string(), &total.to_string(), &failed.to_string()]), accent, running);
            if !self.badge_desc.is_empty() && running {
                ui.add_space(6.0);
                ui.label(RichText::new(&self.badge_desc).italics().color(muted()).size(13.0));
            }
        }

        if let Some((ok, msg)) = &self.badge_result {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                result_icon(ui, *ok);
                ui.add(egui::Label::new(RichText::new(msg).color(if *ok { GREEN } else { RED }).family(bold())).wrap());
            });
        }
    }

    pub(super) fn render_hs_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::HypeSquad.accent();
        page_header(ui, t("hype.title"), t("hype.subtitle"), accent);

        card(ui, accent, |ui| {
            card_title(ui, t("hype.pick_title"), t("hype.pick_hint"));

            let busy = self.is_busy();
            let choices = [
                (
                    "bravery",
                    t("hype.bravery"),
                    t("hype.bravery_desc"),
                    Color32::from_rgb(157, 104, 248),
                ),
                (
                    "brilliance",
                    t("hype.brilliance"),
                    t("hype.brilliance_desc"),
                    Color32::from_rgb(244, 123, 103),
                ),
                (
                    "balance",
                    t("hype.balance"),
                    t("hype.balance_desc"),
                    Color32::from_rgb(69, 190, 174),
                ),
            ];

            let mut chosen: Option<&str> = None;
            for (key, title, description, color) in choices {
                let selected = self.hs_house == key;
                if hype_house_card(
                    ui,
                    key,
                    title,
                    description,
                    color,
                    selected,
                    !busy,
                ) {
                    chosen = Some(key);
                }
                ui.add_space(8.0);
            }

            // Separate the leave action from the three house choices.
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(8.0);
            let leave_selected = self.hs_house == "leave";
            if hype_house_card(
                ui,
                "leave",
                t("hype.leave"),
                t("hype.leave_sub"),
                RED,
                leave_selected,
                !busy,
            ) {
                chosen = Some("leave");
            }

            if let Some(key) = chosen {
                self.hs_house = key.to_string();
            }
        });

        ui.add_space(14.0);
        let logged_in = self.user_info.is_some();
        let running = self.hs_running;
        let can_start = !self.is_busy() && logged_in && !self.hs_house.is_empty();
        let label = if running {
            t("hype.working")
        } else if self.hs_house == "leave" {
            t("hype.btn_leave")
        } else {
            t("hype.btn_apply")
        };
        if primary_button(ui, label, accent, can_start).clicked() {
            self.start_hypesquad();
        }

        if !logged_in {
            ui.add_space(4.0);
            hint(ui, t("hint.login_first"));
        } else if self.quest_running || self.badge_running || self.quest_scanning || self.badge_fp_busy {
            ui.add_space(4.0);
            hint(ui, t("toast.stop_first"));
        }

        if let Some((ok, msg)) = &self.hs_result {
            ui.add_space(12.0);
            card(ui, if *ok { GREEN } else { RED }, |ui| {
                ui.horizontal(|ui| {
                    result_icon(ui, *ok);
                    ui.add(egui::Label::new(
                        RichText::new(msg)
                            .color(if *ok { GREEN } else { RED })
                            .family(bold()),
                    ).wrap());
                });
            });
        }
    }

    pub(super) fn render_stats_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::Stats.accent();
        page_header(ui, t("stats.title"), t("stats.subtitle"), accent);

        let (q_ok, q_fail, games, secs, daily, recent, total_sessions) = {
            let s = self.shared.stats.lock();
            let recent: Vec<_> = s.sessions.iter().rev().take(12).cloned().collect();
            (s.quests_ok, s.quests_fail, s.games, s.secs, s.daily_ok(14), recent, s.sessions.len())
        };

        let cw = (ui.available_width() - 36.0) / 4.0;
        ui.horizontal(|ui| {
            stat_chip(ui, cw, t("stats.chip_quests"), &q_ok.to_string(), GREEN);
            stat_chip(ui, cw, t("stats.chip_failed"), &q_fail.to_string(), if q_fail > 0 { RED } else { muted() });
            stat_chip(ui, cw, t("stats.chip_games"), &games.to_string(), ACCENT_AMBER);
            stat_chip(ui, cw, t("stats.chip_time"), &fmt_dur(secs), accent);
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("stats.chart_title"), t("stats.chart_sub"));
            bar_chart(ui, &daily, accent);
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("stats.recent"), &tf("stats.total_sessions", &[&total_sessions.to_string()]));
            if recent.is_empty() {
                empty_state(ui, accent, t("stats.empty_title"), t("stats.empty_body"));
            }
            ui.spacing_mut().item_spacing.y = 9.0;
            for s in &recent {
                let (label, color) = kind_label(&s.kind);
                ui.horizontal(|ui| {
                    status_dot(ui, color);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        let when = chrono::Local.timestamp_opt(s.ts, 0).single().map(|d| d.format("%d/%m %H:%M").to_string()).unwrap_or_default();
                        let acct = if s.account.is_empty() { String::new() } else { format!("  ·  {}", s.account) };
                        ui.label(RichText::new(label).size(13.5).color(text_c()).family(bold()));
                        ui.label(RichText::new(format!("{}{}", when, acct)).size(11.5).color(faint()));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(fmt_dur(s.secs as u64)).monospace().size(12.0).color(muted()));
                        if s.fail > 0 {
                            ui.label(RichText::new(format!("✗ {}", s.fail)).size(12.5).color(RED));
                        }
                        ui.label(RichText::new(format!("✓ {}", s.ok)).size(12.5).color(GREEN));
                    });
                });
            }
        });
        ui.add_space(14.0);

        let label = if self.confirm_clear_stats { t("stats.clear_confirm") } else { t("stats.clear") };
        if ui.add_enabled(total_sessions > 0 || q_ok + q_fail + games > 0, egui::Button::new(RichText::new(label).size(12.5).color(if self.confirm_clear_stats { RED } else { muted() }))).clicked() {
            if self.confirm_clear_stats {
                self.shared.stats.lock().clear();
                self.confirm_clear_stats = false;
                self.toast(ToastKind::Info, t("stats.cleared"));
            } else {
                self.confirm_clear_stats = true;
            }
        }
    }

    pub(super) fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::Settings.accent();
        page_header(ui, t("settings.title"), t("settings.subtitle"), accent);

        let busy = self.is_busy();
        let user = self.user_info.as_ref().map(|u| (u.display(), u.id.clone()));
        let av = self.avatar_tex.as_ref().map(|t| t.id());
        let mut do_logout = false;
        let mut do_check = false;
        let mut do_install = false;
        let mut open_folder = false;
        let mut open_url: Option<&str> = None;
        let mut lang_change: Option<Lang> = None;
        let mut persist = false;
        let mut tray_changed = false;
        let mut show_guide = false;
        let mut appearance_changed = false;
        let mut watch_changed = false;
        let mut autostart_set: Option<bool> = None;
        let mut do_diag = false;

        card(ui, accent, |ui| {
            card_title(ui, t("settings.accounts"), "");
            match &user {
                Some((name, id)) => {
                    ui.horizontal(|ui| {
                        avatar(ui, 44.0, av, Some(name), accent);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new(name).size(16.0).color(heading()).family(bold()));
                            ui.label(RichText::new(format!("ID {}", id)).size(12.0).color(faint()));
                        });
                    });
                }
                None => {
                    ui.label(RichText::new(t("settings.not_logged")).color(muted()));
                }
            }
            if user.is_some() {
                ui.add_space(10.0);
                if ui.add_enabled(!busy, egui::Button::new(t("settings.logout")))
                    .on_disabled_hover_text(t("toast.stop_first"))
                    .clicked()
                {
                    do_logout = true;
                }
            }
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("settings.general"), "");
            let cur = i18n::lang();
            setting_row(ui, t("settings.language"), "", |ui| {
                if ui.selectable_label(cur == Lang::En, "English").clicked() {
                    lang_change = Some(Lang::En);
                }
                if ui.selectable_label(cur == Lang::Vi, "Tiếng Việt").clicked() {
                    lang_change = Some(Lang::Vi);
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            setting_row(ui, t("settings.notify"), t("settings.notify_sub"), |ui| {
                if ui.checkbox(&mut self.cfg.notify, "").changed() {
                    persist = true;
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            setting_row(ui, t("settings.tray"), t("settings.tray_sub"), |ui| {
                if ui.checkbox(&mut self.cfg.tray_on_close, "").changed() {
                    tray_changed = true;
                    persist = true;
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            setting_row(ui, t("settings.watch"), t("settings.watch_sub"), |ui| {
                if ui.checkbox(&mut self.cfg.watch_quests, "").changed() {
                    watch_changed = true;
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            let mut auto = self.cfg.autostart;
            setting_row(ui, t("settings.autostart"), t("settings.autostart_sub"), |ui| {
                if ui.checkbox(&mut auto, "").changed() {
                    autostart_set = Some(auto);
                }
            });
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("settings.appearance"), "");
            ui.label(RichText::new(t("settings.mode")).size(14.0).color(text_c()));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for (key, label) in [
                    ("light", t("settings.mode_light")),
                    ("dark", t("settings.mode_dark")),
                    ("system", t("settings.mode_system")),
                ] {
                    if ui.selectable_label(self.cfg.theme == key, label).clicked() && self.cfg.theme != key {
                        self.cfg.theme = key.to_string();
                        self.cfg.light = key == "light";
                        appearance_changed = true;
                    }
                }
            });
            let mode_description = match self.cfg.theme.as_str() {
                "light" => t("settings.mode_light_sub"),
                "dark" => t("settings.mode_dark_sub"),
                _ => t("settings.mode_system_sub"),
            };
            ui.add_space(5.0);
            ui.add(
                egui::Label::new(RichText::new(mode_description).size(12.5).color(muted()))
                    .wrap(),
            );
            ui.add_space(8.0);
            ui.label(RichText::new(t("settings.accent")).size(14.0).color(text_c()));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let mut swatch = |ui: &mut egui::Ui, name: &str, color: Option<Color32>, tip: &str| {
                    let selected = self.cfg.accent == name;
                    let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
                    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip);
                    let c = rect.center();
                    let p = ui.painter();
                    match color {
                        Some(col) => {
                            p.circle_filled(c, 12.0, col);
                        }
                        None => {
                            p.circle_filled(c, 12.0, card_inner());
                            p.circle_stroke(c, 12.0, st(1.0, border()));
                            p.text(c, Align2::CENTER_CENTER, "A", egui::FontId::new(13.0, bold()), muted());
                        }
                    }
                    if selected {
                        p.circle_stroke(c, 15.5, st(2.0, heading()));
                    } else if resp.hovered() {
                        p.circle_stroke(c, 15.0, st(1.5, border()));
                    }
                    if resp.clicked() && !selected {
                        self.cfg.accent = name.to_string();
                        appearance_changed = true;
                    }
                };
                swatch(ui, "auto", None, t("accent.auto"));
                for (name, col) in theme::ACCENT_CHOICES.iter() {
                    let tip = match *name {
                        "red" => t("accent.red"),
                        "amber" => t("accent.amber"),
                        "teal" => t("accent.teal"),
                        "blue" => t("accent.blue"),
                        "purple" => t("accent.purple"),
                        _ => t("accent.pink"),
                    };
                    swatch(ui, name, Some(*col), tip);
                }
            });
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("settings.app"), "");
            let beta = updater::is_prerelease(updater::CURRENT);
            let title = if beta { tf("settings.version_beta", &[updater::CURRENT]) } else { tf("settings.version", &[updater::CURRENT]) };
            let sub = if self.update_info.is_some() {
                t("settings.update_available")
            } else if self.update_checking {
                t("update.checking")
            } else if self.update_uptodate {
                t("settings.uptodate")
            } else {
                t("settings.update_hint")
            };
            setting_row(ui, &title, sub, |ui| {
                if self.update_info.is_some() {
                    if ui.add_enabled(!busy && !self.update_installing, egui::Button::new(RichText::new(t("update.now")).color(Color32::WHITE).family(bold())).fill(brand())).clicked() {
                        do_install = true;
                    }
                } else if ui.add_enabled(!self.update_checking, egui::Button::new(if self.update_checking { t("update.checking") } else { t("update.check") })).clicked() {
                    do_check = true;
                }
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            setting_row(ui, t("settings.data_folder"), t("settings.data_folder_sub"), |ui| {
                if ui.button(t("settings.open_folder")).clicked() {
                    open_folder = true;
                }
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            setting_row(ui, t("settings.diag"), t("settings.diag_sub"), |ui| {
                if ui.button(t("settings.diag_btn")).clicked() {
                    do_diag = true;
                }
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            setting_row(ui, t("settings.guide"), t("settings.guide_sub"), |ui| {
                if ui.button(t("settings.guide_btn")).clicked() {
                    show_guide = true;
                }
            });
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("settings.shortcuts"), "");
            for (keys, desc) in [
                ("Ctrl + 1 … 5", t("settings.sc_tabs")),
                ("Enter", t("settings.sc_enter")),
                (t("settings.sc_drop_key"), t("settings.sc_drop")),
            ] {
                ui.horizontal(|ui| {
                    kbd(ui, keys);
                    ui.label(RichText::new(desc).size(13.0).color(muted()));
                });
            }
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("settings.support"), t("settings.support_sub"));
            ui.horizontal_wrapped(|ui| {
                if ui.add(egui::Button::new(RichText::new(t("settings.kofi")).color(Color32::WHITE).family(bold())).fill(brand())).clicked() {
                    open_url = Some(KOFI_URL);
                }
                if ui.button(t("settings.github")).clicked() {
                    open_url = Some(REPO_URL);
                }
            });
            ui.add_space(8.0);
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.label(RichText::new(t("settings.credit")).size(11.5).color(faint()));
            });
        });
        ui.add_space(14.0);

        egui::Frame::default()
            .fill(ACCENT_AMBER.gamma_multiply(0.12))
            .stroke(st(1.0, ACCENT_AMBER.gamma_multiply(0.5)))
            .rounding(rnd(12.0))
            .inner_margin(mg(14.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(t("settings.note")).size(13.0).color(ACCENT_AMBER).family(bold()));
                ui.add(egui::Label::new(RichText::new(t("settings.note_body")).size(12.5).color(muted())).wrap());
            });

        if let Some(l) = lang_change {
            self.set_language(l);
        }
        if appearance_changed {
            self.apply_appearance();
        }
        if watch_changed {
            self.persist_config();
            self.restart_watcher();
        }
        if let Some(on) = autostart_set {
            self.set_autostart(on);
        }
        if do_diag {
            self.copy_diagnostics();
        }
        if tray_changed {
            self.apply_tray(false);
        }
        if persist {
            self.persist_config();
        }
        if do_logout {
            self.logout();
        }
        if do_check {
            self.check_update(true);
        }
        if do_install {
            self.install_update();
        }
        if open_folder {
            self.log.open_folder();
        }
        if show_guide {
            self.onboard_step = 0;
            self.onboard_ack = false;
        }
        if let Some(url) = open_url {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
    }
}
