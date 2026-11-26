use super::*;

enum MenuAction {
    Settings,
    Logout,
    CopyId(String),
}

impl JokerApp {
    pub(super) fn render_account_menu(&mut self, ctx: &egui::Context) {
        if !self.account_menu_open { return; }
        let anchor_rect = self.top_account_rect;
        let pivot = Align2::RIGHT_TOP;
        let progress = ctx.animate_bool_with_time(egui::Id::new("account_menu_open"), true, 0.18);
        let pos = pos2(anchor_rect.right(), anchor_rect.bottom() + 8.0 + (1.0 - progress) * 8.0);

        let user = self.user_info.as_ref().map(|u| (u.display(), u.username.clone(), u.global_name.clone().filter(|g| !g.is_empty()), u.id.clone()));
        let av = if user.is_some() { self.avatar_tex.as_ref().map(|t| t.id()) } else { None };
        let (q_ok, secs, games) = {
            let s = self.shared.stats.lock();
            (s.quests_ok, s.secs, s.games)
        };
        let busy = self.is_busy();
        let accent = self.current_tab.accent();
        let mut action: Option<MenuAction> = None;

        let area = egui::Area::new(egui::Id::new("account_menu"))
            .order(egui::Order::Foreground)
            .pivot(pivot)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui.set_opacity(progress);
                egui::Frame::default()
                    .fill(card_bg())
                    .stroke(st(1.0, border()))
                    .rounding(rnd(16.0))
                    .inner_margin(mg(16.0))
                    .shadow(egui::epaint::Shadow { offset: vec2(0.0, 10.0), blur: 30.0, spread: 0.0, color: Color32::from_black_alpha(if is_light() { 50 } else { 130 }) })
                    .show(ui, |ui| {
                        ui.set_width(300.0);
                        ui.spacing_mut().item_spacing.y = 4.0;

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 14.0;
                            avatar(ui, 56.0, av, user.as_ref().map(|u| u.0.as_str()), accent);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                match &user {
                                    Some((name, username, global, id)) => {
                                        ui.add(egui::Label::new(RichText::new(name).size(18.0).color(heading()).family(bold())).truncate());
                                        if global.is_some() {
                                            ui.label(RichText::new(format!("@{}", username)).size(12.5).color(muted()));
                                        }
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("ID {}", id)).size(11.5).color(faint()));
                                            let copy = ui.add(egui::Button::new(RichText::new(egui_phosphor::regular::COPY).size(14.0).color(muted())).frame(false));
                                            if copy.on_hover_text(t("profile.copy_id")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                                action = Some(MenuAction::CopyId(id.clone()));
                                            }
                                        });
                                    }
                                    None => {
                                        ui.label(RichText::new(t("profile.guest")).size(18.0).color(heading()).family(bold()));
                                        ui.add(egui::Label::new(RichText::new(t("account.hint_out")).size(12.0).color(muted())).wrap());
                                    }
                                }
                            });
                        });
                        ui.add_space(6.0);
                        if user.is_some() {
                            status_pill(ui, t("profile.signed_in"), GREEN);
                        } else {
                            status_pill(ui, t("profile.signed_out"), faint());
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.label(RichText::new(t("account.stats")).size(11.5).color(faint()).family(bold()));
                        ui.add_space(2.0);
                        ui.columns(3, |cols| {
                            for (col, (value, label, color)) in cols.iter_mut().zip([
                                (q_ok.to_string(), t("stats.chip_quests"), GREEN),
                                (fmt_dur(secs), t("stats.chip_time"), accent),
                                (games.to_string(), t("stats.chip_games"), ACCENT_AMBER),
                            ]) {
                                col.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.label(RichText::new(value).size(17.0).color(color).family(bold()));
                                    ui.add(egui::Label::new(RichText::new(label).size(9.5).color(faint())).wrap());
                                });
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        if menu_row(ui, egui_phosphor::regular::GEAR_SIX, t("account.settings"), text_c()).clicked() {
                            action = Some(MenuAction::Settings);
                        }
                        if user.is_some() {
                            let resp = ui.add_enabled_ui(!busy, |ui| menu_row(ui, egui_phosphor::regular::SIGN_OUT, t("profile.logout"), RED)).inner;
                            if resp.clicked() && !busy {
                                action = Some(MenuAction::Logout);
                            }
                            if busy {
                                resp.on_hover_text(t("toast.stop_first"));
                            }
                        }
                    });
            });

        let menu_rect = area.response.rect;
        let close = ctx.input(|i| {
            i.key_pressed(egui::Key::Escape)
                || (i.pointer.any_pressed()
                    && i.pointer.interact_pos().map(|p| !menu_rect.expand(2.0).contains(p) && !anchor_rect.expand(2.0).contains(p)).unwrap_or(false))
        });
        if close {
            self.account_menu_open = false;
        }

        match action {
            Some(MenuAction::Settings) => {
                self.account_menu_open = false;
                self.set_tab(Tab::Settings);
            }
            Some(MenuAction::Logout) => {
                self.account_menu_open = false;
                self.logout();
            }
            Some(MenuAction::CopyId(id)) => {
                ctx.copy_text(id);
                self.toast(ToastKind::Ok, t("toast.id_copied"));
            }
            None => {}
        }
    }
}
