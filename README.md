# Recentry

[![CI](https://github.com/sean2077/recentry/actions/workflows/ci.yml/badge.svg)](https://github.com/sean2077/recentry/actions/workflows/ci.yml)

Recentry is a low-resource, native launcher for reopening recent development projects. Press a global shortcut, type to filter, then use the keyboard to open or focus a project in VS Code.

Cross-platform support is under active development. The latest public end-user build is the unsigned Windows x64 beta, `v0.1.0-beta.2`. This platform-scoped preview is independent of the first supported cross-platform release. Linux and macOS code, CI, and development packaging do not yet satisfy the native UI, resource, signing, or real-machine acceptance gates and are not supported releases.

| Platform | Current status | Public artifacts |
| --- | --- | --- |
| Windows 10/11 x64 | Beta available; Windows 11 acceptance recorded | NSIS installer and portable ZIP for `v0.1.0-beta.2` |
| Linux x86_64/ARM64 | In development; native UI gate blocked pending real desktops | None |
| macOS 13+ Intel/Apple Silicon | In development; AppKit and native acceptance pending | None |

See [Platform support](docs/platform-support.md) for the evidence boundary.

## Install the Windows beta

Download assets only from the [`v0.1.0-beta.2` release](https://github.com/sean2077/recentry/releases/tag/v0.1.0-beta.2).

### Installer

1. Download `Recentry-0.1.0-beta.2-windows-x64-setup.exe` and the Windows SHA-256 file.
2. Verify the installer hash.
3. Run the installer. It installs per-user to `%LOCALAPPDATA%\Programs\Recentry` without administrator privileges.
4. Leave **Open Recentry** selected on the final page.

This beta is unsigned, so SmartScreen may warn. Verify the checksum before choosing **More info** and **Run anyway**.

### Portable ZIP

1. Download and extract `Recentry-0.1.0-beta.2-windows-x64.zip`.
2. Keep `recentry.exe` and `recentry-ui.exe` together.
3. Run `recentry.exe`.

There are no supported Linux or macOS installation instructions yet. Manually dispatched CI artifacts are explicitly named `unverified-development-*`; they are engineering inputs, not releases.

## Use

- Press `Ctrl+Alt+R` to summon the launcher.
- Type ordinary text to filter VS Code's recent projects.
- Use Up and Down to move, Enter to open, and Esc to dismiss.
- Moving focus to another window dismisses the launcher.
- Use the Windows tray menu for **Open Recentry**, **Settings**, **Diagnostics**, or **Quit**.

Each result stays on one compact line: `[VSCode] project-name path`. An empty query preserves VS Code's recent order. Search text is only a filter; it is never treated as a path or command.

The first Windows run asks whether Recentry may start when you sign in. It writes a current-user startup entry only after confirmation. The shortcut, language, startup preference, and stable VS Code override can be changed in Settings.

## Command line

```text
recentry show
recentry settings
recentry diagnostics
recentry quit
```

Running `recentry` without arguments is equivalent to `recentry show`. The internal `--background` option is reserved for login startup and process supervision.

## VS Code compatibility

- Stable VS Code only; Insiders, Cursor, VSCodium, and other editors are outside this release scope.
- Local folders, `.code-workspace` files, and supported VS Code remote URIs.
- Read-only access to application-shared and legacy `state.vscdb` locations.
- Ordinary recent files are excluded.
- Already-open targets request focus; other targets explicitly request a new window.
- Executables receive argument arrays directly; no shell-composed project command is used.

## Resources and privacy

The Windows 11 x64 production baseline measured a 1.043 MiB host Private Working Set, 0% idle CPU, a 4.168 MiB active process tree, and cold/warm p95 activation of 174.852/88.725 ms over 30 samples. After the shared-host extraction and final Win32 thread-affinity fix, local regression measurements remained within the fixed gates at 0.746 MiB, 0% idle CPU, a 2.785 MiB active tree, and 160.932/93.065 ms cold/warm p95. See the original [Windows acceptance report](docs/performance/2026-07-21-production-windows-acceptance.md) and the [shared-host regression](docs/performance/2026-07-21-shared-host-regression.md). These are Windows 11 real-machine measurements; Windows 10 real-machine acceptance remains pending.

Recentry has no telemetry, cloud synchronization, or background network requests. Stable VS Code data is read only. Diagnostics hash configuration paths and do not list project paths.

## Development

Rust 1.86 is the minimum supported toolchain. Run the full local contract with:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

Native CI runs the workspace on Windows x64, Linux x86_64/ARM64, and macOS Intel/Apple Silicon. Compilation or hosted CI alone never changes a platform's support status.

Authoritative development packaging commands are:

```text
pwsh -File tools/package-windows.ps1
bash tools/package-linux.sh --development --arch <x86_64|aarch64> --appimagetool <path>
bash tools/package-macos.sh --development
bash tools/package-manifest.sh --development
```

Windows packaging requires NSIS 3. Linux packaging requires `dpkg-deb`, `desktop-file-validate`, and a verified `appimagetool`. macOS Universal 2 packaging requires Xcode command-line tools. Native smoke commands live beside their package commands under `tools/`. Release mode fails closed unless the protected native-acceptance and signing inputs are present.

Read [Architecture](docs/architecture.md), [Troubleshooting](docs/troubleshooting.md), [Release and rollback](docs/release-and-rollback.md), and [Contributing](CONTRIBUTING.md) before changing platform integration or release claims.

License: [MIT](LICENSE).
