use crate::ui::fx;
use super::*;

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
        let text_w = (ui.available_width() - 190.0).max(160.0);
        ui.allocate_ui_with_layout(vec2(text_w, 0.0), Layout::top_down(Align::Min), |ui| {
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

pub(super) fn time_left(expires: &str) -> Option<(String, bool)> {
    let d = crate::quest::parse_expiry_datetime(expires)?;
    let secs = (d - chrono::Utc::now()).num_seconds();
    if secs <= 0 {
        return Some((t("quest.ended").to_string(), true));
    }
    let (days, hours, mins) = (secs / 86_400, (secs % 86_400) / 3_600, (secs % 3_600) / 60);
    let txt = if days > 0 {
        tf("quest.left_d", &[&days.to_string(), &hours.to_string()])
    } else if hours > 0 {
        tf("quest.left_h", &[&hours.to_string(), &mins.to_string()])
    } else {
        tf("quest.left_m", &[&mins.max(1).to_string()])
    };
    Some((txt, secs < 86_400))
}

fn matches_search(q: &QuestInfo, needle: &str) -> bool {
    needle.is_empty() || q.name.to_lowercase().contains(needle) || q.reward.as_deref().map(|r| r.to_lowercase().contains(needle)).unwrap_or(false)
}

fn sort_view(v: &mut Vec<&QuestInfo>, sort: QuestSort) {
    v.sort_by(|a, b| super::compare_quests(a, b, sort));
}

fn ellipsize(p: &egui::Painter, text: &str, font: egui::FontId, color: Color32, max_w: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), font, color);
    job.wrap = egui::text::TextWrapping { max_width: max_w.max(10.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    p.layout_job(job)
}

fn chip(p: &egui::Painter, left_center: egui::Pos2, text: &str, color: Color32) -> f32 {
    let g = p.layout_no_wrap(text.to_string(), egui::FontId::new(11.0, bold()), color);
    let size = g.size() + vec2(14.0, 6.0);
    let rect = Rect::from_min_size(pos2(left_center.x, left_center.y - size.y / 2.0), size);
    p.rect_filled(rect, rnd(size.y / 2.0), color.gamma_multiply(0.16));
    p.galley(pos2(rect.left() + 7.0, rect.center().y - g.size().y / 2.0), g, color);
    size.x
}

fn quest_card(ui: &mut egui::Ui, q: &QuestInfo, selected: &mut HashSet<String>, accent: Color32) {
    let selectable = q.selectable();
    let h = 88.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), if selectable { Sense::click() } else { Sense::hover() });
    let hv = ui.ctx().animate_bool_with_time(resp.id.with("hv"), selectable && resp.hovered(), 0.14);
    let is_sel = selected.contains(&q.id);
    let partial = q.enrolled && q.done > 0.0 && !q.completed;
    let status = if q.completed { GREEN } else if !q.completable { faint() } else if partial { ACCENT_AMBER } else { accent };

    ui.painter().add(fx::panel_shape(rect, 14.0, card_bg(), hv));
    if is_sel {
        ui.painter().rect_filled(rect, rnd(14.0), accent.gamma_multiply(0.07));
        ui.painter().rect_stroke(rect.shrink(0.5), rnd(14.0), st(1.6, accent));
    } else if hv > 0.0 {
        ui.painter().rect_stroke(rect.shrink(0.5), rnd(14.0), st(1.0, status.gamma_multiply(0.55 * hv)));
    }
    ui.painter().rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 20.0), vec2(3.0, h - 40.0)), rnd(2.0), status);

    let x0 = rect.left() + 18.0;
    let x1 = rect.right() - 16.0;
    let dim = if selectable { 1.0 } else { 0.6 };
    let p = ui.painter();

    let g = ellipsize(p, &q.name, egui::FontId::new(14.5, bold()), heading().gamma_multiply(dim), x1 - x0 - 34.0);
    p.galley(pos2(x0, rect.top() + 12.0), g, heading());

    let mut cx = x0;
    cx += chip(p, pos2(cx, rect.top() + 43.0), task_label(&q.task_type), muted().gamma_multiply(dim)) + 6.0;
    if let Some(r) = &q.reward {
        let r: String = if r.chars().count() > 24 { r.chars().take(23).collect::<String>() + "…" } else { r.clone() };
        chip(p, pos2(cx, rect.top() + 43.0), &r, ACCENT_AMBER.gamma_multiply(dim));
    }

    let m = pos2(x1 - 8.0, rect.top() + 21.0);
    if q.completed {
        p.circle_filled(m, 9.0, GREEN.gamma_multiply(0.22));
        p.line_segment([m + vec2(-3.5, 0.0), m + vec2(-1.0, 2.8)], st(1.8, GREEN));
        p.line_segment([m + vec2(-1.0, 2.8), m + vec2(4.0, -3.0)], st(1.8, GREEN));
    } else if !q.completable {
        p.text(m, Align2::CENTER_CENTER, "—", egui::FontId::new(14.0, egui::FontFamily::Proportional), faint());
    } else if is_sel {
        p.circle_filled(m, 9.0, accent);
        p.line_segment([m + vec2(-3.5, 0.0), m + vec2(-1.0, 2.8)], st(1.8, Color32::WHITE));
        p.line_segment([m + vec2(-1.0, 2.8), m + vec2(4.0, -3.0)], st(1.8, Color32::WHITE));
    } else {
        p.circle_stroke(m, 9.0, st(1.5, muted().lerp_to_gamma(accent, hv)));
    }

    let by = rect.top() + 68.0;
    let mut right_x = x1;
    if let Some((txt, urgent)) = q.expires.as_deref().and_then(time_left) {
        let color = if urgent { RED } else { muted() };
        let g = p.layout_no_wrap(txt, egui::FontId::new(12.0, egui::FontFamily::Proportional), color);
        right_x = x1 - g.size().x;
        p.galley(pos2(right_x, by - g.size().y / 2.0), g, color);
        right_x -= 12.0;
    }
    if !q.completed && q.completable && q.needed > 0 {
        let remain = (q.needed as f64 - q.done).max(0.0);
        let g = p.layout_no_wrap(fmt_mmss(remain), egui::FontId::new(11.5, egui::FontFamily::Monospace), faint());
        right_x -= g.size().x;
        p.galley(pos2(right_x, by - g.size().y / 2.0), g, faint());
        right_x -= 12.0;
        let frac = (q.done / q.needed as f64).clamp(0.0, 1.0) as f32;
        let track = Rect::from_min_max(pos2(x0, by - 3.0), pos2(right_x.max(x0 + 40.0), by + 3.0));
        p.rect_filled(track, rnd(3.0), bg1().gamma_multiply(0.9));
        if frac > 0.0 {
            p.rect_filled(Rect::from_min_size(track.min, vec2((track.width() * frac).max(6.0), track.height())), rnd(3.0), status);
        }
    }

    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && selectable {
        if is_sel { selected.remove(&q.id); } else { selected.insert(q.id.clone()); }
    }
}

pub(super) fn queue_list(
    ui: &mut egui::Ui,
    queue: &mut Vec<String>,
    drag: &mut Option<usize>,
    list: &[QuestInfo],
    accent: Color32,
) {
    let n = queue.len();
    if n == 0 {
        return;
    }
    let (h, gap) = (46.0_f32, 6.0_f32);
    let w = ui.available_width();
    let (area, _) = ui.allocate_exact_size(vec2(w, n as f32 * (h + gap) - gap), Sense::hover());
    let ptr = ui.input(|i| i.pointer.interact_pos());
    let ctx = ui.ctx().clone();

    let target = match (*drag, ptr) {
        (Some(_), Some(pp)) => Some((((pp.y - area.top()) / (h + gap)).floor().clamp(0.0, (n - 1) as f32)) as usize),
        _ => None,
    };
    let mut order: Vec<usize> = (0..n).collect();
    if let (Some(from), Some(to)) = (*drag, target) {
        if from < n {
            let it = order.remove(from);
            order.insert(to.min(order.len()), it);
        }
    }
    let mut slots: Vec<(usize, usize)> = order.iter().enumerate().map(|(slot, &idx)| (slot, idx)).collect();
    slots.sort_by_key(|(_, idx)| Some(*idx) == *drag); // dragged row is painted last (on top)

    let mut started = None;
    let mut dropped = false;
    for (slot, idx) in slots {
        let qid = queue[idx].clone();
        let id = ui.id().with(("queue_row", &qid));
        let slot_y = area.top() + slot as f32 * (h + gap);
        let is_drag = *drag == Some(idx);
        let y = if is_drag {
            ptr.map(|pp| (pp.y - h / 2.0).clamp(area.top() - 6.0, area.bottom() - h + 6.0)).unwrap_or(slot_y)
        } else {
            ctx.animate_value_with_time(id.with("y"), slot_y, 0.15)
        };
        let rect = Rect::from_min_size(pos2(area.left(), y), vec2(w, h));
        let resp = ui.interact(rect, id, Sense::drag());
        if resp.drag_started() {
            started = Some(idx);
        }
        if resp.drag_stopped() {
            dropped = true;
        }
        if resp.dragged() {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if resp.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }

        let lift = if is_drag { 1.0 } else if resp.hovered() { 0.4 } else { 0.0 };
        ui.painter().add(fx::panel_shape(rect, 12.0, card_bg(), lift));
        if is_drag {
            ui.painter().rect_stroke(rect.shrink(0.5), rnd(12.0), st(1.5, accent));
        }
        for r in 0..3 {
            for c in 0..2 {
                ui.painter().circle_filled(pos2(rect.left() + 14.0 + c as f32 * 5.0, rect.center().y - 6.0 + r as f32 * 6.0), 1.5, faint());
            }
        }
        let q = list.iter().find(|q| q.id == qid);
        let p = ui.painter();
        p.circle_filled(pos2(rect.left() + 24.0, rect.center().y), 9.0, accent.gamma_multiply(0.22));
        p.text(pos2(rect.left() + 24.0, rect.center().y), Align2::CENTER_CENTER, (slot + 1).to_string(), egui::FontId::new(11.5, bold()), accent);
        let text_x = rect.left() + 48.0;
        if let Some(q) = q {
            let g = ellipsize(p, &q.name, egui::FontId::new(13.5, bold()), heading(), rect.right() - text_x - 14.0);
            p.galley(pos2(text_x, rect.top() + 8.0), g, heading());
            let mut sub = task_label(&q.task_type).to_string();
            if let Some(r) = &q.reward {
                sub.push_str(&format!("  ·  {}", r));
            }
            let g = ellipsize(p, &sub, egui::FontId::new(11.5, egui::FontFamily::Proportional), muted(), rect.right() - text_x - 14.0);
            p.galley(pos2(text_x, rect.top() + 26.0), g, muted());
        }
    }
    if let Some(i) = started {
        *drag = Some(i);
    }
    if dropped {
        if let (Some(from), Some(to)) = (*drag, target) {
            if from < queue.len() {
                let it = queue.remove(from);
                queue.insert(to.min(queue.len()), it);
            }
        }
        *drag = None;
    }
    if drag.is_some() {
        ctx.request_repaint();
    }
}

fn draw_hype_emblem(ui: &mut egui::Ui, kind: &str, color: Color32, texture: Option<egui::TextureId>) {
    let (rect, _) = ui.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
    if let Some(texture) = texture {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().image(texture, rect, uv, Color32::WHITE);
        return;
    }
    // Vector fallback is used only when an optional house PNG has not been supplied.
    let center = rect.center();
    let painter = ui.painter();
    painter.circle_filled(center, 18.0, color.gamma_multiply(0.16));
    painter.circle_stroke(center, 18.0, st(1.0, color.gamma_multiply(0.75)));

    let stroke = st(2.0, color);
    match kind {
        "bravery" => {
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
            painter.line_segment([pos2(center.x, center.y - 10.0), pos2(center.x, center.y + 10.0)], stroke);
            painter.line_segment([pos2(center.x - 10.0, center.y), pos2(center.x + 10.0, center.y)], stroke);
            painter.line_segment([pos2(center.x - 6.0, center.y - 6.0), pos2(center.x + 6.0, center.y + 6.0)], st(1.5, color));
            painter.line_segment([pos2(center.x + 6.0, center.y - 6.0), pos2(center.x - 6.0, center.y + 6.0)], st(1.5, color));
            painter.circle_filled(center, 2.2, color);
        }
        "balance" => {
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
    icon: Option<egui::TextureId>,
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
            draw_hype_emblem(ui, key, color, icon);
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

impl JokerApp {
    pub(super) fn render_quest_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::Quest.accent();
        page_header(ui, t("quest.subtitle"), accent);

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
        if !self.quest_running && self.quest_list.is_some() {
            let total = self.quest_list.as_ref().map(|l| l.len()).unwrap_or(0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tf("quest.found", &[&total.to_string()])).size(15.0).color(heading()).family(bold()));
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
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.add(egui::TextEdit::singleline(&mut self.quest_search).hint_text(t("quest.search")).margin(mgs(10.0, 6.0)).desired_width(170.0));
                for (sort, label) in [
                    (QuestSort::Expiry, t("quest.sort_expiry")),
                    (QuestSort::Name, t("quest.sort_name")),
                ] {
                    if ui.selectable_label(self.quest_sort == sort, RichText::new(label).size(12.5)).clicked() {
                        self.quest_sort = sort;
                    }
                }
            });
            ui.add_space(12.0);

            // Expired or unsupported quests must not remain selected or in the run queue.
            if let Some(list) = &self.quest_list {
                self.quest_selected.retain(|id| list.iter().any(|q| q.id.as_str() == id.as_str() && q.selectable()));
            }

            // Keep the run queue in sync with the selection: drop deselected quests, append new ones.
            self.quest_queue.retain(|id| self.quest_selected.contains(id));
            if let Some(list) = &self.quest_list {
                for q in list {
                    if self.quest_selected.contains(&q.id) && !self.quest_queue.contains(&q.id) {
                        self.quest_queue.push(q.id.clone());
                    }
                }
            }

            if let Some(list) = &self.quest_list {
                if self.quest_queue.len() > 1 {
                    card(ui, accent, |ui| {
                        card_title(ui, &tf("quest.queue_title", &[&self.quest_queue.len().to_string()]), t("quest.queue_hint"));
                        queue_list(ui, &mut self.quest_queue, &mut self.queue_drag, list, accent);
                    });
                    ui.add_space(14.0);
                }

                let needle = self.quest_search.trim().to_lowercase();
                let mut actionable: Vec<&QuestInfo> = list
                    .iter()
                    .filter(|q| q.selectable() && matches_search(q, &needle))
                    .collect();
                let mut finished: Vec<&QuestInfo> = list
                    .iter()
                    .filter(|q| q.completed && !q.is_expired() && matches_search(q, &needle))
                    .collect();
                let mut expired: Vec<&QuestInfo> = list
                    .iter()
                    .filter(|q| q.is_expired() && matches_search(q, &needle))
                    .collect();
                let mut unsupported: Vec<&QuestInfo> = list
                    .iter()
                    .filter(|q| !q.completed && !q.is_expired() && !q.completable && matches_search(q, &needle))
                    .collect();

                sort_view(&mut actionable, self.quest_sort);
                sort_view(&mut finished, self.quest_sort);
                sort_view(&mut expired, self.quest_sort);
                sort_view(&mut unsupported, self.quest_sort);

                ui.spacing_mut().item_spacing.y = 10.0;
                if list.is_empty() {
                    empty_state(ui, accent, t("quest.none_title"), t("quest.none_body"));
                } else if actionable.is_empty()
                    && expired.is_empty()
                    && unsupported.is_empty()
                    && finished.is_empty()
                {
                    hint(ui, t("quest.no_match"));
                }

                // Keep actionable quests first; completed, expired and unsupported entries follow.
                for q in &actionable {
                    quest_card(ui, q, &mut self.quest_selected, accent);
                }
                if !finished.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(tf("quest.group_done", &[&finished.len().to_string()])).size(12.0).color(faint()));
                    for q in &finished {
                        quest_card(ui, q, &mut self.quest_selected, accent);
                    }
                }
                if !expired.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(tf("quest.group_expired", &[&expired.len().to_string()])).size(12.0).color(faint()));
                    for q in &expired {
                        quest_card(ui, q, &mut self.quest_selected, accent);
                    }
                }
                if !unsupported.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(tf("quest.group_unsupported", &[&unsupported.len().to_string()])).size(12.0).color(faint()));
                    for q in &unsupported {
                        quest_card(ui, q, &mut self.quest_selected, accent);
                    }
                }
            }
            ui.add_space(14.0);
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
                    ui.add_space(10.0);
                    ui.horizontal_centered(|ui| {
                        ui.label(RichText::new(if self.quest_scanning { t("quest.scanning_list") } else { t("quest.scanning_run") }).size(12.0).italics().color(muted()));
                    });
                    ui.add_space(10.0);
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
            let order: Vec<String> = self.quest_queue.iter().filter(|id| self.quest_selected.contains(*id)).cloned().collect();
            self.start_quest(Some(order));
        }
    }

    pub(super) fn render_badge_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::Badge.accent();
        page_header(ui, t("badge.subtitle"), accent);

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
            tier_slider(ui, &time_tiers, &mut self.badge_time_key, !running, accent);
        });
        ui.add_space(14.0);
        card(ui, accent, |ui| {
            card_title(ui, t("badge.games_title"), t("badge.games_sub"));
            tier_slider(ui, &game_tiers, &mut self.badge_game_key, !running, accent);
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
            ui.add_space(8.0);
            ui.label(RichText::new(tf("badge.explain", &[&tm.hours.to_string(), &g.games.to_string(), &format!("{:.1}", hpg)])).size(12.5).color(muted()));
            let (risk_key, risk_col) = if tm.hours >= 1000 { ("badge.risk_high", RED) } else if tm.hours >= 150 { ("badge.risk_mid", ACCENT_AMBER) } else { ("badge.risk_low", GREEN) };
            ui.horizontal_wrapped(|ui| {
                status_dot(ui, risk_col);
                ui.label(RichText::new(t(risk_key)).size(12.5).color(risk_col).family(bold()));
                ui.label(RichText::new(t("badge.risk_note")).size(12.0).color(muted()));
            });
            ui.add_space(14.0);
        }

        let cookie_missing = self.badge_cookie.trim().is_empty();
        self.render_badge_guides(ui, accent, cookie_missing);
        ui.add_space(14.0);
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
        page_header(ui, t("hype.subtitle"), accent);

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
                    self.hype_icons.get(key).map(|texture| texture.id()),
                ) {
                    chosen = Some(key);
                }
                ui.add_space(8.0);
            }

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
                None,
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

    pub(super) fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        let accent = Tab::Settings.accent();
        page_header(ui, t("settings.subtitle"), accent);

        let busy = self.is_busy();
        let mut do_check = false;
        let mut do_install = false;
        let mut open_folder = false;
        let mut open_url: Option<&str> = None;
        let mut lang_change: Option<Lang> = None;
        let mut persist = false;
        let mut show_guide = false;
        let mut appearance_changed = false;
        let mut watch_changed = false;
        let mut do_diag = false;

        card(ui, accent, |ui| {
            card_title(ui, t("settings.appearance"), "");
            let mut selected_fps = self.cfg.motion_fps;
            setting_row(ui, t("settings.motion"), t("settings.motion_sub"), |ui| {
                egui::ComboBox::from_id_salt("animation_fps")
                    .selected_text(format!("{} FPS", selected_fps))
                    .width(108.0)
                    .show_ui(ui, |ui| {
                        for fps in crate::config::MOTION_FPS_OPTIONS {
                            ui.selectable_value(&mut selected_fps, fps, format!("{} FPS", fps));
                        }
                    });
            });
            if selected_fps != self.cfg.motion_fps {
                self.cfg.motion_fps = selected_fps;
                persist = true;
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(8.0);
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
            setting_row(ui, t("settings.watch"), t("settings.watch_sub"), |ui| {
                if ui.checkbox(&mut self.cfg.watch_quests, "").changed() {
                    watch_changed = true;
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

        let has_stats = {
            let s = self.shared.stats.lock();
            !s.sessions.is_empty() || s.quests_ok + s.quests_fail + s.games > 0
        };
        card(ui, accent, |ui| {
            card_title(ui, t("settings.data"), "");
            setting_row(ui, t("stats.clear"), t("settings.stats_sub"), |ui| {
                let label = if self.confirm_clear_stats { t("stats.clear_confirm") } else { t("stats.clear") };
                let color = if self.confirm_clear_stats { RED } else { muted() };
                if ui.add_enabled(has_stats, egui::Button::new(RichText::new(label).size(12.5).color(color))).clicked() {
                    if self.confirm_clear_stats {
                        self.shared.stats.lock().clear();
                        self.confirm_clear_stats = false;
                        self.toast(ToastKind::Info, t("stats.cleared"));
                    } else {
                        self.confirm_clear_stats = true;
                    }
                }
            });
        });
        ui.add_space(14.0);

        card(ui, accent, |ui| {
            card_title(ui, t("settings.shortcuts"), "");
            for (keys, desc) in [
                ("Ctrl + 1 … 4", t("settings.sc_tabs")),
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
        if do_diag {
            self.copy_diagnostics();
        }
        if persist {
            self.persist_config();
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
