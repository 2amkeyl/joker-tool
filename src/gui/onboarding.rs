use super::update::dim_block;
use super::*;

impl JokerApp {
    pub(super) fn render_onboarding(&mut self, ctx: &egui::Context, screen: Rect) {
        dim_block(ctx, "onboard_block", 215);
        let step = self.onboard_step;
        let mut next = false;
        let mut back = false;
        let mut finish = false;
        let mut new_lang: Option<Lang> = None;
        let logo = self.logo_tex.as_ref().map(|t| t.id());
        let ack = &mut self.onboard_ack;
        let cur_lang = i18n::lang();

        egui::Area::new(Id::new("onboard")).order(Order::Foreground).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            egui::Frame::default()
                .fill(card_bg())
                .stroke(st(1.0, border()))
                .rounding(rnd(18.0))
                .inner_margin(mg(28.0))
                .shadow(egui::Shadow { offset: Vec2::new(0.0, 10.0), blur: 30.0, spread: 0.0, color: Color32::from_black_alpha(150) })
                .show(ui, |ui| {
                    ui.set_width((screen.width() - 90.0).clamp(300.0, 520.0));

                    ui.horizontal(|ui| {
                        if let Some(id) = logo {
                            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                            ui.painter().image(id, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            ui.painter().rect_stroke(r, rnd(10.0), st(1.5, brand()));
                        }
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(RichText::new("JOKER TOOL").size(18.0).color(heading()).family(bold()));
                            ui.label(RichText::new(tf("onb.step", &[&(step + 1).to_string(), "3"])).size(12.0).color(muted()));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            for i in (0..3).rev() {
                                let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                let col = if i == step { brand() } else if i < step { brand().gamma_multiply(0.5) } else { border() };
                                ui.painter().circle_filled(r.center(), if i == step { 5.0 } else { 3.5 }, col);
                            }
                        });
                    });
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(14.0);

                    match step {
                        0 => {
                            ui.label(RichText::new(t("onb.welcome_title")).size(22.0).color(heading()).family(bold()));
                            ui.add_space(6.0);
                            ui.add(egui::Label::new(RichText::new(t("onb.welcome_body")).size(14.0).color(text_c())).wrap());
                            ui.add_space(14.0);
                            ui.label(RichText::new(t("onb.language")).size(12.5).color(muted()));
                            ui.horizontal(|ui| {
                                if ui.selectable_label(cur_lang == Lang::Vi, RichText::new("Tiếng Việt").size(14.0)).clicked() {
                                    new_lang = Some(Lang::Vi);
                                }
                                if ui.selectable_label(cur_lang == Lang::En, RichText::new("English").size(14.0)).clicked() {
                                    new_lang = Some(Lang::En);
                                }
                            });
                        }
                        1 => {
                            ui.label(RichText::new(t("onb.login_title")).size(22.0).color(heading()).family(bold()));
                            ui.add_space(6.0);
                            for key in ["onb.login_1", "onb.login_2", "onb.login_3"] {
                                ui.horizontal_top(|ui| {
                                    ui.label(RichText::new("•").color(brand()));
                                    ui.add(egui::Label::new(RichText::new(t(key)).size(14.0).color(text_c())).wrap());
                                });
                                ui.add_space(2.0);
                            }
                        }
                        _ => {
                            ui.label(RichText::new(t("onb.risk_title")).size(22.0).color(ACCENT_AMBER).family(bold()));
                            ui.add_space(6.0);
                            ui.add(egui::Label::new(RichText::new(t("onb.risk_body")).size(14.0).color(text_c())).wrap());
                            ui.add_space(12.0);
                            ui.checkbox(ack, RichText::new(t("onb.risk_ack")).size(13.5).color(text_c()));
                        }
                    }

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        if step > 0 && ui.button(t("common.back")).clicked() {
                            back = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if step < 2 {
                                if ui.add(egui::Button::new(RichText::new(t("common.next")).color(Color32::WHITE).family(bold())).fill(brand()).min_size(vec2(110.0, 34.0))).clicked() {
                                    next = true;
                                }
                            } else if ui.add_enabled(*ack, egui::Button::new(RichText::new(t("onb.start")).color(Color32::WHITE).family(bold())).fill(brand()).min_size(vec2(130.0, 34.0))).clicked() {
                                finish = true;
                            }
                        });
                    });
                });
        });

        if let Some(l) = new_lang {
            self.set_language(l);
        }
        if next {
            self.onboard_step = (self.onboard_step + 1).min(2);
        }
        if back {
            self.onboard_step = self.onboard_step.saturating_sub(1);
        }
        if finish {
            self.cfg.onboarded = true;
            self.onboard_step = usize::MAX;
            self.persist_config();
            if self.remember && !self.token_input.trim().is_empty() && self.user_info.is_none() {
                self.check_token();
            }
        }
    }
}
