# Recentry v1 Windows-first implementation

> Historical record for the `v0.1.0-beta.1` Windows beta. The active plan is [Recentry Cross-Platform v1](cross-platform-implementation-plan.md); this file does not describe current Linux or macOS support.

Goal: deliver a Windows x64 beta of Recentry that opens or focuses VS Code recent projects while meeting the agreed process-tree memory, CPU, and activation-latency budgets.

### Slice `gate` — choose a UI technology from measured Windows evidence

- **Status:** `done`
- **Blocked by:** none
- **Touches:** disposable release-mode host and equivalent Tauri 2 / egui launchers plus a direct Win32 rework outside tracked production source; retained measurement reports and ADRs only
- **Test seam:** each launcher exposes the same 40-item searchable keyboard flow and reports readiness to the same Win32 host over a named pipe; the Win32 rework compared a standard native list with a compact owner-drawn native list while preserving the single-line row contract
- **Verification:** run 30 cold and 30 warm activations and record Private Working Set for the host and complete process tree plus idle CPU
- **Result/Evidence:** Tauri and egui failed the fixed active-memory gate. The direct Win32 compact rework passed: host 0.965 MiB and 0% idle CPU; active tree 4.902 MiB; cold/warm p95 110.748/64.517 ms. See [ADR 0001](adr/0001-ui-technology-gate.md), [ADR 0002](adr/0002-direct-win32-ui.md), and the [measurement report](performance/2026-07-21-direct-win32-ui-gate.md).

### Slice `core` — discover, rank, and safely open VS Code recent projects

- **Status:** `done`
- **Blocked by:** `gate`
- **Touches:** platform-neutral domain model; VS Code product/database/window-state discovery; fuzzy ranking; open-or-focus command construction; versioned configuration and diagnostics
- **Test seam:** fixture SQLite/JSON databases and a fake `code.exe` capture exact arguments
- **Verification:** `cargo test -p recentry-core`
- **Result/Evidence:** `cargo test -p recentry-core -p recentry-protocol --locked` passed 12 tests. `cargo fmt --all --check` and strict Clippy passed. Fixtures cover current/legacy recent shapes, versioned Windows installs, ordinary-file exclusion, URI/path deduplication, search order, corrupt SQLite, window state, safe argument generation, fake `code.exe` startup, and launch failure.

### Slice `host` — summon one reusable UI from a low-memory Windows resident

- **Status:** `done`
- **Blocked by:** `gate`
- **Touches:** public `recentry.exe`; Win32 mutex, named pipe, global hotkey, tray, child supervision, idle shutdown protocol, and per-user autostart adapter
- **Test seam:** protocol integration tests plus a fake UI child exercise single-instance, summon, crash recovery, and shutdown behavior
- **Verification:** `cargo test -p recentry-host` and release-mode host resource probe
- **Result/Evidence:** IPC completed RED→GREEN with current-owner ACL, local-only pipes, 1 MiB framing limit, and a documented single-I/O-owner invariant. Host tests cover atomic config preservation and hotkey conversion. Release smokes verified single-instance behavior, missing-UI safe failure, command forwarding, clean quit, explicit autostart enable/disable, hotkey-conflict fallback, and UI crash recovery. Final 60-second host measurements are recorded in `acceptance`.

### Slice `ui` — provide the compact bilingual keyboard launcher

- **Status:** `done`
- **Blocked by:** `core`, `host`
- **Touches:** direct Win32 UI child; centered active-monitor window; search/list state; compact single-line result rows; keyboard flow; settings; diagnostics; system theme and zh-CN/en localization
- **Test seam:** every result uses `[VSCode] <project name>  <path>` on one line with no secondary row; the path is visually muted, ellipsized when needed, and exposed in full on hover; state-machine tests and a real UI smoke run cover the installed VS Code data
- **Verification:** workspace tests plus scripted summon/search/open smoke test
- **Result/Evidence:** direct Win32 production UI is complete. Strict Clippy and all workspace tests pass. A release smoke loaded 17 projects from the installed stable VS Code, confirmed the compact one-line row visually, and passed filtering, queued keyboard navigation, Esc hide, query reset, bilingual settings save, and UI crash/restart checks.

### Slice `package` — produce an installable, diagnosable Windows beta

- **Status:** `done`
- **Blocked by:** `ui`
- **Touches:** MIT/readme/license; Windows x64 NSIS and portable ZIP packaging; SHA-256 generation; Windows CI and platform-neutral compile checks
- **Test seam:** clean-directory portable launch and silent install/uninstall checks
- **Verification:** release build, package smoke checks, and CI-equivalent commands
- **Result/Evidence:** MIT license, bilingual user README, changelog, per-user NSIS definition, portable ZIP packaging, SHA-256 generation, fail-closed overwrite behavior, and Windows/Linux/macOS CI jobs are present. The final artifacts passed portable launch plus silent install, same-directory upgrade, and uninstall cleanup on Windows 11 x64.

### Slice `acceptance` — prove the beta meets every hard gate

- **Status:** `done`
- **Blocked by:** `package`
- **Touches:** resource harness, compatibility fixtures, full self-review, and retained acceptance report
- **Test seam:** actual Windows release binaries and stable VS Code installation
- **Verification:** all tests/lints, 30 cold + 30 warm activations, 60-second background memory/CPU sample, active process-tree memory sample, installation and open-or-focus manual smoke
- **Result/Evidence:** all production resource gates pass on the final binaries: host 1.043 MiB Private Working Set after 60 seconds; 0% idle CPU over 60 seconds; active host+UI tree 4.168 MiB; 30-sample cold/warm p95 174.852/88.725 ms. Portable launch, silent install, in-place upgrade, uninstall cleanup, autostart toggling, hotkey conflict fallback, and UI crash recovery also pass. See the [production acceptance report](performance/2026-07-21-production-windows-acceptance.md).
