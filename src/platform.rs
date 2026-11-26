#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE};

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
        let title: Vec<u16> = "Joker Tool".encode_utf16().chain(std::iter::once(0)).collect();
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if !hwnd.is_null() {
            unsafe {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
        }
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

    /// Remove the legacy Run-key value now that the startup option has been removed.
    pub fn clear_legacy_autostart() {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
        use winreg::RegKey;

        const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
            let _ = key.delete_value("JokerTool");
        }
    }

    pub fn os_version() -> String {
        std::process::Command::new("cmd").args(["/C", "ver"]).creation_flags(0x0800_0000)
            .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty()).unwrap_or_else(|| "Windows".to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn notify(_title: &str, _body: &str) {}
    pub fn acquire_single_instance(_retry_ms: u64) -> bool { true }
    pub fn focus_existing() {}
    pub fn system_prefers_light() -> Option<bool> { None }
    pub fn clear_legacy_autostart() {}
    pub fn os_version() -> String { std::env::consts::OS.to_string() }
}

pub use imp::*;
