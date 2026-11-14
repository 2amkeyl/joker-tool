#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Open,
    OpenQuest,
    OpenBadge,
    OpenHypeSquad,
    StopTasks,
    OpenStats,
    OpenSettings,
    Quit,
}

pub struct TrayLabels<'a> {
    pub open: &'a str,
    pub quest: &'a str,
    pub badge: &'a str,
    pub hypesquad: &'a str,
    pub stop_tasks: &'a str,
    pub stats: &'a str,
    pub settings: &'a str,
    pub quit: &'a str,
}

#[cfg(windows)]
mod imp {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use super::{TrayAction, TrayLabels};
    use std::sync::mpsc::{self, Receiver};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE, SW_SHOW};

    pub struct Tray {
        _icon: TrayIcon,
        actions: Receiver<TrayAction>,
    }

    impl Tray {
        pub fn drain_actions(&mut self) -> Vec<TrayAction> {
            self.actions.try_iter().collect()
        }
    }

    pub fn find_hwnd(cc: &eframe::CreationContext<'_>) -> isize {
        match cc.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
            _ => 0,
        }
    }

    pub fn show_window(hwnd: isize) {
        if hwnd == 0 { return; }
        unsafe {
            ShowWindow(hwnd as _, SW_SHOW);
            ShowWindow(hwnd as _, SW_RESTORE);
            SetForegroundWindow(hwnd as _);
        }
    }

    pub fn hide_window(hwnd: isize) {
        if hwnd == 0 { return; }
        unsafe { ShowWindow(hwnd as _, SW_HIDE); }
    }

    fn tray_icon_image() -> Option<Icon> {
        let img = image::load_from_memory(include_bytes!("../assets/icon.png")).ok()?.resize_exact(32, 32, image::imageops::FilterType::Lanczos3).to_rgba8();
        Icon::from_rgba(img.into_raw(), 32, 32).ok()
    }

    pub fn create_tray(ctx: eframe::egui::Context, hwnd: isize, labels: TrayLabels<'_>) -> Option<Tray> {
        if hwnd == 0 { return None; }
        let menu = Menu::new();

        let open = MenuItem::with_id("open", labels.open, true, None);
        let quest = MenuItem::with_id("page.quest", labels.quest, true, None);
        let badge = MenuItem::with_id("page.badge", labels.badge, true, None);
        let hypesquad = MenuItem::with_id("page.hypesquad", labels.hypesquad, true, None);
        let stop_tasks = MenuItem::with_id("tasks.stop", labels.stop_tasks, true, None);
        let stats = MenuItem::with_id("stats.open", labels.stats, true, None);
        let settings = MenuItem::with_id("settings.open", labels.settings, true, None);
        let quit = MenuItem::with_id("quit", labels.quit, true, None);

        menu.append_items(&[
            &open,
            &PredefinedMenuItem::separator(),
            &quest,
            &badge,
            &hypesquad,
            &PredefinedMenuItem::separator(),
            &stop_tasks,
            &PredefinedMenuItem::separator(),
            &stats,
            &settings,
            &PredefinedMenuItem::separator(),
            &quit,
        ]).ok()?;

        let icon = TrayIconBuilder::new().with_menu(Box::new(menu)).with_menu_on_left_click(false)
            .with_tooltip("Joker Tool").with_icon(tray_icon_image()?).build().ok()?;

        let (action_tx, action_rx) = mpsc::channel();
        let ctx_menu = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let action = match e.id.0.as_str() {
                "open" => TrayAction::Open,
                "page.quest" => TrayAction::OpenQuest,
                "page.badge" => TrayAction::OpenBadge,
                "page.hypesquad" => TrayAction::OpenHypeSquad,
                "tasks.stop" => TrayAction::StopTasks,
                "stats.open" => TrayAction::OpenStats,
                "settings.open" => TrayAction::OpenSettings,
                "quit" => TrayAction::Quit,
                _ => return,
            };
            if action_tx.send(action).is_ok() {
                ctx_menu.request_repaint();
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_window(hwnd); ctx.request_repaint();
            }
        }));
        Some(Tray { _icon: icon, actions: action_rx })
    }

    pub fn notify(title: &str, body: &str) {
        use tauri_winrt_notification::Toast;
        let _ = Toast::new(Toast::POWERSHELL_APP_ID).title(title).text1(body).show();
    }

    pub fn acquire_single_instance(retry_ms: u64) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexW;
        let name: Vec<u16> = "Local\\JokerTool.SingleInstance".encode_utf16().chain(std::iter::once(0)).collect();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(retry_ms);
        loop {
            let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
            let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            if !handle.is_null() && !already { return true; }
            if !handle.is_null() { unsafe { CloseHandle(handle); } }
            if std::time::Instant::now() >= deadline { return false; }
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
    }

    pub fn focus_existing() {
        use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
        let title: Vec<u16> = "Joker Tool".encode_utf16().chain(std::iter::once(0)).collect();
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if !hwnd.is_null() { show_window(hwnd as isize); }
    }

    /// Returns `Some(true)` when Windows prefers light app surfaces, `Some(false)` for dark.
    pub fn system_prefers_light() -> Option<bool> {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
            .ok()?;
        let value: u32 = key.get_value("AppsUseLightTheme").ok()?;
        Some(value != 0)
    }

    /// Best-effort tinting of native tray menus. The uxtheme ordinals are undocumented
    /// Windows implementation details, so failure is intentionally ignored.
    pub fn set_menu_theme(mode: u8) {
        // FreeLibrary is exported from Win32::Foundation in windows-sys 0.59.
        use windows_sys::Win32::Foundation::FreeLibrary;
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

        let library_name: Vec<u16> = "uxtheme.dll".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let library = LoadLibraryW(library_name.as_ptr());
            if library.is_null() {
                return;
            }

            let set_mode = GetProcAddress(library, 135usize as *const u8);
            let flush = GetProcAddress(library, 136usize as *const u8);
            if let Some(function) = set_mode {
                let function: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(function);
                let preferred = match mode {
                    1 => 2, // dark
                    2 => 3, // light
                    _ => 1, // system/default
                };
                let _ = function(preferred);
            }
            if let Some(function) = flush {
                let function: unsafe extern "system" fn() = std::mem::transmute(function);
                function();
            }

            let _ = FreeLibrary(library);
        }
    }

    pub fn os_version() -> String {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("cmd").args(["/C", "ver"]).creation_flags(0x0800_0000)
            .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty()).unwrap_or_else(|| "Windows".to_string())
    }

    const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const RUN_NAME: &str = "JokerTool";

    pub fn set_autostart(enabled: bool) -> Result<(), String> {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
        use winreg::RegKey;
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE).map_err(|e| e.to_string())?;
        if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            key.set_value(RUN_NAME, &format!("\"{}\" --minimized", exe.display())).map_err(|e| e.to_string())
        } else {
            match key.delete_value(RUN_NAME) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{TrayAction, TrayLabels};
    pub struct Tray;
    impl Tray {
        pub fn drain_actions(&mut self) -> Vec<TrayAction> { Vec::new() }
    }
    pub fn find_hwnd(_cc: &eframe::CreationContext<'_>) -> isize { 0 }
    pub fn show_window(_hwnd: isize) {}
    pub fn hide_window(_hwnd: isize) {}
    pub fn create_tray(_ctx: eframe::egui::Context, _hwnd: isize, _labels: TrayLabels<'_>) -> Option<Tray> { None }
    pub fn notify(_title: &str, _body: &str) {}
    pub fn acquire_single_instance(_retry_ms: u64) -> bool { true }
    pub fn focus_existing() {}
    pub fn system_prefers_light() -> Option<bool> { None }
    pub fn set_menu_theme(_mode: u8) {}
    pub fn os_version() -> String { std::env::consts::OS.to_string() }
    pub fn set_autostart(_enabled: bool) -> Result<(), String> { Err("unsupported".into()) }
}
pub use imp::*;