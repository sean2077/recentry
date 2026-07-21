use std::{ffi::OsString, fs, path::PathBuf};

use recentry_core::{
    DiagnosticLevel, DiscoveryEnvironment, OpenOutcome, ProjectId, ProjectKind, ProjectOpener,
    ProjectTarget, ProviderId, RecentProject, RecentProjectProvider, VsCodeInstallation,
    VsCodeOpener, VsCodeRecentProvider, build_launch_request, database_candidates, deduplicate,
    discover_vscode, parse_recent_value, search_projects, window_state_targets,
};

fn project(name: &str, target: &str, index: u32) -> RecentProject {
    RecentProject {
        id: ProjectId(format!("vscode:{index}")),
        provider: ProviderId("vscode".to_owned()),
        kind: ProjectKind::Folder,
        target: ProjectTarget::LocalPath(PathBuf::from(target)),
        name: name.to_owned(),
        detail: target.to_owned(),
        recent_index: index,
    }
}

#[test]
fn parses_current_and_legacy_recent_shapes_and_excludes_plain_files() {
    let json = r#"{
        "entries": [
            {"folderUri":"file:///C:/work/recentry"},
            {"workspace":{"configPath":{"scheme":"file","path":"/C:/work/team.code-workspace"}}},
            {"fileUri":"file:///C:/work/notes.txt"},
            {"fileUri":"file:///C:/work/legacy.code-workspace?window=1#recent"},
            {"folderUri":{"scheme":"vscode-remote","authority":"ssh-remote+devbox","path":"/work/api"}}
        ]
    }"#;

    let projects = parse_recent_value(json).unwrap();
    assert_eq!(projects.len(), 4);
    assert_eq!(projects[0].name, "recentry");
    assert_eq!(projects[1].kind, ProjectKind::Workspace);
    assert_eq!(projects[2].name, "legacy");
    assert!(matches!(projects[3].target, ProjectTarget::Uri(_)));
    assert_eq!(projects[0].recent_index, 0);
    assert_eq!(projects[3].recent_index, 4);
}

#[test]
fn empty_search_preserves_recent_order_and_text_search_ranks_name_first() {
    let projects = vec![
        project("Other", r"C:\work\recentry-docs", 0),
        project("Recentry", r"C:\work\app", 1),
        project("recentry-tools", r"C:\work\tools", 2),
    ];
    assert_eq!(
        search_projects(&projects, "")
            .iter()
            .map(|item| item.recent_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(search_projects(&projects, "recentry")[0].name, "Recentry");
}

#[test]
fn deduplication_is_case_insensitive_for_windows_paths_and_keeps_newest() {
    let projects = vec![
        project("First", r"C:\Work\Recentry", 0),
        project("Duplicate", r"c:\work\recentry\", 1),
    ];
    let deduped = deduplicate(projects);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].name, "First");
}

#[test]
fn shared_database_precedes_legacy_global_storage() {
    let environment = DiscoveryEnvironment {
        home: PathBuf::from(r"C:\Users\tester"),
        app_data: PathBuf::from(r"C:\Users\tester\AppData\Roaming"),
        local_app_data: PathBuf::from(r"C:\Users\tester\AppData\Local"),
        program_files: PathBuf::from(r"C:\Program Files"),
        program_files_x86: PathBuf::from(r"C:\Program Files (x86)"),
        path: Vec::new(),
    };
    let installation = VsCodeInstallation {
        code_exe: PathBuf::from(r"C:\Code\Code.exe"),
        product_json: PathBuf::from(r"C:\Code\resources\app\product.json"),
        version: "1.129.1".to_owned(),
        name_short: "Code".to_owned(),
        shared_data_folder_name: ".vscode-shared".to_owned(),
    };
    let candidates = database_candidates(&installation, &environment);
    assert_eq!(
        candidates,
        vec![
            PathBuf::from(r"C:\Users\tester\.vscode-shared\sharedStorage\state.vscdb"),
            PathBuf::from(r"C:\Users\tester\AppData\Roaming\Code\User\globalStorage\state.vscdb"),
        ]
    );
}

#[test]
fn launch_arguments_focus_open_folder_and_create_new_workspace_window() {
    let executable = PathBuf::from(r"C:\Code\Code.exe");
    let folder = project("Recentry", r"C:\work\recentry", 0);
    let folder_request = build_launch_request(
        executable.clone(),
        &folder,
        &["file:///C:/work/recentry".to_owned()],
    )
    .unwrap();
    assert_eq!(folder_request.outcome, OpenOutcome::Focused);
    assert!(
        !folder_request
            .args
            .contains(&OsString::from("--new-window"))
    );
    assert_eq!(folder_request.args[0], "--folder-uri");

    let workspace = RecentProject {
        kind: ProjectKind::Workspace,
        target: ProjectTarget::LocalPath(PathBuf::from(r"C:\work\team.code-workspace")),
        ..project("team", r"C:\work\team.code-workspace", 1)
    };
    let workspace_request = build_launch_request(executable, &workspace, &[]).unwrap();
    assert_eq!(workspace_request.outcome, OpenOutcome::OpenedNew);
    assert_eq!(workspace_request.args[0], "--new-window");
    assert_eq!(workspace_request.args[1], "--file-uri");

    let remote = RecentProject {
        target: ProjectTarget::Uri("vscode-remote://ssh-remote+devbox/work/api".to_owned()),
        ..project("api", r"C:\unused", 2)
    };
    let remote_request =
        build_launch_request(PathBuf::from(r"C:\Code\Code.exe"), &remote, &[]).unwrap();
    assert_eq!(remote_request.args[1], "--folder-uri");
    assert_eq!(
        remote_request.args[2],
        "vscode-remote://ssh-remote+devbox/work/api"
    );

    let unsupported = RecentProject {
        target: ProjectTarget::Uri("https://example.com/project".to_owned()),
        ..project("unsafe", r"C:\unused", 3)
    };
    assert!(build_launch_request(PathBuf::from(r"C:\Code\Code.exe"), &unsupported, &[]).is_err());
}

#[cfg(windows)]
#[test]
fn opener_starts_a_fake_code_executable_and_reports_spawn_failure() {
    let directory = tempfile::tempdir().unwrap();
    let fake_code = directory.path().join("code.exe");
    let system_root = std::env::var_os("SystemRoot").unwrap();
    fs::copy(
        PathBuf::from(system_root).join("System32/where.exe"),
        &fake_code,
    )
    .unwrap();
    let opener = VsCodeOpener {
        executable: fake_code,
        window_state_files: Vec::new(),
    };
    assert_eq!(
        opener
            .open_or_focus(&project("Recentry", r"C:\work\recentry", 0))
            .unwrap(),
        OpenOutcome::OpenedNew
    );

    let missing = VsCodeOpener {
        executable: directory.path().join("missing-code.exe"),
        window_state_files: Vec::new(),
    };
    assert!(
        missing
            .open_or_focus(&project("Recentry", r"C:\work\recentry", 0))
            .is_err()
    );
}

#[test]
fn window_state_extracts_folder_and_workspace_targets() {
    let value = serde_json::json!({
        "windowsState": {
            "lastActiveWindow": {"folder": "file:///C:/work/recentry"},
            "openedWindows": [
                {"workspaceIdentifier": {"configURIPath": "file:///C:/work/team.code-workspace"}}
            ]
        }
    });
    assert_eq!(
        window_state_targets(&value),
        vec![
            "file:///C:/work/recentry".to_owned(),
            "file:///C:/work/team.code-workspace".to_owned()
        ]
    );
}

#[test]
fn malformed_sqlite_is_a_safe_provider_failure() {
    let directory = tempfile::tempdir().unwrap();
    let code = directory.path().join("Code.exe");
    fs::write(&code, b"").unwrap();
    let product = directory.path().join("resources/app/product.json");
    fs::create_dir_all(product.parent().unwrap()).unwrap();
    fs::write(
        &product,
        r#"{"applicationName":"code","nameShort":"Code","version":"1.0.0","sharedDataFolderName":".vscode-shared"}"#,
    )
    .unwrap();
    let database = directory
        .path()
        .join("home/.vscode-shared/sharedStorage/state.vscdb");
    fs::create_dir_all(database.parent().unwrap()).unwrap();
    fs::write(&database, b"not a sqlite database").unwrap();
    let provider = VsCodeRecentProvider {
        environment: DiscoveryEnvironment {
            home: directory.path().join("home"),
            app_data: directory.path().join("appdata"),
            local_app_data: directory.path().join("local"),
            program_files: directory.path().join("program-files"),
            program_files_x86: directory.path().join("program-files-x86"),
            path: Vec::new(),
        },
        override_path: Some(code),
    };
    let report = provider.discover();
    assert!(report.value.is_empty());
    assert_eq!(report.diagnostics[0].level, DiagnosticLevel::Error);
    assert_eq!(report.diagnostics[0].code, "vscode_database_read_failed");
    assert!(
        !report.diagnostics[0]
            .message
            .contains(directory.path().to_string_lossy().as_ref())
    );
}

#[test]
fn discovers_current_versioned_windows_install_layout() {
    let directory = tempfile::tempdir().unwrap();
    let code = directory.path().join("Microsoft VS Code/Code.exe");
    fs::create_dir_all(code.parent().unwrap()).unwrap();
    fs::write(&code, b"").unwrap();
    let product = directory
        .path()
        .join("Microsoft VS Code/abc123/resources/app/product.json");
    fs::create_dir_all(product.parent().unwrap()).unwrap();
    fs::write(
        &product,
        r#"{"applicationName":"code","nameShort":"Code","version":"1.129.1","sharedDataFolderName":".vscode-shared"}"#,
    )
    .unwrap();
    let environment = DiscoveryEnvironment {
        home: directory.path().join("home"),
        app_data: directory.path().join("appdata"),
        local_app_data: directory.path().join("local"),
        program_files: directory.path().join("program-files"),
        program_files_x86: directory.path().join("program-files-x86"),
        path: Vec::new(),
    };
    let installation = discover_vscode(&environment, Some(&code)).unwrap();
    assert_eq!(installation.product_json, product);
    assert_eq!(installation.version, "1.129.1");
}
