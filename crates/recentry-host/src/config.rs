use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
};

use recentry_protocol::Config;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("configuration I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuration JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("configuration is invalid: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub config: Config,
    pub is_new: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<LoadedConfig, ConfigError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LoadedConfig {
                    config: Config::default(),
                    is_new: true,
                });
            }
            Err(error) => return Err(error.into()),
        };
        let config: Config = serde_json::from_slice(&bytes)?;
        config.validate().map_err(ConfigError::Invalid)?;
        Ok(LoadedConfig {
            config,
            is_new: false,
        })
    }

    pub fn save(&self, config: &Config) -> Result<(), ConfigError> {
        config.validate().map_err(ConfigError::Invalid)?;
        let parent = self.path.parent().ok_or_else(|| {
            ConfigError::Invalid("configuration path has no parent directory".to_owned())
        })?;
        fs::create_dir_all(parent)?;
        let mut serialized = serde_json::to_vec_pretty(config)?;
        serialized.push(b'\n');

        let temporary = temporary_path(&self.path);
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(&serialized)?;
            file.sync_all()?;
            drop(file);
            atomic_replace(&temporary, &self.path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(ConfigError::Io)
    }
}

fn temporary_path(target: &Path) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");
    target.with_file_name(format!(".{name}.{}.{}.tmp", process::id(), id))
}

#[cfg(windows)]
fn atomic_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let ok = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use recentry_protocol::{CONFIG_VERSION, Language};

    use super::*;

    #[test]
    fn missing_config_loads_defaults_without_writing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Recentry/config.json");
        let loaded = ConfigStore::new(path.clone()).load().unwrap();
        assert!(loaded.is_new);
        assert_eq!(loaded.config.version, CONFIG_VERSION);
        assert!(!path.exists());
    }

    #[test]
    fn save_is_reloadable_and_leaves_no_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Recentry/config.json");
        let store = ConfigStore::new(path.clone());
        let config = Config {
            language: Language::ZhCn,
            autostart: true,
            first_run_completed: true,
            ..Config::default()
        };
        store.save(&config).unwrap();
        let loaded = store.load().unwrap();
        assert!(!loaded.is_new);
        assert_eq!(loaded.config, config);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn invalid_update_does_not_replace_the_last_good_config() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        let store = ConfigStore::new(path.clone());
        let good = Config::default();
        store.save(&good).unwrap();
        let mut invalid = good.clone();
        invalid.version = CONFIG_VERSION + 1;
        assert!(matches!(store.save(&invalid), Err(ConfigError::Invalid(_))));
        assert_eq!(store.load().unwrap().config, good);
    }
}
