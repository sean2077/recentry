use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use recentry_protocol::{Config, HostCommand, HostResponse, Hotkey, UiCommand, UiResponse};

use crate::ConfigStore;

pub trait HostAdapter: Send + Sync {
    fn request_ui(&self, command: UiCommand) -> Result<UiResponse, String>;
    fn set_autostart(&self, enabled: bool, executable: &Path) -> Result<(), String>;
    fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), String>;
    fn configuration_changed(&self);
    fn notify(&self, title: &str, message: &str, error: bool);
}

pub struct HostRuntime {
    store: ConfigStore,
    config: RwLock<Config>,
    executable: PathBuf,
    adapter: Arc<dyn HostAdapter>,
    hotkey_status: RwLock<String>,
}

impl HostRuntime {
    pub fn new(
        store: ConfigStore,
        config: Config,
        executable: PathBuf,
        adapter: Arc<dyn HostAdapter>,
    ) -> Self {
        Self {
            store,
            config: RwLock::new(config),
            executable,
            adapter,
            hotkey_status: RwLock::new(String::new()),
        }
    }

    pub fn dispatch(&self, command: HostCommand) -> HostResponse {
        match command {
            HostCommand::Ping => HostResponse::Pong,
            HostCommand::Show => self.ui_response(self.request_ui(UiCommand::Show)),
            HostCommand::Settings => self.ui_response(
                self.request_ui(UiCommand::Settings(self.config.read().unwrap().clone())),
            ),
            HostCommand::Diagnostics => {
                self.ui_response(self.request_ui(UiCommand::Diagnostics(self.diagnostics())))
            }
            HostCommand::SaveConfig(config) => self.save_config(config),
            HostCommand::Quit => HostResponse::Bye,
        }
    }

    pub fn config(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    pub fn request_ui(&self, command: UiCommand) -> Result<UiResponse, String> {
        self.adapter.request_ui(command)
    }

    pub fn apply_hotkey(&self) {
        let hotkey = self.config.read().unwrap().hotkey.clone();
        let result = self.adapter.register_hotkey(&hotkey);
        *self.hotkey_status.write().unwrap() = match &result {
            Ok(()) => format!("registered {}", hotkey.display()),
            Err(error) => format!("conflict: {error}"),
        };
        if let Err(error) = result {
            self.adapter.notify(
                "Recentry hotkey",
                &format!("{error}; use the tray to open Recentry."),
                true,
            );
        }
    }

    pub fn hotkey_status(&self) -> String {
        self.hotkey_status.read().unwrap().clone()
    }

    pub fn diagnostics(&self) -> String {
        let config = self.config.read().unwrap();
        format!(
            "Recentry {}\nHotkey: {}\nAutostart: {}\nConfig: cfg#{}\nTelemetry: disabled\nNetwork: disabled",
            env!("CARGO_PKG_VERSION"),
            self.hotkey_status.read().unwrap(),
            config.autostart,
            fingerprint(self.store.path()),
        )
    }

    fn save_config(&self, config: Config) -> HostResponse {
        if let Err(error) = config.validate() {
            return HostResponse::Error(error);
        }
        let previous = self.config.read().unwrap().clone();
        let update = self
            .adapter
            .set_autostart(config.autostart, &self.executable)
            .and_then(|_| self.store.save(&config).map_err(|error| error.to_string()));
        if let Err(error) = update {
            let _ = self
                .adapter
                .set_autostart(previous.autostart, &self.executable);
            return HostResponse::Error(error);
        }
        *self.config.write().unwrap() = config;
        self.adapter.configuration_changed();
        HostResponse::Saved
    }

    fn ui_response(&self, response: Result<UiResponse, String>) -> HostResponse {
        match response {
            Ok(UiResponse::Error(error)) | Err(error) => HostResponse::Error(error),
            Ok(_) => HostResponse::Accepted,
        }
    }
}

fn fingerprint(path: &Path) -> String {
    let hash = path
        .to_string_lossy()
        .bytes()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use recentry_protocol::{Config, HostCommand, HostResponse, Hotkey, UiCommand, UiResponse};
    use tempfile::tempdir;

    use crate::ConfigStore;

    use super::*;

    #[derive(Clone, Default)]
    struct FakeAdapter {
        calls: Arc<Mutex<Vec<String>>>,
        hotkey_error: Arc<Mutex<Option<String>>>,
        ui_error: Arc<Mutex<Option<String>>>,
    }

    impl HostAdapter for FakeAdapter {
        fn request_ui(&self, command: UiCommand) -> Result<UiResponse, String> {
            self.calls.lock().unwrap().push(format!("ui:{command:?}"));
            if let Some(error) = self.ui_error.lock().unwrap().clone() {
                Err(error)
            } else {
                Ok(UiResponse::Shown)
            }
        }

        fn set_autostart(&self, enabled: bool, _executable: &Path) -> Result<(), String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("autostart:{enabled}"));
            Ok(())
        }

        fn register_hotkey(&self, hotkey: &Hotkey) -> Result<(), String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("hotkey:{}", hotkey.display()));
            match self.hotkey_error.lock().unwrap().clone() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }

        fn configuration_changed(&self) {
            self.calls
                .lock()
                .unwrap()
                .push("configuration_changed".to_owned());
        }

        fn notify(&self, title: &str, message: &str, error: bool) {
            self.calls
                .lock()
                .unwrap()
                .push(format!("notify:{title}:{message}:{error}"));
        }
    }

    fn runtime(path: PathBuf, adapter: FakeAdapter) -> HostRuntime {
        HostRuntime::new(
            ConfigStore::new(path),
            Config::default(),
            PathBuf::from("recentry"),
            Arc::new(adapter),
        )
    }

    #[test]
    fn routes_ui_and_reports_failures_without_exposing_adapter_details() {
        let directory = tempdir().unwrap();
        let adapter = FakeAdapter::default();
        let runtime = runtime(directory.path().join("config.json"), adapter.clone());

        assert_eq!(runtime.dispatch(HostCommand::Ping), HostResponse::Pong);
        assert_eq!(runtime.dispatch(HostCommand::Show), HostResponse::Accepted);
        assert_eq!(
            runtime.dispatch(HostCommand::Diagnostics),
            HostResponse::Accepted
        );

        *adapter.ui_error.lock().unwrap() = Some("UI crashed".to_owned());
        assert_eq!(
            runtime.dispatch(HostCommand::Show),
            HostResponse::Error("UI crashed".to_owned())
        );
        assert_eq!(runtime.dispatch(HostCommand::Quit), HostResponse::Bye);
    }

    #[test]
    fn saves_config_atomically_and_rolls_back_autostart_on_failure() {
        let directory = tempdir().unwrap();
        let target_is_directory = directory.path().join("config.json");
        std::fs::create_dir(&target_is_directory).unwrap();
        let adapter = FakeAdapter::default();
        let runtime = runtime(target_is_directory, adapter.clone());
        let updated = Config {
            autostart: true,
            first_run_completed: true,
            ..Config::default()
        };

        assert!(matches!(
            runtime.dispatch(HostCommand::SaveConfig(updated)),
            HostResponse::Error(_)
        ));
        assert_eq!(runtime.config(), Config::default());
        assert!(
            adapter
                .calls
                .lock()
                .unwrap()
                .ends_with(&["autostart:true".to_owned(), "autostart:false".to_owned()])
        );
    }

    #[test]
    fn successful_save_defers_platform_reconfiguration_to_the_platform_thread() {
        let directory = tempdir().unwrap();
        let adapter = FakeAdapter::default();
        let runtime = runtime(directory.path().join("config.json"), adapter.clone());
        let updated = Config {
            first_run_completed: true,
            ..Config::default()
        };

        assert_eq!(
            runtime.dispatch(HostCommand::SaveConfig(updated.clone())),
            HostResponse::Saved
        );
        assert_eq!(runtime.config(), updated);
        assert_eq!(
            adapter.calls.lock().unwrap().as_slice(),
            ["autostart:false", "configuration_changed"]
        );
    }

    #[test]
    fn hotkey_conflict_is_diagnostic_and_keeps_the_host_available() {
        let directory = tempdir().unwrap();
        let adapter = FakeAdapter::default();
        *adapter.hotkey_error.lock().unwrap() = Some("already registered".to_owned());
        let runtime = runtime(directory.path().join("config.json"), adapter.clone());

        runtime.apply_hotkey();

        assert_eq!(runtime.hotkey_status(), "conflict: already registered");
        assert!(adapter.calls.lock().unwrap().iter().any(|call| {
            call.contains("notify:Recentry hotkey") && call.contains("use the tray")
        }));
    }
}
