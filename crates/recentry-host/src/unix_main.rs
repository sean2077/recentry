use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
use recentry_host::set_xdg_autostart;
use recentry_host::{ConfigStore, HostAdapter, HostRuntime, UiCoordinator};
use recentry_ipc::{LocalServer, current_user_endpoint, request};
use recentry_protocol::{
    Config, HOST_ENDPOINT_ID, HostCommand, HostResponse, Hotkey, UiCommand, UiResponse,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestedAction {
    Background,
    Show,
    Settings,
    Diagnostics,
    Quit,
    Invalid,
}

impl RequestedAction {
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

struct UnixDevelopmentAdapter {
    ui: UiCoordinator,
}

impl HostAdapter for UnixDevelopmentAdapter {
    fn request_ui(&self, command: UiCommand) -> Result<UiResponse, String> {
        self.ui.request(command)
    }

    fn set_autostart(&self, enabled: bool, executable: &Path) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            return set_xdg_autostart(&xdg_config_home(), enabled, executable);
        }
        #[cfg(target_os = "macos")]
        {
            let _ = (enabled, executable);
            Err("SMAppService autostart is not implemented in this development build".to_owned())
        }
    }

    fn register_hotkey(&self, _hotkey: &Hotkey) -> Result<(), String> {
        Err("native Unix global shortcuts are not implemented in this development build".to_owned())
    }

    fn configuration_changed(&self) {}

    fn notify(&self, title: &str, message: &str, _error: bool) {
        eprintln!("{title}: {message}");
    }
}

pub fn run() {
    let action = parse_action();
    if action == RequestedAction::Invalid {
        return;
    }
    let endpoint = match current_user_endpoint(HOST_ENDPOINT_ID) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            eprintln!("Recentry IPC: {error}");
            return;
        }
    };

    if action != RequestedAction::Background {
        if let Some(command) = action.command() {
            if forward(&endpoint, &command, 200) {
                return;
            }
        }
        if action == RequestedAction::Quit {
            return;
        }
        if let Err(error) = start_background_host(&endpoint) {
            eprintln!("Recentry: {error}");
            return;
        }
        if let Some(command) = action.command() {
            if !forward(&endpoint, &command, 5_000) {
                eprintln!("Recentry: the background host did not accept the request");
            }
        }
        return;
    }

    if request::<_, HostResponse>(&endpoint, &HostCommand::Ping, 100).is_ok() {
        return;
    }
    run_background(endpoint);
}

fn run_background(endpoint: String) {
    let server = match LocalServer::bind(&endpoint) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("Recentry IPC: {error}");
            return;
        }
    };
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Recentry: cannot locate the host executable: {error}");
            return;
        }
    };
    let config_path = config_path();
    let store = ConfigStore::new(config_path.clone());
    let config = match store.load() {
        Ok(loaded) => loaded.config,
        Err(error) => {
            eprintln!("Recentry configuration: {error}");
            Config {
                first_run_completed: true,
                ..Config::default()
            }
        }
    };
    let (ui, worker) = UiCoordinator::start(
        executable.with_file_name("recentry-ui"),
        config_path,
        endpoint.clone(),
    );
    let adapter = Arc::new(UnixDevelopmentAdapter { ui: ui.clone() });
    let runtime = HostRuntime::new(store, config, executable, adapter);
    runtime.apply_hotkey();

    loop {
        let connection = match server.accept() {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!("Recentry IPC: {error}");
                continue;
            }
        };
        let command = match connection.receive::<HostCommand>() {
            Ok(command) => command,
            Err(error) => {
                let _ = connection.send(&HostResponse::Error(error.to_string()));
                continue;
            }
        };
        let quitting = matches!(command, HostCommand::Quit);
        let response = runtime.dispatch(command);
        let _ = connection.send(&response);
        if quitting {
            break;
        }
    }

    ui.shutdown();
    let _ = worker.join();
}

fn start_background_host(endpoint: &str) -> Result<(), String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    Command::new(executable)
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to start the background host: {error}"))?;

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if request::<_, HostResponse>(endpoint, &HostCommand::Ping, 100).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("the background host did not become ready within 5 seconds".to_owned())
}

fn forward(endpoint: &str, command: &HostCommand, timeout_ms: u32) -> bool {
    match request::<_, HostResponse>(endpoint, command, timeout_ms) {
        Ok(HostResponse::Error(error)) => {
            eprintln!("Recentry: {error}");
            true
        }
        Ok(_) => true,
        Err(_) => false,
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
            eprintln!("Usage: recentry [show|settings|diagnostics|quit|--background]");
            RequestedAction::Invalid
        }
        None => RequestedAction::Show,
    }
}

fn config_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir)
            .join("Library/Application Support/Recentry/config.json");
    }

    #[cfg(not(target_os = "macos"))]
    {
        xdg_config_home().join("recentry/config.json")
    }
}

#[cfg(target_os = "linux")]
fn xdg_config_home() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(env::temp_dir)
}
