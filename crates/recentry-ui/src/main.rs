#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(windows, allow(unsafe_op_in_unsafe_fn))]

#[cfg(not(windows))]
fn main() {
    eprintln!("Recentry's native UI is not available in this development build.");
}

#[cfg(windows)]
mod windows_main {
    use std::{
        env, fs,
        mem::{size_of, zeroed},
        path::PathBuf,
        ptr::{null, null_mut},
        sync::OnceLock,
        thread,
        time::{Duration, Instant},
    };

    use recentry_core::{
        DiscoveryEnvironment, ProjectOpener, RecentProjectProvider, VsCodeOpener,
        VsCodeRecentProvider, discover_vscode, window_state_candidates,
    };
    use recentry_ipc::{connect, request};
    use recentry_protocol::{
        Config, HostCommand, HostResponse, Hotkey, Language, UiCommand, UiResponse,
    };
    use recentry_ui::LauncherState;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Globalization::GetUserDefaultUILanguage,
        Graphics::Gdi::{
            CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_GUI_FONT,
            DT_END_ELLIPSIS, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW,
            FF_MODERN, FIXED_PITCH, FW_NORMAL, FillRect, GetMonitorInfoW, GetStockObject, HBRUSH,
            HDC, HFONT, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
            OUT_DEFAULT_PRECIS, ScreenToClient, SelectObject, SetBkColor, SetBkMode, SetTextColor,
            TRANSPARENT, UpdateWindow,
        },
        System::{
            LibraryLoader::GetModuleHandleW,
            Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
            Threading::{CreateEventW, INFINITE, SetEvent, WaitForSingleObject},
        },
        UI::{
            Controls::{
                BST_CHECKED, DRAWITEMSTRUCT, EM_SETCUEBANNER, EM_SETMARGINS, EM_SETSEL,
                InitCommonControls, NMTTDISPINFOW, SetWindowTheme, TOOLTIPS_CLASSW, TTF_IDISHWND,
                TTF_SUBCLASS, TTM_ADDTOOLW, TTM_SETMAXTIPWIDTH, TTN_GETDISPINFOW, TTTOOLINFOW,
            },
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
                SetProcessDpiAwarenessContext,
            },
            Input::KeyboardAndMouse::{
                GetFocus, GetKeyState, SetFocus, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_F1, VK_F24,
                VK_LWIN, VK_MENU, VK_RETURN, VK_RWIN, VK_SHIFT, VK_UP,
            },
            WindowsAndMessaging::{
                BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, CB_ADDSTRING, CB_GETCURSEL,
                CB_RESETCONTENT, CB_SETCURSEL, CBS_DROPDOWNLIST, CREATESTRUCTW, CS_DROPSHADOW,
                CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
                DestroyWindow, DispatchMessageW, EC_LEFTMARGIN, EC_RIGHTMARGIN, EN_CHANGE,
                ES_AUTOHSCROLL, ES_READONLY, GA_ROOT, GetAncestor, GetClientRect, GetCursorPos,
                GetForegroundWindow, GetMessageW, GetWindowTextLengthW, GetWindowTextW, HMENU,
                HWND_TOPMOST, IDC_ARROW, IsWindowVisible, KillTimer, LB_ADDSTRING, LB_GETCURSEL,
                LB_ITEMFROMPOINT, LB_RESETCONTENT, LB_SETCURSEL, LB_SETITEMHEIGHT, LBN_DBLCLK,
                LBN_SELCHANGE, LBS_HASSTRINGS, LBS_NOTIFY, LBS_OWNERDRAWFIXED, LoadCursorW,
                MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MSG, MessageBoxW, MoveWindow,
                PostMessageW, PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOWNORMAL,
                SWP_NOACTIVATE, SWP_SHOWWINDOW, SendMessageW, SetForegroundWindow, SetTimer,
                SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage, WA_INACTIVE,
                WM_ACTIVATE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLOREDIT,
                WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DPICHANGED, WM_DRAWITEM,
                WM_ERASEBKGND, WM_KEYDOWN, WM_NOTIFY, WM_SETFONT, WM_SETTINGCHANGE, WM_SIZE,
                WM_SYSKEYDOWN, WM_THEMECHANGED, WM_TIMER, WNDCLASSW, WS_BORDER, WS_CAPTION,
                WS_CHILD, WS_EX_CLIENTEDGE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_SYSMENU,
                WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
            },
        },
    };

    const EDIT_ID: usize = 10;
    const LIST_ID: usize = 11;
    const STATUS_ID: usize = 12;
    const HINT_ID: usize = 13;
    const LANGUAGE_ID: usize = 20;
    const HOTKEY_ID: usize = 21;
    const AUTOSTART_ID: usize = 22;
    const VSCODE_PATH_ID: usize = 23;
    const SAVE_ID: usize = 24;
    const CANCEL_ID: usize = 25;
    const WM_PIPE_COMMAND: u32 = WM_APP + 10;
    const IDLE_TIMER: usize = 1;
    const FOCUS_TIMER: usize = 2;
    const IDLE_EXIT_AFTER: Duration = Duration::from_secs(15 * 60);
    const LAUNCHER_WIDTH: i32 = 760;
    const LAUNCHER_HEADER_HEIGHT: i32 = 36;
    const LAUNCHER_ROW_HEIGHT: i32 = 24;
    const LAUNCHER_VISIBLE_ROWS: i32 = 12;
    const LAUNCHER_NONCLIENT_HEIGHT: i32 = 4;
    const STATIC_RIGHT: u32 = 0x0002;
    const STATIC_NOPREFIX: u32 = 0x0080;
    const STATIC_CENTER_IMAGE: u32 = 0x0200;
    const STATIC_NOTIFY: u32 = 0x0100;

    static APP: OnceLock<usize> = OnceLock::new();

    struct Theme {
        dark: bool,
        surface: u32,
        text: u32,
        muted: u32,
        accent: u32,
        selected: u32,
        selected_text: u32,
        background_brush: HBRUSH,
        surface_brush: HBRUSH,
    }

    struct SettingsControls {
        hwnd: HWND,
        language_label: HWND,
        language: HWND,
        hotkey_label: HWND,
        hotkey: HWND,
        autostart: HWND,
        vscode_label: HWND,
        vscode_path: HWND,
        save: HWND,
        cancel: HWND,
    }

    impl Default for SettingsControls {
        fn default() -> Self {
            Self {
                hwnd: null_mut(),
                language_label: null_mut(),
                language: null_mut(),
                hotkey_label: null_mut(),
                hotkey: null_mut(),
                autostart: null_mut(),
                vscode_label: null_mut(),
                vscode_path: null_mut(),
                save: null_mut(),
                cancel: null_mut(),
            }
        }
    }

    struct AppState {
        hwnd: HWND,
        prompt: HWND,
        edit: HWND,
        hint: HWND,
        list: HWND,
        status: HWND,
        tooltip: HWND,
        settings: SettingsControls,
        launcher: LauncherState,
        config: Config,
        config_path: PathBuf,
        host_pipe: String,
        diagnostics: Vec<String>,
        pending_hotkey: Hotkey,
        hidden_since: Option<Instant>,
        launcher_had_focus: bool,
        dpi: u32,
        theme: Theme,
        launcher_font: HFONT,
        tooltip_text: [u16; 2048],
    }

    struct PendingCommand {
        command: UiCommand,
        response: UiResponse,
        done: HANDLE,
    }

    unsafe fn app() -> &'static mut AppState {
        &mut *(*APP.get().expect("app state") as *mut AppState)
    }

    pub fn run() {
        let Some((pipe_name, host_pipe, config_path)) = parse_args() else {
            return;
        };
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            InitCommonControls();
        }
        let config = load_config(&config_path).unwrap_or_default();
        let state = Box::new(AppState {
            hwnd: null_mut(),
            prompt: null_mut(),
            edit: null_mut(),
            hint: null_mut(),
            list: null_mut(),
            status: null_mut(),
            tooltip: null_mut(),
            settings: SettingsControls::default(),
            launcher: LauncherState::default(),
            pending_hotkey: config.hotkey.clone(),
            config,
            config_path,
            host_pipe,
            diagnostics: Vec::new(),
            hidden_since: Some(Instant::now()),
            launcher_had_focus: false,
            dpi: 96,
            theme: Theme::load(),
            launcher_font: null_mut(),
            tooltip_text: [0; 2048],
        });
        let state_pointer = Box::into_raw(state);
        let _ = APP.set(state_pointer as usize);
        if unsafe { create_windows(&mut *state_pointer) }.is_err() {
            unsafe { drop(Box::from_raw(state_pointer)) };
            return;
        }

        let worker_hwnd = unsafe { (*state_pointer).hwnd as isize };
        let _pipe_worker = thread::spawn(move || pipe_worker(worker_hwnd, pipe_name));

        let mut message: MSG = unsafe { zeroed() };
        while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
            if matches!(message.message, WM_KEYDOWN | WM_SYSKEYDOWN)
                && unsafe { handle_key(message.wParam as u16) }
            {
                continue;
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        unsafe {
            if !(*state_pointer).launcher_font.is_null() {
                DeleteObject((*state_pointer).launcher_font);
            }
            (*state_pointer).theme.destroy();
            drop(Box::from_raw(state_pointer));
        }
    }

    fn parse_args() -> Option<(String, String, PathBuf)> {
        let mut arguments = env::args().skip(1);
        let mut pipe = None;
        let mut host_pipe = None;
        let mut config = None;
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--pipe" => pipe = arguments.next(),
                "--host-pipe" => host_pipe = arguments.next(),
                "--config" => config = arguments.next().map(PathBuf::from),
                _ => {}
            }
        }
        Some((pipe?, host_pipe?, config?))
    }

    fn load_config(path: &PathBuf) -> Option<Config> {
        let config: Config = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        config.validate().ok()?;
        Some(config)
    }

    unsafe fn create_windows(state: &mut AppState) -> Result<(), ()> {
        let instance = GetModuleHandleW(null());
        let launcher_class = wide("RecentryLauncherWindow");
        let mut class: WNDCLASSW = zeroed();
        class.style = CS_HREDRAW | CS_VREDRAW | CS_DROPSHADOW;
        class.lpfnWndProc = Some(launcher_proc);
        class.hInstance = instance;
        class.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        class.lpszClassName = launcher_class.as_ptr();
        RegisterClassW(&class);
        state.hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            launcher_class.as_ptr(),
            wide("Recentry").as_ptr(),
            WS_POPUP | WS_BORDER,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            LAUNCHER_WIDTH,
            launcher_height(),
            null_mut(),
            null_mut(),
            instance,
            null::<CREATESTRUCTW>().cast(),
        );
        if state.hwnd.is_null() {
            return Err(());
        }
        state.dpi = GetDpiForWindow(state.hwnd).max(96);
        create_launcher_controls(state, instance);
        create_settings_window(state, instance)?;
        SetTimer(state.hwnd, IDLE_TIMER, 60_000, None);
        Ok(())
    }

    unsafe fn create_launcher_controls(state: &mut AppState, instance: HINSTANCE) {
        state.prompt = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("VS Code ›").as_ptr(),
            WS_CHILD | WS_VISIBLE | STATIC_NOPREFIX | STATIC_CENTER_IMAGE,
            0,
            0,
            0,
            0,
            state.hwnd,
            null_mut(),
            instance,
            null(),
        );
        state.edit = CreateWindowExW(
            0,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            0,
            0,
            0,
            0,
            state.hwnd,
            EDIT_ID as HMENU,
            instance,
            null(),
        );
        state.hint = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | STATIC_NOPREFIX | STATIC_CENTER_IMAGE | STATIC_NOTIFY,
            0,
            0,
            0,
            0,
            state.hwnd,
            HINT_ID as HMENU,
            instance,
            null(),
        );
        state.list = CreateWindowExW(
            0,
            wide("LISTBOX").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_VSCROLL
                | LBS_NOTIFY as u32
                | LBS_OWNERDRAWFIXED as u32
                | LBS_HASSTRINGS as u32,
            0,
            0,
            0,
            0,
            state.hwnd,
            LIST_ID as HMENU,
            instance,
            null(),
        );
        state.status = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | STATIC_RIGHT | STATIC_NOPREFIX | STATIC_CENTER_IMAGE,
            0,
            0,
            0,
            0,
            state.hwnd,
            STATUS_ID as HMENU,
            instance,
            null(),
        );
        rebuild_launcher_font(state);
        SendMessageW(
            state.edit,
            EM_SETCUEBANNER,
            1,
            wide(text(state, "输入筛选", "Type to filter")).as_ptr() as LPARAM,
        );
        SendMessageW(
            state.edit,
            EM_SETMARGINS,
            (EC_LEFTMARGIN | EC_RIGHTMARGIN) as WPARAM,
            0,
        );
        SendMessageW(
            state.list,
            LB_SETITEMHEIGHT,
            0,
            scale(LAUNCHER_ROW_HEIGHT, state.dpi) as LPARAM,
        );
        state.tooltip = CreateWindowExW(
            WS_EX_TOPMOST,
            TOOLTIPS_CLASSW,
            null(),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            state.hwnd,
            null_mut(),
            instance,
            null(),
        );
        let mut tool: TTTOOLINFOW = zeroed();
        tool.cbSize = size_of::<TTTOOLINFOW>() as u32;
        tool.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
        tool.hwnd = state.hwnd;
        tool.uId = state.list as usize;
        tool.lpszText = -1isize as *mut u16;
        SendMessageW(
            state.tooltip,
            TTM_ADDTOOLW,
            0,
            &tool as *const TTTOOLINFOW as LPARAM,
        );
        SendMessageW(
            state.tooltip,
            TTM_SETMAXTIPWIDTH,
            0,
            scale(720, state.dpi) as LPARAM,
        );
        apply_control_theme(state);
    }

    unsafe fn create_settings_window(state: &mut AppState, instance: HINSTANCE) -> Result<(), ()> {
        let class_name = wide("RecentrySettingsWindow");
        let mut class: WNDCLASSW = zeroed();
        class.style = CS_HREDRAW | CS_VREDRAW;
        class.lpfnWndProc = Some(settings_proc);
        class.hInstance = instance;
        class.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        class.lpszClassName = class_name.as_ptr();
        RegisterClassW(&class);
        let width = scale(560, state.dpi);
        let height = scale(330, state.dpi);
        state.settings.hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            wide("Recentry Settings").as_ptr(),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            null_mut(),
            null_mut(),
            instance,
            null::<CREATESTRUCTW>().cast(),
        );
        if state.settings.hwnd.is_null() {
            return Err(());
        }
        let label = |text_value: &str, y: i32| {
            CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(text_value).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                scale(20, state.dpi),
                scale(y, state.dpi),
                scale(145, state.dpi),
                scale(24, state.dpi),
                state.settings.hwnd,
                null_mut(),
                instance,
                null(),
            )
        };
        state.settings.language_label = label("Language", 22);
        state.settings.hotkey_label = label("Hotkey", 72);
        state.settings.vscode_label = label("VS Code path", 172);
        state.settings.language = CreateWindowExW(
            0,
            wide("COMBOBOX").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | CBS_DROPDOWNLIST as u32,
            scale(175, state.dpi),
            scale(18, state.dpi),
            scale(350, state.dpi),
            scale(160, state.dpi),
            state.settings.hwnd,
            LANGUAGE_ID as HMENU,
            instance,
            null(),
        );
        state.settings.hotkey = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_READONLY as u32,
            scale(175, state.dpi),
            scale(68, state.dpi),
            scale(350, state.dpi),
            scale(28, state.dpi),
            state.settings.hwnd,
            HOTKEY_ID as HMENU,
            instance,
            null(),
        );
        state.settings.autostart = CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide("Start Recentry when I sign in").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX as u32,
            scale(175, state.dpi),
            scale(118, state.dpi),
            scale(350, state.dpi),
            scale(28, state.dpi),
            state.settings.hwnd,
            AUTOSTART_ID as HMENU,
            instance,
            null(),
        );
        state.settings.vscode_path = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            scale(175, state.dpi),
            scale(168, state.dpi),
            scale(350, state.dpi),
            scale(28, state.dpi),
            state.settings.hwnd,
            VSCODE_PATH_ID as HMENU,
            instance,
            null(),
        );
        state.settings.save = create_button(state, instance, SAVE_ID, "Save", 335);
        state.settings.cancel = create_button(state, instance, CANCEL_ID, "Cancel", 435);
        let font = GetStockObject(DEFAULT_GUI_FONT);
        for control in [
            state.settings.language_label,
            state.settings.language,
            state.settings.hotkey_label,
            state.settings.hotkey,
            state.settings.autostart,
            state.settings.vscode_label,
            state.settings.vscode_path,
            state.settings.save,
            state.settings.cancel,
        ] {
            SendMessageW(control, WM_SETFONT, font as WPARAM, 1);
        }
        layout_settings(state);
        apply_control_theme(state);
        Ok(())
    }

    unsafe fn create_button(
        state: &AppState,
        instance: HINSTANCE,
        id: usize,
        label: &str,
        x: i32,
    ) -> HWND {
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            scale(x, state.dpi),
            scale(235, state.dpi),
            scale(90, state.dpi),
            scale(30, state.dpi),
            state.settings.hwnd,
            id as HMENU,
            instance,
            null(),
        )
    }

    unsafe fn apply_control_theme(state: &AppState) {
        let theme = if state.theme.dark {
            wide("DarkMode_Explorer")
        } else {
            wide("Explorer")
        };
        for control in [
            state.edit,
            state.list,
            state.settings.language,
            state.settings.hotkey,
            state.settings.autostart,
            state.settings.vscode_path,
            state.settings.save,
            state.settings.cancel,
        ] {
            if !control.is_null() {
                SetWindowTheme(control, theme.as_ptr(), null());
            }
        }
    }

    unsafe fn rebuild_launcher_font(state: &mut AppState) {
        let previous_font = state.launcher_font;
        let next_font = CreateFontW(
            -scale(14, state.dpi),
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            u32::from(FIXED_PITCH | FF_MODERN),
            wide("Consolas").as_ptr(),
        );
        state.launcher_font = next_font;
        let font = if next_font.is_null() {
            GetStockObject(DEFAULT_GUI_FONT)
        } else {
            next_font
        };
        for control in [
            state.prompt,
            state.edit,
            state.hint,
            state.list,
            state.status,
        ] {
            SendMessageW(control, WM_SETFONT, font as WPARAM, 1);
        }
        if !previous_font.is_null() {
            DeleteObject(previous_font);
        }
    }

    unsafe fn discover_projects() {
        let state = app();
        state.config = load_config(&state.config_path).unwrap_or_else(|| state.config.clone());
        let environment = DiscoveryEnvironment::current();
        let override_path = state
            .config
            .vscode_path_override
            .as_deref()
            .map(PathBuf::from);
        let provider = VsCodeRecentProvider {
            environment,
            override_path,
        };
        let report = provider.discover();
        state.diagnostics = report
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{:?} {}: {}",
                    diagnostic.level, diagnostic.code, diagnostic.message
                )
            })
            .collect();
        state.launcher.set_projects(report.value);
        SetWindowTextW(state.edit, wide("").as_ptr());
        state.launcher.reset_query();
        refresh_list();
    }

    unsafe fn refresh_list() {
        let state = app();
        let query = window_text(state.edit).to_lowercase();
        SetWindowTextW(
            state.hint,
            wide(text(state, "输入筛选", "Type to filter")).as_ptr(),
        );
        ShowWindow(
            state.hint,
            if query.is_empty() {
                SW_SHOWNORMAL
            } else {
                SW_HIDE
            },
        );
        state.launcher.set_query(query);
        SendMessageW(state.list, LB_RESETCONTENT, 0, 0);
        for project in state.launcher.visible() {
            let row = wide(&format!("[VSCode] {}  {}", project.name, project.detail));
            SendMessageW(state.list, LB_ADDSTRING, 0, row.as_ptr() as LPARAM);
        }
        if state.launcher.visible().is_empty() {
            let placeholder = wide(&empty_message(state));
            SendMessageW(state.list, LB_ADDSTRING, 0, placeholder.as_ptr() as LPARAM);
        }
        if let Some(selected) = state.launcher.selected_index() {
            SendMessageW(state.list, LB_SETCURSEL, selected, 0);
        }
        update_counter(state);
        InvalidateRect(state.list, null(), 0);
        UpdateWindow(state.list);
    }

    fn empty_message(state: &AppState) -> String {
        state
            .diagnostics
            .iter()
            .find(|entry| entry.starts_with("Error"))
            .cloned()
            .unwrap_or_else(|| text(state, "未找到最近项目", "No recent projects found").to_owned())
    }

    unsafe fn update_counter(state: &AppState) {
        let total = state.launcher.visible().len();
        let current = state
            .launcher
            .selected_index()
            .map_or(0, |selected| selected + 1);
        SetWindowTextW(state.status, wide(&format!("{current}/{total}")).as_ptr());
    }

    unsafe fn show_launcher() {
        discover_projects();
        let state = app();
        state.hidden_since = None;
        state.launcher_had_focus = false;
        state.dpi = active_dpi();
        rebuild_launcher_font(state);
        SendMessageW(
            state.edit,
            EM_SETCUEBANNER,
            1,
            wide(text(state, "输入筛选", "Type to filter")).as_ptr() as LPARAM,
        );
        SendMessageW(
            state.list,
            LB_SETITEMHEIGHT,
            0,
            scale(LAUNCHER_ROW_HEIGHT, state.dpi) as LPARAM,
        );
        center_window(
            state.hwnd,
            scale(LAUNCHER_WIDTH, state.dpi),
            scale(launcher_height(), state.dpi),
        );
        ShowWindow(state.settings.hwnd, SW_HIDE);
        ShowWindow(state.hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(state.hwnd);
        SetFocus(state.edit);
        SendMessageW(state.edit, EM_SETSEL, 0, -1);
        sync_selection(state);
        UpdateWindow(state.hwnd);
        state.launcher_had_focus = launcher_owns_foreground(state);
        SetTimer(state.hwnd, FOCUS_TIMER, 125, None);
    }

    unsafe fn hide_launcher() {
        let state = app();
        KillTimer(state.hwnd, FOCUS_TIMER);
        ShowWindow(state.hwnd, SW_HIDE);
        state.launcher_had_focus = false;
        mark_hidden_if_needed(state);
    }

    unsafe fn open_selected() {
        let state = app();
        let Some(project) = state.launcher.selected().cloned() else {
            return;
        };
        let environment = DiscoveryEnvironment::current();
        let override_path = state
            .config
            .vscode_path_override
            .as_deref()
            .map(PathBuf::from);
        let result =
            discover_vscode(&environment, override_path.as_deref()).and_then(|installation| {
                let opener = VsCodeOpener {
                    executable: installation.code_exe.clone(),
                    window_state_files: window_state_candidates(&installation, &environment),
                };
                opener
                    .open_or_focus(&project)
                    .map_err(|error| recentry_core::VsCodeError::InvalidProduct(error.to_string()))
            });
        match result {
            Ok(_) => hide_launcher(),
            Err(error) => show_error(state.hwnd, &error.to_string()),
        }
    }

    unsafe fn show_settings(config: Config) {
        let state = app();
        state.dpi = active_dpi();
        state.config = config.clone();
        state.pending_hotkey = config.hotkey.clone();
        state.hidden_since = None;
        SetWindowTextW(
            state.settings.hwnd,
            wide(text(state, "Recentry 设置", "Recentry Settings")).as_ptr(),
        );
        SetWindowTextW(
            state.settings.language_label,
            wide(text(state, "语言", "Language")).as_ptr(),
        );
        SetWindowTextW(
            state.settings.hotkey_label,
            wide(text(state, "快捷键", "Hotkey")).as_ptr(),
        );
        SetWindowTextW(
            state.settings.vscode_label,
            wide(text(state, "VS Code 路径", "VS Code path")).as_ptr(),
        );
        SetWindowTextW(
            state.settings.autostart,
            wide(text(
                state,
                "登录时启动 Recentry",
                "Start Recentry when I sign in",
            ))
            .as_ptr(),
        );
        SetWindowTextW(
            state.settings.save,
            wide(text(state, "保存", "Save")).as_ptr(),
        );
        SetWindowTextW(
            state.settings.cancel,
            wide(text(state, "取消", "Cancel")).as_ptr(),
        );
        SendMessageW(state.settings.language, CB_RESETCONTENT, 0, 0);
        for item in ["跟随系统 / System", "简体中文", "English"] {
            SendMessageW(
                state.settings.language,
                CB_ADDSTRING,
                0,
                wide(item).as_ptr() as LPARAM,
            );
        }
        let language = match config.language {
            Language::System => 0,
            Language::ZhCn => 1,
            Language::En => 2,
        };
        SendMessageW(state.settings.language, CB_SETCURSEL, language, 0);
        SetWindowTextW(
            state.settings.hotkey,
            wide(&config.hotkey.display()).as_ptr(),
        );
        SendMessageW(
            state.settings.autostart,
            BM_SETCHECK,
            if config.autostart {
                BST_CHECKED as usize
            } else {
                0
            },
            0,
        );
        SetWindowTextW(
            state.settings.vscode_path,
            wide(config.vscode_path_override.as_deref().unwrap_or("")).as_ptr(),
        );
        center_window(
            state.settings.hwnd,
            scale(560, state.dpi),
            scale(330, state.dpi),
        );
        layout_settings(state);
        KillTimer(state.hwnd, FOCUS_TIMER);
        ShowWindow(state.hwnd, SW_HIDE);
        ShowWindow(state.settings.hwnd, SW_SHOWNORMAL);
        state.hidden_since = None;
        SetForegroundWindow(state.settings.hwnd);
        UpdateWindow(state.settings.hwnd);
    }

    unsafe fn save_settings() {
        let state = app();
        let language = match SendMessageW(state.settings.language, CB_GETCURSEL, 0, 0) {
            1 => Language::ZhCn,
            2 => Language::En,
            _ => Language::System,
        };
        let vscode_path = window_text(state.settings.vscode_path);
        let config = Config {
            language,
            hotkey: state.pending_hotkey.clone(),
            autostart: SendMessageW(state.settings.autostart, BM_GETCHECK, 0, 0)
                == BST_CHECKED as isize,
            vscode_path_override: (!vscode_path.trim().is_empty())
                .then(|| vscode_path.trim().to_owned()),
            first_run_completed: true,
            ..state.config.clone()
        };
        if let Err(error) = config.validate() {
            show_error(state.settings.hwnd, &error);
            return;
        }
        match request::<_, HostResponse>(
            &state.host_pipe,
            &HostCommand::SaveConfig(config.clone()),
            5_000,
        ) {
            Ok(HostResponse::Saved) => {
                state.config = config;
                ShowWindow(state.settings.hwnd, SW_HIDE);
                mark_hidden_if_needed(state);
            }
            Ok(HostResponse::Error(error)) => show_error(state.settings.hwnd, &error),
            Ok(response) => show_error(
                state.settings.hwnd,
                &format!("unexpected host response: {response:?}"),
            ),
            Err(error) => show_error(state.settings.hwnd, &error.to_string()),
        }
    }

    unsafe fn handle_key(key: u16) -> bool {
        let state = app();
        if IsWindowVisible(state.settings.hwnd) != 0 {
            if key == VK_ESCAPE {
                ShowWindow(state.settings.hwnd, SW_HIDE);
                mark_hidden_if_needed(state);
                return true;
            }
            if GetFocus() == state.settings.hotkey {
                return capture_hotkey(key);
            }
            return false;
        }
        if IsWindowVisible(state.hwnd) == 0 {
            return false;
        }
        match key {
            VK_ESCAPE => hide_launcher(),
            VK_DOWN => {
                state.launcher.move_selection(1);
                sync_selection(state);
            }
            VK_UP => {
                state.launcher.move_selection(-1);
                sync_selection(state);
            }
            VK_RETURN => open_selected(),
            _ => return false,
        }
        true
    }

    unsafe fn capture_hotkey(key: u16) -> bool {
        if matches!(key, VK_CONTROL | VK_MENU | VK_SHIFT | VK_LWIN | VK_RWIN) {
            return true;
        }
        let key_name = if (u16::from(b'A')..=u16::from(b'Z')).contains(&key)
            || (u16::from(b'0')..=u16::from(b'9')).contains(&key)
        {
            char::from_u32(u32::from(key)).map(|value| value.to_string())
        } else if (VK_F1..=VK_F24).contains(&key) {
            Some(format!("F{}", key - VK_F1 + 1))
        } else {
            None
        };
        let Some(key_name) = key_name else {
            return true;
        };
        let hotkey = Hotkey {
            ctrl: GetKeyState(VK_CONTROL as i32) < 0,
            alt: GetKeyState(VK_MENU as i32) < 0,
            shift: GetKeyState(VK_SHIFT as i32) < 0,
            win: GetKeyState(VK_LWIN as i32) < 0 || GetKeyState(VK_RWIN as i32) < 0,
            key: key_name,
        };
        let mut config = app().config.clone();
        config.hotkey = hotkey.clone();
        if config.validate().is_ok() {
            app().pending_hotkey = hotkey.clone();
            SetWindowTextW(app().settings.hotkey, wide(&hotkey.display()).as_ptr());
        }
        true
    }

    unsafe fn sync_selection(state: &AppState) {
        if let Some(index) = state.launcher.selected_index() {
            SendMessageW(state.list, LB_SETCURSEL, index, 0);
        }
        update_counter(state);
        InvalidateRect(state.list, null(), 0);
        UpdateWindow(state.list);
    }

    unsafe fn handle_pipe_command(pointer: LPARAM) {
        let pending = &mut *(pointer as *mut PendingCommand);
        match &pending.command {
            UiCommand::Show => {
                show_launcher();
                pending.response = UiResponse::Ready;
            }
            UiCommand::Hide => {
                hide_launcher();
                pending.response = UiResponse::Hidden;
            }
            UiCommand::Settings(config) => {
                show_settings(config.clone());
                pending.response = UiResponse::Shown;
            }
            UiCommand::Diagnostics(host_diagnostics) => {
                let state = app();
                let mut details = host_diagnostics.clone();
                if !state.diagnostics.is_empty() {
                    details.push_str("\n\nVS Code provider:\n");
                    details.push_str(&state.diagnostics.join("\n"));
                }
                pending.response = UiResponse::Shown;
                SetEvent(pending.done);
                MessageBoxW(
                    state.hwnd,
                    wide(&details).as_ptr(),
                    wide("Recentry Diagnostics").as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
                return;
            }
            UiCommand::Quit => {
                pending.response = UiResponse::Quitting;
            }
        }
        SetEvent(pending.done);
    }

    fn pipe_worker(hwnd: isize, pipe_name: String) {
        let Ok(pipe) = connect(&pipe_name, 5_000) else {
            return;
        };
        while let Ok(command) = pipe.receive::<UiCommand>() {
            let done = unsafe { CreateEventW(null(), 0, 0, null()) };
            if done.is_null() {
                break;
            }
            let quitting = matches!(command, UiCommand::Quit);
            let mut pending = Box::new(PendingCommand {
                command,
                response: UiResponse::Error("command was not handled".to_owned()),
                done,
            });
            let pointer = (&mut *pending) as *mut PendingCommand as LPARAM;
            if unsafe { PostMessageW(hwnd as HWND, WM_PIPE_COMMAND, 0, pointer) } == 0 {
                unsafe { CloseHandle(done) };
                break;
            }
            unsafe { WaitForSingleObject(done, INFINITE) };
            let sent = pipe.send(&pending.response).is_ok();
            unsafe { CloseHandle(done) };
            if quitting {
                unsafe { PostMessageW(hwnd as HWND, WM_CLOSE, 0, 0) };
            }
            if !sent || quitting {
                break;
            }
        }
    }

    unsafe extern "system" fn launcher_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_SIZE => {
                layout_launcher(
                    (lparam as u32 & 0xffff) as i32,
                    (lparam as u32 >> 16) as i32,
                );
                0
            }
            WM_COMMAND => {
                let control = wparam & 0xffff;
                let notification = ((wparam >> 16) & 0xffff) as u32;
                if control == EDIT_ID && notification == EN_CHANGE {
                    refresh_list();
                } else if control == HINT_ID {
                    SetFocus(app().edit);
                } else if control == LIST_ID && notification == LBN_SELCHANGE {
                    let selected = SendMessageW(app().list, LB_GETCURSEL, 0, 0);
                    if selected >= 0 {
                        app().launcher.select_index(selected as usize);
                        update_counter(app());
                        InvalidateRect(app().list, null(), 0);
                        UpdateWindow(app().list);
                    }
                } else if control == LIST_ID && notification == LBN_DBLCLK {
                    let selected = SendMessageW(app().list, LB_GETCURSEL, 0, 0);
                    if selected >= 0 {
                        app().launcher.select_index(selected as usize);
                    }
                    open_selected();
                }
                0
            }
            WM_DRAWITEM if wparam == LIST_ID => {
                draw_row(&*(lparam as *const DRAWITEMSTRUCT));
                1
            }
            WM_NOTIFY => {
                let info = lparam as *mut NMTTDISPINFOW;
                if !info.is_null() && (*info).hdr.code == TTN_GETDISPINFOW {
                    update_tooltip(info);
                    return 0;
                }
                DefWindowProcW(hwnd, message, wparam, lparam)
            }
            WM_ACTIVATE => {
                let inactive = (wparam as u32 & 0xffff) == WA_INACTIVE;
                if inactive {
                    if app().launcher_had_focus && IsWindowVisible(app().settings.hwnd) == 0 {
                        hide_launcher();
                    }
                } else {
                    app().launcher_had_focus = true;
                }
                0
            }
            WM_TIMER if wparam == IDLE_TIMER => {
                if app()
                    .hidden_since
                    .is_some_and(|hidden| hidden.elapsed() >= IDLE_EXIT_AFTER)
                {
                    DestroyWindow(hwnd);
                }
                0
            }
            WM_TIMER if wparam == FOCUS_TIMER => {
                let state = app();
                if IsWindowVisible(state.hwnd) != 0 {
                    if launcher_owns_foreground(state) {
                        state.launcher_had_focus = true;
                    } else if state.launcher_had_focus {
                        hide_launcher();
                    }
                }
                0
            }
            WM_PIPE_COMMAND => {
                handle_pipe_command(lparam);
                0
            }
            WM_SETTINGCHANGE | WM_THEMECHANGED => {
                refresh_theme();
                0
            }
            WM_DPICHANGED => {
                let suggested = &*(lparam as *const RECT);
                app().dpi = (wparam as u32 & 0xffff).max(96);
                rebuild_launcher_font(app());
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOACTIVATE,
                );
                SendMessageW(
                    app().list,
                    LB_SETITEMHEIGHT,
                    0,
                    scale(LAUNCHER_ROW_HEIGHT, app().dpi) as LPARAM,
                );
                0
            }
            WM_ERASEBKGND => paint_background(hwnd, wparam as HDC),
            WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
                control_color(wparam as HDC, lparam as HWND)
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                if !app().settings.hwnd.is_null() {
                    DestroyWindow(app().settings.hwnd);
                    app().settings.hwnd = null_mut();
                }
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe extern "system" fn settings_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_COMMAND => {
                match wparam & 0xffff {
                    SAVE_ID => save_settings(),
                    CANCEL_ID => {
                        ShowWindow(hwnd, SW_HIDE);
                        mark_hidden_if_needed(app());
                    }
                    _ => {}
                }
                0
            }
            WM_ERASEBKGND => paint_background(hwnd, wparam as HDC),
            WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                control_color(wparam as HDC, lparam as HWND)
            }
            WM_CLOSE => {
                ShowWindow(hwnd, SW_HIDE);
                mark_hidden_if_needed(app());
                0
            }
            WM_DPICHANGED => {
                let suggested = &*(lparam as *const RECT);
                app().dpi = (wparam as u32 & 0xffff).max(96);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOACTIVATE,
                );
                layout_settings(app());
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn layout_launcher(width: i32, height: i32) {
        let state = app();
        if state.edit.is_null() {
            return;
        }
        let horizontal_padding = scale(10, state.dpi);
        let header_height = scale(LAUNCHER_HEADER_HEIGHT, state.dpi);
        let prompt_width = scale(84, state.dpi);
        let counter_width = scale(58, state.dpi);
        MoveWindow(
            state.prompt,
            horizontal_padding,
            0,
            prompt_width,
            header_height,
            1,
        );
        MoveWindow(
            state.edit,
            horizontal_padding + prompt_width,
            scale(4, state.dpi),
            width - 2 * horizontal_padding - prompt_width - counter_width,
            header_height - scale(8, state.dpi),
            1,
        );
        MoveWindow(
            state.hint,
            horizontal_padding + prompt_width,
            scale(4, state.dpi),
            width - 2 * horizontal_padding - prompt_width - counter_width,
            header_height - scale(8, state.dpi),
            1,
        );
        MoveWindow(
            state.status,
            width - horizontal_padding - counter_width,
            0,
            counter_width,
            header_height,
            1,
        );
        MoveWindow(
            state.list,
            0,
            header_height,
            width,
            height - header_height,
            1,
        );
    }

    unsafe fn layout_settings(state: &AppState) {
        if state.settings.hwnd.is_null() {
            return;
        }
        let dpi = state.dpi;
        for (control, x, y, width, height) in [
            (state.settings.language_label, 20, 22, 145, 24),
            (state.settings.language, 175, 18, 350, 160),
            (state.settings.hotkey_label, 20, 72, 145, 24),
            (state.settings.hotkey, 175, 68, 350, 28),
            (state.settings.autostart, 175, 118, 350, 28),
            (state.settings.vscode_label, 20, 172, 145, 24),
            (state.settings.vscode_path, 175, 168, 350, 28),
            (state.settings.save, 335, 235, 90, 30),
            (state.settings.cancel, 435, 235, 90, 30),
        ] {
            MoveWindow(
                control,
                scale(x, dpi),
                scale(y, dpi),
                scale(width, dpi),
                scale(height, dpi),
                1,
            );
        }
    }

    unsafe fn draw_row(draw: &DRAWITEMSTRUCT) {
        if draw.itemID == u32::MAX {
            return;
        }
        let state = app();
        let project = state.launcher.visible().get(draw.itemID as usize);
        let current_selection = SendMessageW(state.list, LB_GETCURSEL, 0, 0);
        let selected = current_selection >= 0 && current_selection as u32 == draw.itemID;
        let background = if selected {
            state.theme.selected
        } else {
            state.theme.surface
        };
        let brush = windows_sys::Win32::Graphics::Gdi::CreateSolidBrush(background);
        FillRect(draw.hDC, &draw.rcItem, brush);
        DeleteObject(brush);
        SetBkMode(draw.hDC, TRANSPARENT as i32);
        let font = if state.launcher_font.is_null() {
            GetStockObject(DEFAULT_GUI_FONT)
        } else {
            state.launcher_font
        };
        SelectObject(draw.hDC, font);
        let text_color = if selected {
            state.theme.selected_text
        } else {
            state.theme.text
        };
        let muted = if selected {
            text_color
        } else {
            state.theme.muted
        };
        let tag = if selected {
            text_color
        } else {
            state.theme.accent
        };
        let padding = scale(10, state.dpi);
        let tag_width = scale(78, state.dpi);
        let name_width = scale(210, state.dpi);
        let flags = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX;
        let Some(project) = project else {
            let mut message_rect = RECT {
                left: draw.rcItem.left + padding,
                top: draw.rcItem.top,
                right: draw.rcItem.right - padding,
                bottom: draw.rcItem.bottom,
            };
            SetTextColor(draw.hDC, state.theme.muted);
            draw_text(draw.hDC, &empty_message(state), &mut message_rect, flags);
            return;
        };
        let mut tag_rect = RECT {
            left: draw.rcItem.left + padding,
            top: draw.rcItem.top,
            right: draw.rcItem.left + padding + tag_width,
            bottom: draw.rcItem.bottom,
        };
        let mut name_rect = RECT {
            left: tag_rect.right,
            top: draw.rcItem.top,
            right: tag_rect.right + name_width,
            bottom: draw.rcItem.bottom,
        };
        let mut path_rect = RECT {
            left: name_rect.right + scale(8, state.dpi),
            top: draw.rcItem.top,
            right: draw.rcItem.right - padding,
            bottom: draw.rcItem.bottom,
        };
        SetTextColor(draw.hDC, tag);
        draw_text(draw.hDC, "[VSCode]", &mut tag_rect, flags);
        SetTextColor(draw.hDC, text_color);
        draw_text(draw.hDC, &project.name, &mut name_rect, flags);
        SetTextColor(draw.hDC, muted);
        draw_text(draw.hDC, &project.detail, &mut path_rect, flags);
    }

    unsafe fn draw_text(hdc: HDC, value: &str, rect: &mut RECT, flags: u32) {
        let mut value = wide(value);
        DrawTextW(hdc, value.as_mut_ptr(), -1, rect, flags);
    }

    unsafe fn update_tooltip(info: *mut NMTTDISPINFOW) {
        let state = app();
        let mut point = POINT::default();
        GetCursorPos(&mut point);
        ScreenToClient(state.list, &mut point);
        let packed = ((point.y as u16 as u32) << 16) | point.x as u16 as u32;
        let hit = SendMessageW(state.list, LB_ITEMFROMPOINT, 0, packed as LPARAM) as u32;
        let row = (hit & 0xffff) as usize;
        let value = if hit >> 16 == 0 {
            state
                .launcher
                .visible()
                .get(row)
                .map(|project| project.detail.as_str())
                .unwrap_or("")
        } else {
            ""
        };
        state.tooltip_text.fill(0);
        for (slot, unit) in state
            .tooltip_text
            .iter_mut()
            .zip(value.encode_utf16().chain(Some(0)))
        {
            *slot = unit;
        }
        (*info).lpszText = state.tooltip_text.as_mut_ptr();
    }

    unsafe fn refresh_theme() {
        let state = app();
        state.theme.destroy();
        state.theme = Theme::load();
        apply_control_theme(state);
        InvalidateRect(state.hwnd, null(), 1);
        InvalidateRect(state.settings.hwnd, null(), 1);
    }

    unsafe fn paint_background(hwnd: HWND, hdc: HDC) -> LRESULT {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect);
        FillRect(hdc, &rect, app().theme.background_brush);
        1
    }

    unsafe fn control_color(hdc: HDC, control: HWND) -> LRESULT {
        let theme = &app().theme;
        SetTextColor(
            hdc,
            if control == app().hint {
                theme.muted
            } else {
                theme.text
            },
        );
        SetBkColor(hdc, theme.surface);
        theme.surface_brush as LRESULT
    }

    unsafe fn center_window(hwnd: HWND, width: i32, height: i32) {
        let foreground = GetForegroundWindow();
        let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(monitor, &mut info);
        let x = info.rcWork.left + (info.rcWork.right - info.rcWork.left - width) / 2;
        let y = info.rcWork.top + (info.rcWork.bottom - info.rcWork.top - height) / 2;
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_SHOWWINDOW);
    }

    const fn launcher_height() -> i32 {
        LAUNCHER_HEADER_HEIGHT
            + LAUNCHER_ROW_HEIGHT * LAUNCHER_VISIBLE_ROWS
            + LAUNCHER_NONCLIENT_HEIGHT
    }

    unsafe fn launcher_owns_foreground(state: &AppState) -> bool {
        let foreground = GetForegroundWindow();
        if foreground.is_null() {
            return false;
        }
        let root = GetAncestor(foreground, GA_ROOT);
        root == state.hwnd || root == state.settings.hwnd
    }

    unsafe fn active_dpi() -> u32 {
        GetDpiForWindow(GetForegroundWindow()).max(96)
    }

    unsafe fn mark_hidden_if_needed(state: &mut AppState) {
        if IsWindowVisible(state.hwnd) == 0 && IsWindowVisible(state.settings.hwnd) == 0 {
            state.hidden_since = Some(Instant::now());
        }
    }

    unsafe fn window_text(hwnd: HWND) -> String {
        let length = GetWindowTextLengthW(hwnd);
        let mut buffer = vec![0u16; length.max(0) as usize + 1];
        GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }

    unsafe fn show_error(hwnd: HWND, message: &str) {
        MessageBoxW(
            hwnd,
            wide(message).as_ptr(),
            wide("Recentry").as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }

    fn text<'a>(state: &AppState, zh: &'a str, en: &'a str) -> &'a str {
        match state.config.language {
            Language::ZhCn => zh,
            Language::En => en,
            Language::System => {
                if unsafe { GetUserDefaultUILanguage() & 0x03ff == 0x0004 } {
                    zh
                } else {
                    en
                }
            }
        }
    }

    fn scale(value: i32, dpi: u32) -> i32 {
        ((i64::from(value) * i64::from(dpi) + 48) / 96) as i32
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    fn rgb(red: u8, green: u8, blue: u8) -> u32 {
        u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16)
    }

    impl Theme {
        fn load() -> Self {
            let dark = system_uses_dark_theme();
            let (background, surface, text, muted, accent, selected, selected_text) = if dark {
                (
                    rgb(24, 25, 27),
                    rgb(24, 25, 27),
                    rgb(235, 235, 235),
                    rgb(145, 149, 154),
                    rgb(104, 193, 211),
                    rgb(13, 92, 108),
                    rgb(255, 255, 255),
                )
            } else {
                (
                    rgb(247, 247, 244),
                    rgb(247, 247, 244),
                    rgb(32, 34, 37),
                    rgb(103, 107, 111),
                    rgb(31, 105, 121),
                    rgb(17, 78, 91),
                    rgb(255, 255, 255),
                )
            };
            Self {
                dark,
                surface,
                text,
                muted,
                accent,
                selected,
                selected_text,
                background_brush: unsafe {
                    windows_sys::Win32::Graphics::Gdi::CreateSolidBrush(background)
                },
                surface_brush: unsafe {
                    windows_sys::Win32::Graphics::Gdi::CreateSolidBrush(surface)
                },
            }
        }

        unsafe fn destroy(&mut self) {
            if !self.background_brush.is_null() {
                DeleteObject(self.background_brush);
                self.background_brush = null_mut();
            }
            if !self.surface_brush.is_null() {
                DeleteObject(self.surface_brush);
                self.surface_brush = null_mut();
            }
        }
    }

    fn system_uses_dark_theme() -> bool {
        let mut value = 1u32;
        let mut size = size_of::<u32>() as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize").as_ptr(),
                wide("AppsUseLightTheme").as_ptr(),
                RRF_RT_REG_DWORD,
                null_mut(),
                (&mut value as *mut u32).cast(),
                &mut size,
            )
        };
        result == 0 && value == 0
    }
}

#[cfg(windows)]
fn main() {
    windows_main::run();
}
