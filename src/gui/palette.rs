use super::update::dim_block;
use super::*;

#[derive(Clone)]
pub(super) enum Cmd {
    Tab(Tab),
    Scan,
    Stop,
    Lang(Lang),
    Theme(&'static str),
    Accent(&'static str),
    Diagnostics,
    CheckUpdate,
    OpenFolder,
}

fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ằ' | 'ắ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' => 'a',
            'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => 'e',
            'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
            'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ' | 'ở' | 'ỡ' | 'ợ' => 'o',
            'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => 'u',
            'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
            'đ' => 'd',
            other => other,
        })
        .collect()
}

fn accent_name(name: &str) -> &'static str {
    match name {
        "red" => t("accent.red"),
        "amber" => t("accent.amber"),
        "teal" => t("accent.teal"),
        "blue" => t("accent.blue"),
        "purple" => t("accent.purple"),
        "pink" => t("accent.pink"),
        _ => t("accent.auto"),
    }
}

impl JokerApp {
    fn palette_items(&self) -> Vec<(Cmd, String, String)> {
        let mut v: Vec<(Cmd, String, String)> = Vec::new();
        for (i, tab) in Tab::ALL.iter().enumerate() {
            v.push((Cmd::Tab(*tab), tf("pal.go_tab", &[tab.label()]), format!("Ctrl+{}", i + 1)));
        }
        if self.user_info.is_some() && !self.quest_running {
            v.push((Cmd::Scan, t("pal.scan").to_string(), t("tab.quest").to_string()));
        }
        if self.quest_running || self.badge_running {
            v.push((Cmd::Stop, t("pal.stop").to_string(), String::new()));
        }
        if i18n::lang() == Lang::Vi {
            v.push((Cmd::Lang(Lang::En), t("pal.lang_en").to_string(), t("settings.language").to_string()));
        } else {
            v.push((Cmd::Lang(Lang::Vi), t("pal.lang_vi").to_string(), t("settings.language").to_string()));
        }
        for (key, label) in [
            ("system", t("pal.theme_system")),
            ("dark", t("pal.dark")),
            ("light", t("pal.light")),
        ] {
            if self.cfg.theme != key {
                v.push((Cmd::Theme(key), label.to_string(), t("settings.appearance").to_string()));
            }
        }
        v.push((Cmd::Accent("auto"), tf("pal.accent", &[accent_name("auto")]), t("settings.appearance").to_string()));
        for (name, _) in theme::ACCENT_CHOICES.iter() {
            v.push((Cmd::Accent(name), tf("pal.accent", &[accent_name(name)]), t("settings.appearance").to_string()));
        }
        v.push((Cmd::Diagnostics, t("pal.diag").to_string(), t("settings.app").to_string()));
        v.push((Cmd::CheckUpdate, t("update.check").to_string(), t("settings.app").to_string()));
        v.push((Cmd::OpenFolder, t("settings.open_folder").to_string(), t("settings.data_folder").to_string()));
        v
    }

    fn run_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Tab(tab) => self.set_tab(tab),
            Cmd::Scan => {
                self.set_tab(Tab::Quest);
                self.quest_list = None;
                self.scan_quests();
            }
            Cmd::Stop => self.stop_all(),
            Cmd::Lang(l) => self.set_language(l),
            Cmd::Theme(mode) => {
                self.cfg.theme = mode.to_string();
                self.cfg.light = mode == "light";
                self.apply_appearance();
            }
            Cmd::Accent(name) => {
                self.cfg.accent = name.to_string();
                self.apply_appearance();
            }
            Cmd::Diagnostics => self.copy_diagnostics(),
            Cmd::CheckUpdate => self.check_update(true),
            Cmd::OpenFolder => self.log.open_folder(),
        }
    }

    pub(super) fn toggle_palette(&mut self) {
        self.palette_open = !self.palette_open;
        self.palette_query.clear();
        self.palette_sel = 0;
    }

    pub(super) fn render_palette(&mut self, ctx: &egui::Context) {
        dim_block(ctx, "palette_block", 120);
        let items = self.palette_items();
        let q = fold(&self.palette_query);
        let toks: Vec<&str> = q.split_whitespace().collect();
        let matches: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, (_, label, hint))| {
                let hay = fold(&format!("{} {}", label, hint));
                toks.iter().all(|tk| hay.contains(tk))
            })
            .map(|(i, _)| i)
            .take(9)
            .collect();

        let (down, up, enter, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        let n = matches.len();
        if n > 0 {
            self.palette_sel = self.palette_sel.min(n - 1);
            if down {
                self.palette_sel = (self.palette_sel + 1) % n;
            }
            if up {
                self.palette_sel = (self.palette_sel + n - 1) % n;
            }
        }
        let mut run: Option<usize> = if enter && n > 0 { Some(matches[self.palette_sel]) } else { None };
        let mut close = esc;
        let mut sel = self.palette_sel;

        let accent = self.current_tab.accent();
        let area = egui::Area::new(Id::new("palette")).order(Order::Foreground).anchor(Align2::CENTER_TOP, vec2(0.0, 84.0)).show(ctx, |ui| {
            egui::Frame::default()
                .fill(card_bg())
                .stroke(st(1.0, border()))
                .rounding(rnd(16.0))
                .inner_margin(mg(12.0))
                .shadow(egui::Shadow { offset: Vec2::new(0.0, 10.0), blur: 30.0, spread: 0.0, color: Color32::from_black_alpha(130) })
                .show(ui, |ui| {
                    ui.set_width(520.0);
                    let res = ui.add(
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text(t("pal.hint"))
                            .desired_width(f32::INFINITY)
                            .margin(mgs(12.0, 10.0))
                            .font(egui::FontId::new(16.0, egui::FontFamily::Proportional)),
                    );
                    res.request_focus();
                    if res.changed() {
                        sel = 0;
                    }
                    ui.add_space(8.0);
                    if matches.is_empty() {
                        ui.label(RichText::new(t("pal.none")).color(muted()).size(13.0));
                    }
                    for (row, idx) in matches.iter().enumerate() {
                        let (_, label, hint) = &items[*idx];
                        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
                        let selected = row == sel;
                        if selected || resp.hovered() {
                            ui.painter().rect_filled(rect, rnd(10.0), accent.gamma_multiply(if selected { 0.2 } else { 0.1 }));
                        }
                        if resp.hovered() {
                            sel = row;
                        }
                        ui.painter().text(pos2(rect.left() + 14.0, rect.center().y), Align2::LEFT_CENTER, label, egui::FontId::new(14.5, if selected { bold() } else { egui::FontFamily::Proportional }), if selected { heading() } else { text_c() });
                        if !hint.is_empty() {
                            ui.painter().text(pos2(rect.right() - 14.0, rect.center().y), Align2::RIGHT_CENTER, hint, egui::FontId::new(12.0, egui::FontFamily::Proportional), faint());
                        }
                        if resp.clicked() {
                            run = Some(*idx);
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(t("pal.footer")).color(faint()).size(11.5));
                });
        });
        self.palette_sel = sel;

        if ctx.input(|i| i.pointer.primary_clicked()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !area.response.rect.contains(pos) {
                    close = true;
                }
            }
        }
        if let Some(idx) = run {
            let cmd = items[idx].0.clone();
            self.palette_open = false;
            self.run_cmd(cmd);
        } else if close {
            self.palette_open = false;
        }
    }
}
