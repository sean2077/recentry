use std::{mem::size_of, path::Path, ptr::null_mut};

use recentry_protocol::Hotkey;
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, GetLastError, HWND},
    System::Registry::{
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
        RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
    },
    UI::{
        Input::KeyboardAndMouse::{
            MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
            UnregisterHotKey,
        },
        Shell::{
            NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIM_ADD, NIM_DELETE,
            NIM_MODIFY, NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
        },
        WindowsAndMessaging::{IDI_APPLICATION, LoadIconW, WM_APP},
    },
};

pub const HOTKEY_ID: i32 = 1;
pub const WM_TRAY: u32 = WM_APP + 1;
pub const WM_CONFIG_CHANGED: u32 = WM_APP + 2;

pub trait HostPlatform {
    fn register_hotkey(&mut self, hotkey: &Hotkey) -> Result<(), String>;
    fn notify(&self, title: &str, message: &str, error: bool);
    fn set_autostart(&self, enabled: bool, executable: &Path) -> Result<(), String>;
}

pub struct WindowsHostPlatform {
    hwnd: isize,
    hotkey_registered: bool,
    tray_installed: bool,
}

unsafe impl Send for WindowsHostPlatform {}

impl WindowsHostPlatform {
    pub fn install(hwnd: HWND) -> Result<Self, String> {
        let mut platform = Self {
            hwnd: hwnd as isize,
            hotkey_registered: false,
            tray_installed: false,
        };
        let mut tray = platform.tray_data();
        tray.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        tray.uCallbackMessage = WM_TRAY;
        tray.hIcon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };
        copy_wide(&mut tray.szTip, "Recentry");
        if unsafe { Shell_NotifyIconW(NIM_ADD, &tray) } == 0 {
            return Err(format!("Shell_NotifyIconW failed: {}", unsafe {
                GetLastError()
            }));
        }
        platform.tray_installed = true;
        unsafe {
            tray.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            Shell_NotifyIconW(NIM_SETVERSION, &tray);
        }
        Ok(platform)
    }

    pub fn uninstall(&mut self) {
        if self.hotkey_registered {
            unsafe { UnregisterHotKey(self.hwnd as HWND, HOTKEY_ID) };
            self.hotkey_registered = false;
        }
        if self.tray_installed {
            let tray = self.tray_data();
            unsafe { Shell_NotifyIconW(NIM_DELETE, &tray) };
            self.tray_installed = false;
        }
    }

    fn tray_data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd as HWND,
            uID: 1,
            ..Default::default()
        }
    }
}

impl HostPlatform for WindowsHostPlatform {
    fn register_hotkey(&mut self, hotkey: &Hotkey) -> Result<(), String> {
        if self.hotkey_registered {
            unsafe { UnregisterHotKey(self.hwnd as HWND, HOTKEY_ID) };
            self.hotkey_registered = false;
        }
        let (modifiers, key) = win32_hotkey(hotkey)?;
        if unsafe { RegisterHotKey(self.hwnd as HWND, HOTKEY_ID, modifiers, key) } == 0 {
            return Err(format!("RegisterHotKey failed with error {}", unsafe {
                GetLastError()
            }));
        }
        self.hotkey_registered = true;
        Ok(())
    }

    fn notify(&self, title: &str, message: &str, error: bool) {
        let mut tray = self.tray_data();
        tray.uFlags = NIF_INFO;
        copy_wide(&mut tray.szInfoTitle, title);
        copy_wide(&mut tray.szInfo, message);
        tray.dwInfoFlags = if error { NIIF_ERROR } else { NIIF_INFO };
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &tray);
        }
    }

    fn set_autostart(&self, enabled: bool, executable: &Path) -> Result<(), String> {
        set_autostart(enabled, executable)
    }
}

pub fn win32_hotkey(hotkey: &Hotkey) -> Result<(u32, u32), String> {
    hotkey.validate_key()?;
    let mut modifiers = MOD_NOREPEAT;
    if hotkey.ctrl {
        modifiers |= MOD_CONTROL;
    }
    if hotkey.alt {
        modifiers |= MOD_ALT;
    }
    if hotkey.shift {
        modifiers |= MOD_SHIFT;
    }
    if hotkey.win {
        modifiers |= MOD_WIN;
    }
    let key = hotkey.key.to_ascii_uppercase();
    let virtual_key = if key.len() == 1 {
        u32::from(key.as_bytes()[0])
    } else {
        let number = key[1..]
            .parse::<u32>()
            .map_err(|_| "invalid function key".to_owned())?;
        0x70 + number - 1
    };
    Ok((modifiers, virtual_key))
}

trait ValidateHotkeyKey {
    fn validate_key(&self) -> Result<(), String>;
}

impl ValidateHotkeyKey for Hotkey {
    fn validate_key(&self) -> Result<(), String> {
        let config = recentry_protocol::Config {
            hotkey: self.clone(),
            ..Default::default()
        };
        config.validate()
    }
}

pub fn set_autostart(enabled: bool, executable: &Path) -> Result<(), String> {
    let key_path = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let value_name = wide("Recentry");
    let mut key = null_mut();
    let result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null_mut(),
            &mut key,
            null_mut(),
        )
    };
    if result != 0 {
        return Err(format!(
            "opening the autostart registry key failed: {result}"
        ));
    }
    let result = if enabled {
        let command = wide(&autostart_command(executable));
        unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * size_of::<u16>()) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, value_name.as_ptr()) }
    };
    unsafe { RegCloseKey(key) };
    if result == 0
        || (!enabled && (result == ERROR_FILE_NOT_FOUND || result == ERROR_PATH_NOT_FOUND))
    {
        Ok(())
    } else {
        Err(format!("updating autostart failed: {result}"))
    }
}

fn autostart_command(executable: &Path) -> String {
    format!("\"{}\" --background", executable.to_string_lossy())
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn copy_wide<const N: usize>(target: &mut [u16; N], value: &str) {
    target.fill(0);
    for (slot, unit) in target.iter_mut().zip(value.encode_utf16().chain(Some(0))) {
        *slot = unit;
    }
}

#[cfg(test)]
mod tests {
    use recentry_protocol::Hotkey;

    use super::*;

    #[test]
    fn converts_default_and_function_hotkeys() {
        let (_, key) = win32_hotkey(&Hotkey::default()).unwrap();
        assert_eq!(key, u32::from(b'R'));
        let (_, key) = win32_hotkey(&Hotkey {
            key: "F12".to_owned(),
            ..Hotkey::default()
        })
        .unwrap();
        assert_eq!(key, 0x7b);
    }

    #[test]
    fn autostart_uses_non_interactive_background_mode() {
        assert_eq!(
            autostart_command(Path::new(r"C:\Program Files\Recentry\recentry.exe")),
            r#""C:\Program Files\Recentry\recentry.exe" --background"#
        );
    }
}
