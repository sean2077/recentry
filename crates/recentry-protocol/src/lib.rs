use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u32 = 1;
pub const HOST_PIPE_NAME: &str = r"\\.\pipe\recentry-host-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    ZhCn,
    En,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: false,
            win: false,
            key: "R".to_owned(),
        }
    }
}

impl Hotkey {
    pub fn display(&self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.win {
            parts.push("Win");
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub language: Language,
    pub hotkey: Hotkey,
    pub autostart: bool,
    pub vscode_path_override: Option<String>,
    pub first_run_completed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            language: Language::System,
            hotkey: Hotkey::default(),
            autostart: false,
            vscode_path_override: None,
            first_run_completed: false,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != CONFIG_VERSION {
            return Err(format!("unsupported config version {}", self.version));
        }
        let key = self.hotkey.key.trim();
        let valid_key = key.len() == 1 && key.bytes().all(|value| value.is_ascii_alphanumeric())
            || key
                .strip_prefix('F')
                .and_then(|value| value.parse::<u8>().ok())
                .is_some_and(|value| (1..=24).contains(&value));
        if !valid_key {
            return Err("hotkey key must be A-Z, 0-9, or F1-F24".to_owned());
        }
        if !(self.hotkey.ctrl || self.hotkey.alt || self.hotkey.shift || self.hotkey.win) {
            return Err("hotkey must contain at least one modifier".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum HostCommand {
    Ping,
    Show,
    Settings,
    Diagnostics,
    SaveConfig(Config),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum HostResponse {
    Pong,
    Accepted,
    Saved,
    Error(String),
    Bye,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum UiCommand {
    Show,
    Hide,
    Settings(Config),
    Diagnostics(String),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum UiResponse {
    Ready,
    Hidden,
    Shown,
    Error(String),
    Quitting,
}

pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(message)
}

pub fn decode<'a, T: Deserialize<'a>>(message: &'a [u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_stable_version_and_hotkey() {
        let config = Config::default();
        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.hotkey.display(), "Ctrl+Alt+R");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn protocol_round_trips_config_updates() {
        let command = HostCommand::SaveConfig(Config {
            language: Language::ZhCn,
            autostart: true,
            first_run_completed: true,
            ..Config::default()
        });
        let encoded = encode(&command).unwrap();
        assert_eq!(decode::<HostCommand>(&encoded).unwrap(), command);
    }

    #[test]
    fn config_rejects_unmodified_keys() {
        let mut config = Config::default();
        config.hotkey.ctrl = false;
        config.hotkey.alt = false;
        assert!(config.validate().is_err());
    }
}
