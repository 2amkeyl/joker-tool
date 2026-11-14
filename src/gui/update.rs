use super::*;
use std::sync::atomic::Ordering as AtomicOrdering;

fn block_input(ctx: &egui::Context, id: &'static str, dim: u8) {
    egui::Area::new(Id::new(id)).order(Order::Middle).fixed_pos(pos2(0.0, 0.0)).show(ctx, |ui| {
        let (rect, _) = ui.allocate_exact_size(ctx.screen_rect().size(), Sense::click_and_drag());
        ui.painter().rect_filled(rect, rnd(0.0), Color32::from_black_alpha(dim));
    });
}

impl eframe::App for JokerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.load_textures(ctx);
        self.frame_no = self.frame_no.saturating_add(1);
        if self.frame_no == 1 {
            self.refresh_theme();
            // Keep the main window at its designed dimensions to prevent
            // fixed-width panels from colliding when the native window is resized.
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
        }
        if self.cfg.theme == "system" && self.sys_theme_check.elapsed() > Duration::from_millis(1500) {
            self.sys_theme_check = Instant::now();
            if platform::system_prefers_light().unwrap_or(false) != theme::is_light() {
                self.refresh_theme();
            }
        }
        if self.frame_no == 2 {
            if let Some(ms) = ctx.input(|i| i.viewport().monitor_size) {
                let w = 900.0_f32.min(ms.x * 0.88);
                let h = 640.0_f32.min(ms.y * 0.84);
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(vec2(640.0_f32.min(w), 420.0_f32.min(h))));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(w, h)));
            }
        }
        if self.start_minimized && self.frame_no >= 2 {
            self.start_minimized = false;
            if self.tray.is_some() && !self.onboarding_active() {
                self.shared.focused.store(false, AtomicOrdering::Relaxed);
                platform::hide_window(self.hwnd);
            }
        }
        self.shared.focused.store(ctx.input(|i| i.focused), AtomicOrdering::Relaxed);
        self.shared.notify.store(self.cfg.notify, AtomicOrdering::Relaxed);

        let mut got_event = false;
        while let Ok(ev) = self.rx.try_recv() {
            self.handle_event(ev);
            got_event = true;
        }
        let tray_actions = self.tray.as_mut().map(|tray| tray.drain_actions()).unwrap_or_default();
        for action in tray_actions {
            self.handle_tray_action(action);
            got_event = true;
        }
        if self.log_entries.len() > MAX_LOG_LINES {
            let extra = self.log_entries.len() - MAX_LOG_LINES;
            self.log_entries.drain(0..extra);
        }
        self.toasts.retain(|t| t.born.elapsed() < t.life());

        let tab_t = self.tab_changed.elapsed().as_secs_f32();
        let tab_anim = (tab_t / 0.22).clamp(0.0, 1.0);
        let animating = self.is_busy()
            || self.checking_token
            || self.quest_scanning
            || self.badge_fp_busy
            || self.update_checking
            || self.update_installing
            || !self.toasts.is_empty()
            || self.rate_limit_until.is_some()
            || tab_anim < 1.0;
        if got_event {
            ctx.request_repaint();
        } else if animating {
            ctx.request_repaint_after(Duration::from_millis(33));
        } else if self.cfg.theme == "system" {
            ctx.request_repaint_after(Duration::from_millis(1500));
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.tray.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.shared.focused.store(false, AtomicOrdering::Relaxed);
                platform::hide_window(self.hwnd);
                if self.is_busy() {
                    platform::notify("Joker Tool", t("tray.running_bg"));
                }
            } else if self.is_busy() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.confirm_close = true;
            }
        }

        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        let screen = ctx.screen_rect();
        if minimized || screen.width() < 300.0 || screen.height() < 240.0 {
            return;
        }

        let onboarding = self.onboarding_active();
        if !onboarding {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
                self.toggle_palette();
            }
            let mut shortcut_tab = None;
            ctx.input_mut(|i| {
                for (idx, key) in [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5].iter().enumerate() {
                    if i.consume_key(egui::Modifiers::COMMAND, *key) {
                        shortcut_tab = Some(Tab::ALL[idx]);
                    }
                }
            });
            if let Some(tab) = shortcut_tab {
                self.set_tab(tab);
            }
            let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
            if let Some(path) = dropped.into_iter().next() {
                self.on_file_dropped(path);
            }
        }
        let hovering_file = !onboarding && ctx.input(|i| !i.raw.hovered_files.is_empty());

        let narrow = screen.width() < 860.0;
        let accent = self.current_tab.accent();
        let logged_in = self.user_info.is_some();
        let busy = self.is_busy();
        let user_name = self.user_info.as_ref().map(|u| u.display());
        let user_id = self.user_info.as_ref().map(|u| u.id.clone());
        let av = if logged_in { self.avatar_tex.as_ref().map(|t| t.id()) } else { None };
        let logo = self.logo_tex.as_ref().map(|t| t.id());
        let upd = self.update_info.clone();
        let upd_checking = self.update_checking;
        let upd_installing = self.update_installing;
        let mut next_tab = self.current_tab;
        let mut do_check_update = false;
        let mut do_install_update = false;

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(if narrow { 184.0 } else { 224.0 })
            .frame(egui::Frame::default().fill(bg2()).stroke(st(1.0, edge())))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = NAV_GAP;
                ui.add_space(18.0);
                logo_block(ui, narrow, logo);
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    ui.label(RichText::new("Phantom Thieves").color(muted()).size(12.0).italics());
                });
                ui.add_space(18.0);

                let extra_gap = 14.0;
                let start_y = ui.cursor().top();
                let anim_idx = ctx.animate_value_with_time(Id::new("nav_highlight"), self.current_tab.index() as f32, 0.2);
                let hl_y = start_y + anim_idx * (NAV_H + NAV_GAP) + extra_gap * (anim_idx - 3.0).clamp(0.0, 1.0);
                let panel = ui.max_rect();
                let hl = Rect::from_min_size(pos2(panel.left() + 10.0, hl_y), vec2(panel.width() - 20.0, NAV_H));
                ui.painter().rect_filled(hl, rnd(10.0), accent.gamma_multiply(0.16));
                ui.painter().rect_filled(Rect::from_center_size(pos2(hl.left() + 3.0, hl.center().y), vec2(4.0, 24.0)), rnd(2.0), accent);

                for tab in Tab::ALL {
                    if tab == Tab::Settings {
                        ui.add_space(extra_gap);
                    }
                    if nav_item(ui, tab, self.current_tab == tab, self.tab_busy(tab)).clicked() {
                        next_tab = tab;
                    }
                }

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.add_space(12.0);
                    let beta = updater::is_prerelease(updater::CURRENT);
                    ui.label(RichText::new(format!("v{}", updater::CURRENT)).size(11.5).color(if beta { ACCENT_AMBER } else { faint() }));
                    ui.add_space(4.0);
                    if account_card(ui, user_name.clone(), av, accent).clicked() {
                        next_tab = Tab::Settings;
                    }
                    ui.add_space(8.0);
                    if let Some(info) = &upd {
                        let w = (ui.available_width() - 20.0).max(80.0);
                        egui::Frame::default()
                            .fill(brand().gamma_multiply(0.14))
                            .stroke(st(1.0, brand()))
                            .rounding(rnd(12.0))
                            .inner_margin(mg(10.0))
                            .show(ui, |ui| {
                                ui.set_width(w - 20.0);
                                ui.label(RichText::new(t("update.new")).size(13.0).color(heading()).family(bold()));
                                ui.label(RichText::new(tf("update.using", &[&info.version, updater::CURRENT])).size(11.5).color(muted()));
                                ui.add_space(4.0);
                                let label = if upd_installing { t("update.installing") } else { t("update.now") };
                                let can = !upd_installing && !busy;
                                let btn = ui
                                    .add_enabled_ui(can, |ui| {
                                        ui.add_sized([w - 20.0, 30.0], egui::Button::new(RichText::new(label).size(13.0).color(Color32::WHITE).family(bold())).fill(brand()))
                                    })
                                    .inner;
                                if busy && !upd_installing {
                                    btn.on_disabled_hover_text(t("update.stop_first"));
                                } else if btn.clicked() {
                                    do_install_update = true;
                                }
                            });
                    } else {
                        let text = if upd_checking { t("update.checking") } else { t("update.check") };
                        if ui.add_enabled(!upd_checking, egui::Button::new(RichText::new(text).size(12.0).color(muted())).frame(false)).clicked() {
                            do_check_update = true;
                        }
                    }
                });
            });
        self.set_tab(next_tab);
        if do_check_update {
            self.check_update(true);
        }
        if do_install_update {
            self.install_update();
        }

        let mut copy_log = false;
        let max_log_h = (screen.height() * 0.40).max(100.0);
        let log_frame = egui::Frame::default().inner_margin(mgs(16.0, 10.0)).fill(bg1()).stroke(st(1.0, edge()));
        let log_panel = egui::TopBottomPanel::bottom("log_panel").frame(log_frame);
        let log_panel = if self.log_open {
            log_panel.resizable(true).default_height(170.0_f32.min(max_log_h)).height_range(90.0..=max_log_h)
        } else {
            log_panel.resizable(false).exact_height(46.0)
        };
        log_panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                status_dot(ui, accent);
                ui.label(RichText::new(t("log.title")).strong().size(12.0).color(muted()));
                let errs = self.log_entries.iter().filter(|e| e.kind == LogKind::Err).count();
                if self.log_open {
                    ui.add_space(6.0);
                    if ui.selectable_label(self.log_filter == LogFilter::All, RichText::new(tf("log.all", &[&self.log_entries.len().to_string()])).size(12.5)).clicked() {
                        self.log_filter = LogFilter::All;
                    }
                    if ui.selectable_label(self.log_filter == LogFilter::Errors, RichText::new(tf("log.errors", &[&errs.to_string()])).size(12.5)).clicked() {
                        self.log_filter = LogFilter::Errors;
                    }
                } else if errs > 0 {
                    ui.label(RichText::new(tf("log.errors_short", &[&errs.to_string()])).size(12.0).color(RED));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let toggle = if self.log_open { t("log.collapse") } else { t("log.expand") };
                    if ui.small_button(RichText::new(toggle).size(12.0)).clicked() {
                        self.log_open = !self.log_open;
                    }
                    if self.log_open {
                        if ui.small_button(RichText::new(t("log.copy")).size(12.0)).clicked() {
                            copy_log = true;
                        }
                        if ui.small_button(RichText::new(t("log.clear")).size(12.0)).clicked() {
                            self.log_entries.clear();
                        }
                        ui.checkbox(&mut self.auto_scroll, RichText::new(t("log.autoscroll")).size(12.0).color(muted()));
                    }
                });
            });
            if self.log_open {
                ui.add_space(2.0);
                ui.separator();
                egui::ScrollArea::vertical().stick_to_bottom(self.auto_scroll).auto_shrink([false; 2]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    let mut shown = 0;
                    for e in &self.log_entries {
                        if self.log_filter == LogFilter::Errors && e.kind != LogKind::Err {
                            continue;
                        }
                        shown += 1;
                        let color = match e.kind {
                            LogKind::Ok => GREEN,
                            LogKind::Err => RED,
                            LogKind::Info => text_c(),
                        };
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(vec2(64.0, 16.0), Layout::left_to_right(Align::Center), |ui| {
                                ui.set_min_width(64.0);
                                ui.label(RichText::new(&e.time).size(11.5).color(faint()));
                            });
                            ui.add(egui::Label::new(RichText::new(&e.text).color(color).size(12.5)).wrap());
                        });
                    }
                    if shown == 0 {
                        let msg = if self.log_filter == LogFilter::Errors { t("log.no_errors") } else { t("log.empty") };
                        ui.label(RichText::new(msg).italics().size(12.5).color(muted()));
                    }
                });
            }
        });
        if copy_log {
            let text = self.log_entries.iter().map(|e| format!("[{}] {}", e.time, e.text)).collect::<Vec<_>>().join("\n");
            ctx.copy_text(text);
            self.toast(ToastKind::Ok, t("toast.log_copied"));
        }

        let mut do_check = false;
        let mut remember_changed = false;
        let top = egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::default().inner_margin(mgs(24.0, 12.0)).fill(bg2()).stroke(st(1.0, edge())))
            .show(ctx, |ui| {
                let bar_top = ui.max_rect().top() - 12.0;
                ui.ctx().layer_painter(ui.layer_id()).rect_filled(Rect::from_min_max(pos2(screen.left(), bar_top), pos2(screen.right(), bar_top + 3.0)), rnd(0.0), accent);

                if logged_in {
                    ui.horizontal(|ui| {
                        avatar(ui, 36.0, av, user_name.as_deref(), accent);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(user_name.clone().unwrap_or_default()).size(15.0).color(heading()).family(bold()));
                            ui.label(RichText::new(format!("ID {}", user_id.clone().unwrap_or_default())).size(11.5).color(faint()));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some(label) = self.activity_label() {
                                ui.label(RichText::new(label).size(13.0).color(GREEN));
                                pulse_dot(ui, GREEN);
                            }
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("login.label")).size(11.5).color(muted()).family(bold()));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let can_check = !self.checking_token && !self.token_input.trim().is_empty();
                            if ui
                                .add_enabled(can_check, egui::Button::new(RichText::new(t("login.button")).color(Color32::WHITE).family(bold())).fill(brand()))
                                .clicked()
                            {
                                do_check = true;
                            }
                            if ui.checkbox(&mut self.remember, RichText::new(t("login.remember")).size(12.5).color(muted())).changed() {
                                remember_changed = true;
                            }
                            ui.checkbox(&mut self.show_token, RichText::new(t("login.show")).size(12.5).color(muted()));
                            let res = ui.add_enabled(
                                !self.checking_token,
                                egui::TextEdit::singleline(&mut self.token_input)
                                    .password(!self.show_token)
                                    .desired_width((ui.available_width() - 20.0).max(120.0))
                                    .margin(mgs(10.0, 7.0))
                                    .hint_text(t("login.hint")),
                            );
                            if res.changed() {
                                self.token_error = None;
                            }
                            if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_check {
                                do_check = true;
                            }
                        });
                    });
                    if self.checking_token {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new(t("login.checking")).color(muted()).size(12.5));
                        });
                    } else if let Some(err) = &self.token_error {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            status_dot(ui, RED);
                            ui.label(RichText::new(err).color(RED).size(12.5));
                        });
                    }
                }
            });
        let top_bottom = top.response.rect.bottom();
        if remember_changed {
            self.persist_config();
        }
        if do_check {
            self.check_token();
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(mgs(24.0, 16.0)).fill(bg3()))
            .show(ctx, |ui| {
                let r = ui.max_rect().expand2(vec2(24.0, 16.0));
                let deco = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 22);
                let p = ui.painter();
                p.add(egui::Shape::convex_polygon(vec![pos2(r.right() - 260.0, r.top()), pos2(r.right(), r.top()), pos2(r.right(), r.top() + 150.0)], deco, egui::Stroke::NONE));
                p.add(egui::Shape::convex_polygon(vec![pos2(r.left(), r.bottom() - 130.0), pos2(r.left() + 220.0, r.bottom()), pos2(r.left(), r.bottom())], deco, egui::Stroke::NONE));

                let top_pad = ((ui.available_height() - 520.0) * 0.12).clamp(0.0, 60.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let avail = ui.available_width() - 14.0;
                    let w = avail.min(CONTENT_MAX_WIDTH);
                    let pad = ((avail - w) / 2.0).max(0.0);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(pad);
                        ui.vertical(|ui| {
                            ui.set_width(w);
                            ui.spacing_mut().item_spacing.x = 12.0;
                            let ease = 1.0 - (1.0 - tab_anim) * (1.0 - tab_anim);
                            ui.set_opacity(0.3 + 0.7 * ease);
                            ui.add_space(top_pad + (1.0 - ease) * 12.0);
                            if let Some(until) = self.rate_limit_until {
                                let left = until.saturating_duration_since(Instant::now()).as_secs();
                                if until <= Instant::now() {
                                    self.rate_limit_until = None;
                                } else {
                                    egui::Frame::default()
                                        .fill(ACCENT_AMBER.gamma_multiply(0.14))
                                        .stroke(st(1.0, ACCENT_AMBER.gamma_multiply(0.55)))
                                        .rounding(rnd(12.0))
                                        .inner_margin(mgs(14.0, 10.0))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label(RichText::new(tf("rate.waiting", &[&fmt_mmss(left as f64 + 1.0)])).color(heading()).family(bold()).size(13.5));
                                            });
                                            ui.label(RichText::new(t("rate.sub")).size(12.0).color(muted()));
                                        });
                                    ui.add_space(12.0);
                                }
                            }
                            match self.current_tab {
                                Tab::Quest => self.render_quest_tab(ui),
                                Tab::Badge => self.render_badge_tab(ui),
                                Tab::HypeSquad => self.render_hs_tab(ui),
                                Tab::Stats => self.render_stats_tab(ui),
                                Tab::Settings => self.render_settings_tab(ui),
                            }
                            ui.add_space(24.0);
                        });
                    });
                });
            });

        if hovering_file {
            let painter = ctx.layer_painter(egui::LayerId::new(Order::Foreground, Id::new("drop_overlay")));
            painter.rect_filled(screen, rnd(0.0), Color32::from_black_alpha(190));
            let box_rect = Rect::from_center_size(screen.center(), vec2(440.0, 150.0));
            painter.rect_filled(box_rect, rnd(18.0), card_bg());
            painter.rect_stroke(box_rect, rnd(18.0), st(2.0, ACCENT_AMBER));
            painter.text(box_rect.center() - vec2(0.0, 14.0), Align2::CENTER_CENTER, t("drop.title"), egui::FontId::new(18.0, bold()), Color32::WHITE);
            painter.text(box_rect.center() + vec2(0.0, 16.0), Align2::CENTER_CENTER, t("drop.sub"), egui::FontId::new(13.0, egui::FontFamily::Proportional), muted());
        }

        if !self.toasts.is_empty() {
            egui::Area::new(Id::new("toasts"))
                .order(Order::Foreground)
                .anchor(Align2::RIGHT_TOP, vec2(-16.0, top_bottom + 12.0))
                .interactable(false)
                .show(ctx, |ui| {
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        for tst in self.toasts.iter().rev() {
                            toast_card(ui, tst.kind, &tst.text, tst.alpha());
                        }
                    });
                });
        }

        if onboarding {
            self.render_onboarding(ctx, screen);
        } else if self.palette_open {
            self.render_palette(ctx);
        }

        if self.confirm_close {
            block_input(ctx, "close_block", 150);
            let mut stay = false;
            let mut leave = false;
            egui::Window::new(t("close.title"))
                .order(Order::Foreground)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(340.0);
                    ui.label(RichText::new(t("close.body")).color(text_c()));
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new(t("close.leave")).color(Color32::WHITE).family(bold())).fill(STOP)).clicked() {
                            leave = true;
                        }
                        if ui.button(t("close.stay")).clicked() {
                            stay = true;
                        }
                    });
                });
            if stay {
                self.confirm_close = false;
            }
            if leave {
                self.stop_all();
                self.allow_close = true;
                self.confirm_close = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

pub(super) fn dim_block(ctx: &egui::Context, id: &'static str, dim: u8) {
    block_input(ctx, id, dim);
}
