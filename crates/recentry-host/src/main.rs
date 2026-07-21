#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(windows, allow(unsafe_op_in_unsafe_fn))]

#[cfg(not(any(windows, unix)))]
fn main() {
    eprintln!("Recentry is unavailable on this platform.");
}

#[cfg(unix)]
mod unix_main;

#[cfg(unix)]
fn main() {
    unix_main::run();
}

#[cfg(windows)]
mod windows_main {
    use std::{
        env,
        ffi::c_void,
        mem::zeroed,
        path::{Path, PathBuf},
        ptr::{null, null_mut},
        sync::{Arc, Mutex, OnceLock},
        thread,
    };

    use recentry_host::{
        ConfigStore, HOTKEY_ID, HostAdapter, HostPlatform, HostRuntime, UiCoordinator,
        WM_CONFIG_CHANGED, WM_TRAY, WindowsHostPlatform,
    };
    use recentry_ipc::{LocalServer, current_user_endpoint, request};
    use recentry_protocol::{
        Config, HOST_ENDPOINT_ID, HostCommand, HostResponse, Hotkey, Language, UiCommand,
    };
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, WPARAM,
        },
        Globalization::GetUserDefaultUILanguage,
        System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
        UI::WindowsAndMessaging::{
            ASFW_ANY, AllowSetForegroundWindow, AppendMenuW, CREATESTRUCTW, CW_USEDEFAULT,
            CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
            DispatchMessageW, GetCursorPos, GetMessageW, HMENU, MB_DEFBUTTON1, MB_ICONQUESTION,
            MB_OK, MB_YESNO, MF_SEPARATOR, MF_STRING, MSG, MessageBoxW, PostMessageW,
            PostQuitMessage, RegisterClassW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
            TPM_RETURNCMD, TrackPopupMenu, TranslateMessage, WM_CLOSE, WM_DESTROY, WM_HOTKEY,
            WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
        },
    };

    const MUTEX_NAME: &str = r"Local\RecentryHost-v1";
    const MENU_OPEN: usize = 1001;
    const MENU_SETTINGS: usize = 1002;
    const MENU_DIAGNOSTICS: usize = 1003;
    const MENU_QUIT: usize = 1004;

    static CONTEXT: OnceLock<Arc<HostContext>> = OnceLock::new();
    static PLATFORM: OnceLock<Mutex<WindowsHostPlatform>> = OnceLock::new();

    struct HostContext {
        runtime: HostRuntime,
        ui: UiCoordinator,
        host_endpoint: String,
        hwnd: isize,
    }

    struct WindowsRuntimeAdapter {
        ui: UiCoordinator,
        hwnd: isize,
    }

    impl HostAdapter for WindowsRuntimeAdapter {
        fn request_ui(&self, command: UiCommand) -> Result<recentry_protocol::UiResponse, String> {
            self.ui.request(command)
        }

        fn set_autostart(&self, enabled: bool, executable: &Path) -> Result<(), String> {
            PLATFORM
                .get()
                .ok_or_else(|| "Windows platform is not initialized".to_owned())?
                .lock()
                .unwrap()
                .set_autostart(enabled, executable)
        }

        fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), String> {
            PLATFORM
                .get()
                .ok_or_else(|| "Windows platform is not initialized".to_owned())?
                .lock()
                .unwrap()
                .register_hotkey(hotkey)
        }

        fn configuration_changed(&self) {
            unsafe {
                PostMessageW(self.hwnd as HWND, WM_CONFIG_CHANGED, 0, 0);
            }
        }

        fn notify(&self, title: &str, message: &str, error: bool) {
            if let Some(platform) = PLATFORM.get() {
                platform.lock().unwrap().notify(title, message, error);
            }
        }
    }

    #[derive(Clone, Copy)]
    enum RequestedAction {
        Background,
        Show,
        Settings,
        Diagnostics,
        Quit,
        Invalid,
    }

    impl RequestedAction {
        fn opens_ui(self) -> bool {
            matches!(self, Self::Show | Self::Settings | Self::Diagnostics)
        }

        fn command(self) -> Option<HostCommand> {
            match self {
                Self::Background => None,
                Self::Show => Some(HostCommand::Show),
                Self::Settings => Some(HostCommand::Settings),
                Self::Diagnostics => Some(HostCommand::Diagnostics),
                Self::Quit => Some(HostCommand::Quit),
                Self::Invalid => None,
            }
        }
    }

    pub fn run() {
        let requested = parse_action();
        if matches!(requested, RequestedAction::Invalid) {
            return;
        }
        if requested.opens_ui() {
            unsafe { AllowSetForegroundWindow(ASFW_ANY) };
        }
        let host_endpoint = match current_user_endpoint(HOST_ENDPOINT_ID) {
            Ok(endpoint) => endpoint,
            Err(_) => return,
        };
        if let Some(command) = requested.command() {
            if forward_command(&host_endpoint, &command, 200) {
                return;
            }
        } else if request::<_, HostResponse>(&host_endpoint, &HostCommand::Ping, 100).is_ok() {
            return;
        }

        let mutex_name = wide(MUTEX_NAME);
        let mutex = unsafe { CreateMutexW(null(), 0, mutex_name.as_ptr()) };
        if mutex.is_null() {
            return;
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            if let Some(command) = requested.command() {
                let _ = forward_command(&host_endpoint, &command, 5_000);
            }
            unsafe { CloseHandle(mutex) };
            return;
        }
        if matches!(requested, RequestedAction::Quit) {
            unsafe { CloseHandle(mutex) };
            return;
        }

        let executable = match env::current_exe() {
            Ok(path) => path,
            Err(_) => {
                unsafe { CloseHandle(mutex) };
                return;
            }
        };
        let config_path = config_path();
        let store = ConfigStore::new(config_path.clone());
        let (mut config, is_new, startup_warning) = match store.load() {
            Ok(loaded) => (loaded.config, loaded.is_new, None),
            Err(error) => {
                let fallback = Config {
                    first_run_completed: true,
                    ..Default::default()
                };
                (fallback, false, Some(error.to_string()))
            }
        };

        let class_name = wide("RecentryHostWindow");
        let mut class: WNDCLASSW = unsafe { zeroed() };
        class.lpfnWndProc = Some(window_proc);
        class.hInstance = unsafe { GetModuleHandleW(null()) };
        class.lpszClassName = class_name.as_ptr();
        unsafe { RegisterClassW(&class) };
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide("Recentry Host").as_ptr(),
                0,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                0,
                0,
                null_mut(),
                null_mut(),
                class.hInstance,
                null::<CREATESTRUCTW>() as *const c_void,
            )
        };
        if hwnd.is_null() {
            unsafe { CloseHandle(mutex) };
            return;
        }

        let platform = match WindowsHostPlatform::install(hwnd) {
            Ok(platform) => platform,
            Err(_) => {
                unsafe {
                    DestroyWindow(hwnd);
                    CloseHandle(mutex);
                }
                return;
            }
        };
        let _ = PLATFORM.set(Mutex::new(platform));

        if is_new && !config.first_run_completed {
            let chinese = uses_chinese(&config);
            let (title, message) = if chinese {
                (
                    "Recentry 首次运行",
                    "是否允许 Recentry 开机启动？\n\n推荐开启，之后可在设置中修改。",
                )
            } else {
                (
                    "Recentry first run",
                    "Start Recentry when you sign in?\n\nRecommended; you can change this later in Settings.",
                )
            };
            let answer = unsafe {
                MessageBoxW(
                    hwnd,
                    wide(message).as_ptr(),
                    wide(title).as_ptr(),
                    MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON1,
                )
            };
            config.autostart = answer == 6;
            config.first_run_completed = true;
            let save_result = PLATFORM
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .set_autostart(config.autostart, &executable)
                .and_then(|_| store.save(&config).map_err(|error| error.to_string()));
            if let Err(error) = save_result {
                let _ = PLATFORM
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_autostart(false, &executable);
                config.autostart = false;
                PLATFORM.get().unwrap().lock().unwrap().notify(
                    "Recentry",
                    &format!("First-run settings could not be saved: {error}"),
                    true,
                );
            }
        }

        let (ui, ui_worker) = UiCoordinator::start(
            executable.with_file_name("recentry-ui.exe"),
            config_path,
            host_endpoint.clone(),
        );
        let adapter = Arc::new(WindowsRuntimeAdapter {
            ui: ui.clone(),
            hwnd: hwnd as isize,
        });
        let context = Arc::new(HostContext {
            runtime: HostRuntime::new(store, config, executable, adapter),
            ui,
            host_endpoint,
            hwnd: hwnd as isize,
        });
        let _ = CONTEXT.set(context.clone());
        context.runtime.apply_hotkey();

        if let Some(warning) = startup_warning {
            PLATFORM.get().unwrap().lock().unwrap().notify(
                "Recentry configuration",
                &warning,
                true,
            );
        }

        let server_context = context.clone();
        thread::spawn(move || control_server(server_context));

        match requested {
            RequestedAction::Show => spawn_ui(UiCommand::Show),
            RequestedAction::Settings => spawn_ui(UiCommand::Settings(context.runtime.config())),
            RequestedAction::Diagnostics => {
                spawn_ui(UiCommand::Diagnostics(context.runtime.diagnostics()))
            }
            RequestedAction::Quit | RequestedAction::Background | RequestedAction::Invalid => {}
        }

        let mut message: MSG = unsafe { zeroed() };
        while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        context.ui.shutdown();
        let _ = ui_worker.join();
        if let Some(platform) = PLATFORM.get() {
            platform.lock().unwrap().uninstall();
        }
        unsafe { CloseHandle(mutex) };
    }

    fn control_server(context: Arc<HostContext>) {
        let server = match LocalServer::bind(&context.host_endpoint) {
            Ok(server) => server,
            Err(error) => {
                PLATFORM.get().unwrap().lock().unwrap().notify(
                    "Recentry IPC",
                    &error.to_string(),
                    true,
                );
                return;
            }
        };
        loop {
            let connection = match server.accept() {
                Ok(connection) => connection,
                Err(_) => continue,
            };
            let command = match connection.receive::<HostCommand>() {
                Ok(command) => command,
                Err(error) => {
                    let _ = connection.send(&HostResponse::Error(error.to_string()));
                    continue;
                }
            };
            let quitting = matches!(command, HostCommand::Quit);
            let response = context.runtime.dispatch(command);
            let _ = connection.send(&response);
            if quitting {
                unsafe { PostMessageW(context.hwnd as HWND, WM_CLOSE, 0, 0) };
                break;
            }
        }
    }

    fn forward_command(endpoint: &str, command: &HostCommand, timeout_ms: u32) -> bool {
        match request::<_, HostResponse>(endpoint, command, timeout_ms) {
            Ok(HostResponse::Error(error)) => {
                unsafe {
                    MessageBoxW(
                        null_mut(),
                        wide(&error).as_ptr(),
                        wide("Recentry").as_ptr(),
                        MB_OK,
                    );
                }
                true
            }
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn spawn_ui(command: UiCommand) {
        let Some(context) = CONTEXT.get() else {
            return;
        };
        let context = context.clone();
        thread::spawn(move || {
            if let Err(error) = context.runtime.request_ui(command) {
                if let Some(platform) = PLATFORM.get() {
                    platform.lock().unwrap().notify("Recentry UI", &error, true);
                }
            }
        });
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_CONFIG_CHANGED => {
                if let Some(context) = CONTEXT.get() {
                    context.runtime.apply_hotkey();
                }
                0
            }
            WM_HOTKEY if wparam == HOTKEY_ID as usize => {
                spawn_ui(UiCommand::Show);
                0
            }
            WM_TRAY => {
                let event = (lparam as u32) & 0xffff;
                if event == WM_LBUTTONUP {
                    spawn_ui(UiCommand::Show);
                } else if event == WM_RBUTTONUP {
                    show_tray_menu(hwnd);
                }
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    unsafe fn show_tray_menu(hwnd: HWND) {
        let chinese = CONTEXT
            .get()
            .is_some_and(|context| uses_chinese(&context.runtime.config()));
        let labels = if chinese {
            ["打开 Recentry", "设置", "诊断", "退出"]
        } else {
            ["Open Recentry", "Settings", "Diagnostics", "Quit"]
        };
        let menu: HMENU = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, MENU_OPEN, wide(labels[0]).as_ptr());
        AppendMenuW(menu, MF_STRING, MENU_SETTINGS, wide(labels[1]).as_ptr());
        AppendMenuW(menu, MF_STRING, MENU_DIAGNOSTICS, wide(labels[2]).as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, MENU_QUIT, wide(labels[3]).as_ptr());
        let mut point = POINT::default();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            hwnd,
            null(),
        ) as usize;
        DestroyMenu(menu);
        match selected {
            MENU_OPEN => spawn_ui(UiCommand::Show),
            MENU_SETTINGS => {
                if let Some(context) = CONTEXT.get() {
                    spawn_ui(UiCommand::Settings(context.runtime.config()));
                }
            }
            MENU_DIAGNOSTICS => {
                if let Some(context) = CONTEXT.get() {
                    spawn_ui(UiCommand::Diagnostics(context.runtime.diagnostics()));
                }
            }
            MENU_QUIT => {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
            _ => {}
        }
    }

    fn parse_action() -> RequestedAction {
        match env::args().nth(1).as_deref() {
            Some("--background") => RequestedAction::Background,
            Some("show") => RequestedAction::Show,
            Some("settings") => RequestedAction::Settings,
            Some("diagnostics") => RequestedAction::Diagnostics,
            Some("quit") => RequestedAction::Quit,
            Some(_) => {
                unsafe {
                    MessageBoxW(
                        null_mut(),
                        wide("Usage: recentry.exe [show|settings|diagnostics|quit|--background]")
                            .as_ptr(),
                        wide("Recentry").as_ptr(),
                        MB_OK,
                    );
                }
                RequestedAction::Invalid
            }
            None => RequestedAction::Show,
        }
    }

    fn config_path() -> PathBuf {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir)
            .join("Recentry/config.json")
    }

    fn uses_chinese(config: &Config) -> bool {
        match config.language {
            Language::ZhCn => true,
            Language::En => false,
            Language::System => unsafe { GetUserDefaultUILanguage() & 0x03ff == 0x0004 },
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
}

#[cfg(windows)]
fn main() {
    windows_main::run();
}
