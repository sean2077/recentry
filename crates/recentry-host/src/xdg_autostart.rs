use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn set_xdg_autostart(
    config_home: &Path,
    enabled: bool,
    executable: &Path,
) -> Result<(), String> {
    let directory = config_home.join("autostart");
    let target = directory.join("io.github.sean2077.recentry.desktop");
    if !enabled {
        return match fs::remove_file(&target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("removing XDG autostart entry failed: {error}")),
        };
    }

    let command = desktop_exec(executable)?;
    let contents = format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Recentry\nComment=Open recent development projects\nExec={command}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    );
    fs::create_dir_all(&directory)
        .map_err(|error| format!("creating XDG autostart directory failed: {error}"))?;
    let temporary = temporary_path(&target);
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("writing XDG autostart entry failed: {error}"))
}

fn desktop_exec(executable: &Path) -> Result<String, String> {
    let value = executable
        .to_str()
        .ok_or_else(|| "the executable path is not valid UTF-8".to_owned())?;
    if value.chars().any(char::is_control) || value.contains('=') {
        return Err("the executable path cannot be represented in an XDG desktop entry".to_owned());
    }
    let mut escaped = String::with_capacity(value.len() + 16);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '%' => escaped.push_str("%%"),
            '"' | '`' | '$' => {
                escaped.push_str(r"\\");
                escaped.push(character);
            }
            '\\' => escaped.push_str(r"\\\\"),
            _ => escaped.push(character),
        }
    }
    escaped.push_str("\" --background");
    Ok(escaped)
}

fn temporary_path(target: &Path) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    target.with_file_name(format!(
        ".io.github.sean2077.recentry.{}.{}.tmp",
        process::id(),
        id
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_removes_an_xdg_autostart_entry() {
        let directory = tempfile::tempdir().unwrap();
        let executable = Path::new(r"/opt/Recentry %build $channel/back\slash/recentry");
        set_xdg_autostart(directory.path(), true, executable).unwrap();
        let entry = fs::read_to_string(
            directory
                .path()
                .join("autostart/io.github.sean2077.recentry.desktop"),
        )
        .unwrap();
        assert!(entry.contains(
            r#"Exec="/opt/Recentry %%build \\$channel/back\\\\slash/recentry" --background"#
        ));
        assert!(!entry.contains("sh -c"));

        set_xdg_autostart(directory.path(), false, executable).unwrap();
        assert!(
            !directory
                .path()
                .join("autostart/io.github.sean2077.recentry.desktop")
                .exists()
        );
    }

    #[test]
    fn rejects_control_characters_in_an_exec_path() {
        assert!(desktop_exec(Path::new("/tmp/recentry\nother")).is_err());
        assert!(desktop_exec(Path::new("/tmp/recentry=other")).is_err());
    }
}
