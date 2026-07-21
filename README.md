# Recentry

Recentry is a low-resource Windows launcher for recent development projects. Press `Ctrl+Alt+R`, type to filter, then use the arrow keys and Enter to open or focus a VS Code project.

The launcher uses a compact single-line layout and dismisses itself when it loses focus, when you press Esc, or after a project opens successfully.

The current beta supports Windows x64 and the stable edition of VS Code. Linux and macOS builds are not published or supported yet.

## Install

### Installer (recommended)

1. Download `Recentry-0.1.0-beta.1-windows-x64-setup.exe` and `Recentry-0.1.0-beta.1-windows-x64-SHA256SUMS.txt`.
2. Verify the installer hash against the checksum file.
3. Run the installer. It installs per-user to `%LOCALAPPDATA%\Programs\Recentry` and does not require administrator privileges.
4. Leave **Open Recentry** selected on the final page.

This beta is unsigned, so Windows SmartScreen may show a warning. Verify the SHA-256 checksum before choosing **More info** and **Run anyway**.

### Portable ZIP

1. Download and extract `Recentry-0.1.0-beta.1-windows-x64.zip`.
2. Keep `recentry.exe` and `recentry-ui.exe` in the same directory.
3. Run `recentry.exe`; the project list opens immediately.

## Use

- Press `Ctrl+Alt+R` to open Recentry from anywhere.
- Type ordinary text to filter the discovered projects.
- Use Up and Down to move, Enter to open, and Esc to dismiss.
- Clicking another window dismisses the launcher automatically.
- Use the tray menu for **Open Recentry**, **Settings**, **Diagnostics**, or **Quit**.

On first run, Recentry asks whether it may start when you sign in. It writes the current-user startup entry only after explicit confirmation. The default hotkey can be changed in **Settings**.

Each result stays on one line: `[VSCode] project-name path`. An empty query preserves VS Code's recent order. Search text is only a filter; it is never executed as a path or command.

## Command line

```powershell
recentry.exe show
recentry.exe settings
recentry.exe diagnostics
recentry.exe quit
```

Running `recentry.exe` without arguments is equivalent to `recentry.exe show`. The sign-in startup entry uses the internal `--background` option and does not open the launcher at login.

## VS Code compatibility

- Local folders, `.code-workspace` files, and remote project URIs supported by VS Code.
- Read-only access to both the application-shared `state.vscdb` and the legacy `globalStorage/state.vscdb` locations.
- Ordinary recent files are excluded.
- Already-open projects request focus; other projects explicitly open in a new window.
- A stable VS Code executable can be selected manually in Settings.
- VS Code Insiders, Cursor, and other editors are not supported in this beta.

## Resources and privacy

The formal Windows 11 x64 release acceptance baseline recorded a 1.043 MiB host Private Working Set, 0% idle CPU, a 4.168 MiB active process tree, and cold/warm popup p95 values of 174.852/88.725 ms over 30 iterations. See the [production acceptance report](docs/performance/2026-07-21-production-windows-acceptance.md) for the complete measurement method.

The final compact UI revision was also regression-checked at a 1.047 MiB host Private Working Set, 0% idle CPU, and a 2.832 MiB active process tree.

Recentry has no telemetry, cloud synchronization, or background network requests. Diagnostics expose configuration hashes and runtime state without project paths. Configuration is stored at `%APPDATA%\Recentry\config.json` and is retained during uninstall so it can be reused after reinstalling; delete that directory manually for a complete reset.

## Development

Building requires Rust 1.86 or newer and the Windows SDK. Creating the NSIS installer also requires NSIS 3.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

The authoritative Windows packaging commands are:

```powershell
pwsh -File tools/package-windows.ps1 -MakeNsis C:\path\to\makensis.exe
pwsh -File tools/test-windows-package.ps1
```

The packaging command creates the NSIS installer, portable ZIP, and SHA-256 checksum file in `dist/`. It refuses to replace existing artifacts unless `-Force` is passed explicitly. The package smoke test uses `scratch/package-smoke/` and removes the test directory after a successful run.

## Architecture

- `recentry.exe`: the single-instance lightweight host for the tray, global hotkey, configuration, IPC, and UI lifecycle.
- `recentry-ui.exe`: the on-demand Win32 UI for VS Code discovery, search, and project opening; it exits after remaining hidden for 15 minutes.
- `recentry-core`: portable project models, provider/opener contracts, ranking, and VS Code compatibility logic.

License: [MIT](LICENSE).
