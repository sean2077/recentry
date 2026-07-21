use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{Connection, OpenFlags, types::ValueRef};
use serde_json::Value;
use url::Url;

use crate::{
    DiagnosticLevel, ProjectId, ProjectKind, ProjectTarget, ProviderDiagnostic, ProviderId,
    ProviderReport, RecentProject, RecentProjectProvider, deduplicate, target_key,
};

pub const RECENT_KEY: &str = "history.recentlyOpenedPathsList";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryEnvironment {
    pub home: PathBuf,
    pub app_data: PathBuf,
    pub local_app_data: PathBuf,
    pub program_files: PathBuf,
    pub program_files_x86: PathBuf,
    pub path: Vec<PathBuf>,
}

impl DiscoveryEnvironment {
    pub fn current() -> Self {
        let value = |name: &str| env::var_os(name).map(PathBuf::from).unwrap_or_default();
        Self {
            home: value("USERPROFILE"),
            app_data: value("APPDATA"),
            local_app_data: value("LOCALAPPDATA"),
            program_files: value("ProgramFiles"),
            program_files_x86: value("ProgramFiles(x86)"),
            path: env::var_os("PATH")
                .map(|value| env::split_paths(&value).collect())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VsCodeInstallation {
    pub code_exe: PathBuf,
    pub product_json: PathBuf,
    pub version: String,
    pub name_short: String,
    pub shared_data_folder_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum VsCodeError {
    #[error("VS Code stable installation was not found")]
    NotFound,
    #[error("invalid VS Code product metadata: {0}")]
    InvalidProduct(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum RecentFormatError {
    #[error("recent value is invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("recent value has an unsupported root shape")]
    UnsupportedRoot,
}

pub fn discover_vscode(
    environment: &DiscoveryEnvironment,
    override_path: Option<&Path>,
) -> Result<VsCodeInstallation, VsCodeError> {
    if let Some(override_path) = override_path {
        let executable = normalize_executable_candidate(override_path);
        return installation_from_executable(&executable);
    }

    let mut candidates = Vec::new();
    for directory in &environment.path {
        candidates.push(directory.join("Code.exe"));
        candidates.push(directory.join("code.exe"));
        if directory
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
        {
            if let Some(parent) = directory.parent() {
                candidates.push(parent.join("Code.exe"));
            }
        }
    }
    candidates.extend([
        environment
            .local_app_data
            .join("Programs/Microsoft VS Code/Code.exe"),
        environment.program_files.join("Microsoft VS Code/Code.exe"),
        environment
            .program_files_x86
            .join("Microsoft VS Code/Code.exe"),
    ]);

    let mut seen = HashSet::new();
    for candidate in candidates {
        let key = candidate.to_string_lossy().to_lowercase();
        if !seen.insert(key) || !candidate.is_file() {
            continue;
        }
        if let Ok(installation) = installation_from_executable(&candidate) {
            return Ok(installation);
        }
    }
    Err(VsCodeError::NotFound)
}

pub fn database_candidates(
    installation: &VsCodeInstallation,
    environment: &DiscoveryEnvironment,
) -> Vec<PathBuf> {
    vec![
        environment
            .home
            .join(&installation.shared_data_folder_name)
            .join("sharedStorage/state.vscdb"),
        environment
            .app_data
            .join(&installation.name_short)
            .join("User/globalStorage/state.vscdb"),
    ]
}

pub fn window_state_candidates(
    installation: &VsCodeInstallation,
    environment: &DiscoveryEnvironment,
) -> Vec<PathBuf> {
    vec![
        environment
            .app_data
            .join(&installation.name_short)
            .join("User/globalStorage/storage.json"),
    ]
}

pub fn parse_recent_value(value: &str) -> Result<Vec<RecentProject>, RecentFormatError> {
    let parsed: Value = serde_json::from_str(value)?;
    let entries = match &parsed {
        Value::Object(object) => object
            .get("entries")
            .and_then(Value::as_array)
            .ok_or(RecentFormatError::UnsupportedRoot)?,
        Value::Array(entries) => entries,
        _ => return Err(RecentFormatError::UnsupportedRoot),
    };

    let projects = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| recent_project(entry, index as u32))
        .collect();
    Ok(deduplicate(projects))
}

pub struct VsCodeRecentProvider {
    pub environment: DiscoveryEnvironment,
    pub override_path: Option<PathBuf>,
}

impl RecentProjectProvider for VsCodeRecentProvider {
    fn discover(&self) -> ProviderReport<Vec<RecentProject>> {
        let installation = match discover_vscode(&self.environment, self.override_path.as_deref()) {
            Ok(installation) => installation,
            Err(error) => {
                return ProviderReport {
                    value: Vec::new(),
                    diagnostics: vec![ProviderDiagnostic {
                        level: DiagnosticLevel::Error,
                        code: "vscode_not_found",
                        message: error.to_string(),
                    }],
                };
            }
        };
        let candidates = database_candidates(&installation, &self.environment);
        let mut diagnostics = Vec::new();
        for (index, database) in candidates.iter().enumerate() {
            if !database.is_file() {
                continue;
            }
            match read_recent_value(database) {
                Ok(Some(value)) => match parse_recent_value(&value) {
                    Ok(projects) => {
                        diagnostics.push(ProviderDiagnostic {
                            level: DiagnosticLevel::Info,
                            code: "vscode_recent_loaded",
                            message: format!(
                                "loaded {} projects from db#{}",
                                projects.len(),
                                path_fingerprint(database)
                            ),
                        });
                        return ProviderReport {
                            value: projects,
                            diagnostics,
                        };
                    }
                    Err(error) => {
                        diagnostics.push(ProviderDiagnostic {
                            level: DiagnosticLevel::Error,
                            code: "vscode_recent_format_changed",
                            message: format!("db#{}: {error}", path_fingerprint(database)),
                        });
                        if index == 0 {
                            return ProviderReport {
                                value: Vec::new(),
                                diagnostics,
                            };
                        }
                    }
                },
                Ok(None) => diagnostics.push(ProviderDiagnostic {
                    level: DiagnosticLevel::Info,
                    code: "vscode_recent_key_missing",
                    message: format!("db#{} has no recent key", path_fingerprint(database)),
                }),
                Err(error) => {
                    diagnostics.push(ProviderDiagnostic {
                        level: DiagnosticLevel::Error,
                        code: "vscode_database_read_failed",
                        message: format!("db#{}: {error}", path_fingerprint(database)),
                    });
                    if index == 0 {
                        return ProviderReport {
                            value: Vec::new(),
                            diagnostics,
                        };
                    }
                }
            }
        }
        if diagnostics.is_empty() {
            diagnostics.push(ProviderDiagnostic {
                level: DiagnosticLevel::Warning,
                code: "vscode_database_not_found",
                message: "no VS Code recent database was found".to_owned(),
            });
        }
        ProviderReport {
            value: Vec::new(),
            diagnostics,
        }
    }
}

fn normalize_executable_candidate(path: &Path) -> PathBuf {
    if path.is_dir() {
        return path.join("Code.exe");
    }
    if path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("code.cmd"))
        && path
            .parent()
            .and_then(Path::parent)
            .is_some_and(|parent| parent.join("Code.exe").is_file())
    {
        return path.parent().unwrap().parent().unwrap().join("Code.exe");
    }
    path.to_owned()
}

fn installation_from_executable(executable: &Path) -> Result<VsCodeInstallation, VsCodeError> {
    if !executable.is_file() {
        return Err(VsCodeError::NotFound);
    }
    let directory = executable.parent().ok_or_else(|| {
        VsCodeError::InvalidProduct("Code.exe has no parent directory".to_owned())
    })?;
    let mut products = vec![directory.join("resources/app/product.json")];
    let mut versioned = fs::read_dir(directory)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let product = entry.path().join("resources/app/product.json");
            product.is_file().then(|| {
                let modified = entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok();
                (modified, product)
            })
        })
        .collect::<Vec<_>>();
    versioned.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    products.extend(versioned.into_iter().map(|(_, product)| product));

    for product_json in products {
        if !product_json.is_file() {
            continue;
        }
        let value: Value = serde_json::from_slice(&fs::read(&product_json)?)?;
        let application_name = value
            .get("applicationName")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if application_name != "code" {
            continue;
        }
        let required = |name: &str| {
            value
                .get(name)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| VsCodeError::InvalidProduct(format!("missing {name}")))
        };
        return Ok(VsCodeInstallation {
            code_exe: executable.to_owned(),
            product_json,
            version: required("version")?,
            name_short: required("nameShort")?,
            shared_data_folder_name: required("sharedDataFolderName")?,
        });
    }
    Err(VsCodeError::InvalidProduct(
        "stable product.json was not found next to Code.exe".to_owned(),
    ))
}

fn read_recent_value(database: &Path) -> Result<Option<String>, String> {
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| error.to_string())?;
    connection
        .busy_timeout(Duration::from_millis(100))
        .map_err(|error| error.to_string())?;
    match connection.query_row(
        "SELECT value FROM ItemTable WHERE key = ?1",
        [RECENT_KEY],
        |row| match row.get_ref(0)? {
            ValueRef::Text(value) | ValueRef::Blob(value) => {
                Ok(String::from_utf8_lossy(value).into_owned())
            }
            _ => Err(rusqlite::Error::InvalidColumnType(
                0,
                "value".to_owned(),
                row.get_ref(0)?.data_type(),
            )),
        },
    ) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn recent_project(entry: &Value, recent_index: u32) -> Option<RecentProject> {
    let object = entry.as_object()?;
    let label = object.get("label").and_then(Value::as_str);
    let (kind, raw_target) = if let Some(folder) = object.get("folderUri") {
        (ProjectKind::Folder, uri_from_value(folder)?)
    } else if let Some(workspace) = object.get("workspace") {
        let target = workspace
            .get("configPath")
            .and_then(uri_from_value)
            .or_else(|| workspace.get("uri").and_then(uri_from_value))
            .or_else(|| uri_from_value(workspace))?;
        (ProjectKind::Workspace, target)
    } else if let Some(workspace) = object.get("workspaceUri") {
        (ProjectKind::Workspace, uri_from_value(workspace)?)
    } else if let Some(file) = object.get("fileUri") {
        let target = uri_from_value(file)?;
        if !is_workspace_target(&target) {
            return None;
        }
        (ProjectKind::Workspace, target)
    } else {
        return None;
    };

    let target = project_target(&raw_target)?;
    let fallback_name = target_name(kind, &target)?;
    let name = label
        .filter(|label| !label.trim().is_empty())
        .unwrap_or(&fallback_name)
        .to_owned();
    let detail = match &target {
        ProjectTarget::LocalPath(path) => path.to_string_lossy().into_owned(),
        ProjectTarget::Uri(uri) => uri.clone(),
    };
    let key = target_key(&target);
    Some(RecentProject {
        id: ProjectId(format!("vscode:{:016x}", fnv1a(key.as_bytes()))),
        provider: ProviderId("vscode".to_owned()),
        kind,
        target,
        name,
        detail,
        recent_index,
    })
}

fn is_workspace_target(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .map(|url| url.path().to_lowercase().ends_with(".code-workspace"))
        .unwrap_or_else(|| value.to_lowercase().ends_with(".code-workspace"))
}

fn uri_from_value(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_owned());
    }
    let object = value.as_object()?;
    if let Some(external) = object.get("external").and_then(Value::as_str) {
        return Some(external.to_owned());
    }
    let scheme = object.get("scheme").and_then(Value::as_str)?;
    let authority = object
        .get("authority")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let path = object
        .get("path")
        .or_else(|| object.get("fsPath"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .replace('\\', "/");
    let raw = if authority.is_empty() {
        format!("{scheme}://{path}")
    } else {
        format!("{scheme}://{authority}{path}")
    };
    Url::parse(&raw).ok().map(Into::into)
}

fn project_target(value: &str) -> Option<ProjectTarget> {
    if let Ok(url) = Url::parse(value) {
        if url.scheme() == "file" {
            if url
                .host_str()
                .is_some_and(|host| !host.eq_ignore_ascii_case("localhost"))
            {
                return None;
            }
            return url.to_file_path().ok().map(ProjectTarget::LocalPath);
        }
        if matches!(url.scheme(), "vscode-remote" | "vscode") {
            return Some(ProjectTarget::Uri(url.into()));
        }
        return None;
    }
    let path = PathBuf::from(value);
    path.is_absolute().then_some(ProjectTarget::LocalPath(path))
}

fn target_name(kind: ProjectKind, target: &ProjectTarget) -> Option<String> {
    let name = match target {
        ProjectTarget::LocalPath(path) => match kind {
            ProjectKind::Folder => path.file_name()?.to_string_lossy().into_owned(),
            ProjectKind::Workspace => path.file_stem()?.to_string_lossy().into_owned(),
        },
        ProjectTarget::Uri(value) => {
            let url = Url::parse(value).ok()?;
            let segment = url.path_segments()?.next_back()?.to_owned();
            match kind {
                ProjectKind::Folder => segment,
                ProjectKind::Workspace => segment
                    .strip_suffix(".code-workspace")
                    .unwrap_or(&segment)
                    .to_owned(),
            }
        }
    };
    (!name.is_empty()).then_some(name)
}

fn path_fingerprint(path: &Path) -> String {
    format!("{:016x}", fnv1a(path.to_string_lossy().as_bytes()))
}

fn fnv1a(value: &[u8]) -> u64 {
    value.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
