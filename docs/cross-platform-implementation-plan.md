# Recentry Cross-Platform v1 Implementation Plan

This plan implements the approved [cross-platform v1 specification](cross-platform-v1-spec.md) while preserving the released Windows beta and keeping every unverified platform claim explicitly `in development`.

## Baseline

- Source baseline: `83ee188` (`v0.1.0-beta.1`).
- Windows workspace baseline: formatting, strict Clippy, and all 22 tests pass.
- Existing production implementation: Windows x64 host, direct Win32 UI, named-pipe IPC, NSIS installer, and portable ZIP.
- Existing non-Windows implementation: portable core/protocol tests only; host, UI, IPC, and packaging are stubs or absent.
- Available local environment: Windows x64. Ubuntu 24.04 is registered under WSL2 but cannot start because firmware virtualization is disabled. No macOS, GNOME, KDE, X11 desktop, Linux ARM64, signing credentials, or notarization credentials are available locally.

## Delivery rule

A slice is `done` only after its stated verifier passes. Source that can compile but cannot be exercised on a required real environment remains `blocked`, and public positioning remains `in development`.

### Slice `S1` — VS Code projects behave correctly on all filesystem models

- **Status:** `done`
- **Blocked by:** none
- **Touches:** `recentry-core` platform layout module, VS Code discovery, target identity, portable fixtures, fake opener tests.
- **Test seam:** the `recentry-core` interface discovers installations/databases and produces stable projects/launch requests from Windows, Linux, and macOS fixtures.
- **Verification:** `cargo test -p recentry-core --locked` on Windows; the same command in Ubuntu 24.04 WSL; strict Clippy and formatting.
- **Result/Evidence:** RED first failed on the absent `VsCodePlatformLayout` and `TargetIdentityPolicy` interfaces. GREEN: 12 core contract tests pass on Windows, including Linux/macOS installation and storage fixtures plus case-sensitive POSIX identity; all 35 current workspace tests and strict Clippy pass. `cargo zigbuild` builds `recentry-core` for x86_64/aarch64 Linux and Intel/Apple-Silicon macOS. Native Linux execution remains unavailable because WSL cannot start.

### Slice `S2` — Current-user IPC works on Windows and Unix without protocol drift

- **Status:** `done` for implementation; native Unix acceptance is carried by `S8`
- **Blocked by:** `S1`
- **Touches:** `recentry-ipc` endpoint interface, Windows named-pipe adapter, Unix-domain-socket adapter, owner-only permissions, cleanup and framing tests.
- **Test seam:** identical protocol round trips, oversize rejection, timeout behavior, peer scope, and stale-endpoint recovery through the transport interface.
- **Verification:** workspace tests on Windows, strict Clippy, and compile the Unix test binaries for all supported Linux/macOS architectures. Execute those same tests on native Unix as a mandatory `S8` release gate.
- **Result/Evidence:** RED first failed on the absent platform endpoint and local transport interfaces. GREEN on Windows: four IPC tests pass, including endpoint validation, framing, and oversize rejection. The Unix-domain-socket implementation and its owner-only permission, cleanup, stale-recovery, active-endpoint, round-trip, and oversize tests compile for x86_64/aarch64 Linux and Intel/Apple-Silicon macOS with `cargo zigbuild`. They have not run natively because WSL and macOS are unavailable, so this evidence permits dependent implementation but does not permit a Unix support or release claim.

### Slice `S3` — One shared host lifecycle preserves Windows behavior and admits native adapters

- **Status:** `done` locally; fresh remote CI is carried by `S8`
- **Blocked by:** `S2`
- **Touches:** shared host runtime, command routing, configuration, UI supervision, single-instance interface, child-process interface, platform adapters, existing Win32 entry point.
- **Test seam:** an in-memory host adapter drives show/settings/diagnostics/quit, hotkey conflict, UI crash/restart, configuration update, and shutdown; the Windows package smoke remains green.
- **Verification:** `cargo test -p recentry-host --locked`, workspace tests, Windows release package smoke, and host resource regression measurement.
- **Result/Evidence:** RED first failed on the absent `HostRuntime`/`HostAdapter` seam. GREEN: the shared runtime owns command routing, configuration rollback, diagnostics, shortcut conflict handling, and UI requests; the UI coordinator now uses the platform-neutral local transport and restart policy. Thirteen host tests pass, including a regression contract that defers configuration-triggered hotkey registration to the platform thread. The Win32 adapter posts that work back to the hidden-window thread instead of calling `RegisterHotKey` from the IPC worker. Unix development hosts compile for all four Linux/macOS targets and use the shared runtime, Unix socket, UI supervision, and Linux XDG autostart; their native shortcut/status/AppKit services remain platform-slice work. The final Windows 11 release regression passes at 0.746 MiB host Private Working Set, 0% idle CPU, 2.785 MiB active tree, 160.932 ms cold p95, and 93.065 ms warm p95 over 30 samples. Cold samples killed the UI and therefore exercised restart. The final Rust 1.86 binaries were rebuilt into a portable ZIP and NSIS installer with NSIS 3.12; portable launch, silent install, same-directory upgrade, and uninstall cleanup pass locally. Existing baseline CI does not count as remote verification of these changes.

### Slice `S4` — Linux native integration reaches a measurable prototype gate

- **Status:** `blocked` at the native prototype gate
- **Blocked by:** `S3`
- **Touches:** disposable Linux UI candidates under `scratch/prototypes/`, XDG GlobalShortcuts portal adapter, X11 shortcut fallback, StatusNotifierItem integration, XDG autostart, native launcher adapter, accessibility and focus-loss behavior.
- **Test seam:** one command launches switchable prototype variants with the 40-item keyboard flow; production code is rebuilt separately only after a candidate passes.
- **Verification:** compile and unit tests in Ubuntu WSL; actual 30-cold/30-warm, active-memory, idle-host, GNOME Wayland, KDE Wayland, X11, and ARM64 observations on required real environments.
- **Result/Evidence:** The isolated gate preflight was run for the GTK4/direct-X11 candidate question. `Ubuntu-24.04` could not start and returned `HCS_E_HYPERV_NOT_INSTALLED`; there is no GNOME, KDE, X11, or ARM64 graphical target. The recorded production disposition is `rework`. No Linux toolkit or UI budget has been selected, so production UI code and support claims remain blocked. Platform-neutral Linux core/IPC/host code and XDG autostart may continue independently.

### Slice `S5` — macOS native integration implements the AppKit contract

- **Status:** `blocked` pending an AppKit prototype and native Macs
- **Blocked by:** `S3`
- **Touches:** AppKit launcher/menu-bar adapters, native global shortcut, single-instance integration, Unix IPC, SMAppService login item, UI-child activation and lifecycle, Universal 2 compatibility.
- **Test seam:** shared host/UI state tests plus macOS-native integration and UI automation on Intel and Apple Silicon.
- **Verification:** cross-target type checking where possible; native `cargo test`, 30-cold/30-warm measurements, multi-display/focus tests, and lifecycle tests on both macOS architectures.
- **Result/Evidence:** The standard-control versus compact-owner-drawn AppKit preflight ran and stopped because no native macOS system, AppKit runtime, SDK, Intel Mac, or Apple-Silicon Mac is available. Its production disposition is `rework`; no macOS UI budget or production rendering variant is selected. Core/IPC/shared-host test binaries compile for both macOS architectures, and Universal 2 development packaging is implemented but unexecuted. This is not native UI evidence.

### Slice `S6` — Every platform has fail-closed installer and portable packaging jobs

- **Status:** `in_progress`; commands/workflow implemented, native package runs pending
- **Blocked by:** `S3`; Linux production packaging additionally blocked by `S4`; macOS production packaging additionally blocked by `S5`.
- **Touches:** authoritative project-owned packaging commands, Linux DEB/AppImage metadata, macOS app bundle/DMG/ZIP metadata, Windows signing hook, checksum manifest, OpenPGP signature hook, CI artifact matrix, native smoke tests.
- **Test seam:** each packaging command produces an isolated bundle, refuses unsafe overwrite, verifies expected files/metadata, and supports unsigned development artifacts without promoting them as releases.
- **Verification:** package smoke on matching native runners; signature/notarization checks for promoted releases; SHA-256 coverage audit for every artifact.
- **Result/Evidence:** ADR 0003 records the job boundaries and command contracts. Linux DEB/AppImage, macOS Universal 2 app ZIP/DMG, native smoke, complete-manifest/OpenPGP, and Windows Authenticode hooks are implemented. A manual workflow labels all outputs `unverified-development` and refuses atomic manifest completion when any mandatory asset is absent. Shell syntax/help, PowerShell parsing, fail-closed release arguments, workflow semantics, metadata parsing, and overwrite contracts are checked locally. Portable NSIS 3.12 rebuilt the final Windows ZIP/installer, and their lifecycle smoke passes. Linux/macOS commands have not run on their native systems. No package is release evidence yet.

### Slice `S7` — Public documentation and repository metadata reflect evidence exactly

- **Status:** `done`; source publication is included in the authorized main-branch delivery
- **Blocked by:** `S1`, `S2`, `S3`; release-support wording remains blocked by `S4`, `S5`, `S6`, signing credentials, and the real acceptance matrix.
- **Touches:** English README, architecture and platform docs, installation/status matrix, troubleshooting, GitHub description/topics, CI badges, release guidance, rollback runbook.
- **Test seam:** documentation links and commands resolve; every support claim maps to recorded evidence; no unsupported artifact is presented as available.
- **Verification:** repository text audit for stale Windows-only product positioning and premature cross-platform support claims; GitHub metadata inspection.
- **Result/Evidence:** The English README now positions Recentry as cross-platform work in development while retaining the exact historical Windows beta installation path. Platform status, architecture, troubleshooting, contributing, security, package governance, release/rollback, and native-gate evidence are documented. Relative link checks pass and the stale non-Windows binary message was removed. The live GitHub description, README homepage, and topics were updated and independently verified on the repository page; this authorized main-branch delivery publishes the source documentation.

### Slice `S8` — Complete verification and delivery decision

- **Status:** `blocked` after local source verification; native acceptance and signing remain pending
- **Blocked by:** `S1`–`S7`
- **Touches:** no new behavior; full self-review, CI/native evidence collection, resource reports, release-readiness and rollback rehearsal.
- **Test seam:** the approved specification checklist is mapped one-to-one to passing evidence or a named blocker.
- **Verification:** formatting, strict Clippy, workspace tests on all targets, package smoke, native UI automation, resource gates, signatures, checksums, installation/upgrade/removal, and documentation audit.
- **Result/Evidence:** Local source verification is green on the fixed Rust 1.86.0 toolchain: formatting, strict workspace/all-target Clippy, and all 35 Windows-runnable tests pass. Workspace test binaries and optimized release binaries compile for Linux x86_64/ARM64 and macOS Intel/Apple Silicon. Packaging scripts pass local syntax, metadata, overwrite, and fail-closed release-mode checks. The final Windows 11 resource run and local installer/portable lifecycle smoke pass. Native Unix tests, Linux/macOS UI acceptance, native Linux/macOS package smoke, Windows 10 acceptance, signatures, notarization, and rollback rehearsal are still unavailable; remote CI must pass on the resulting commit. A cross-platform release is forbidden until every mandatory gate is green.

## Known external gates

- User-selected Linux distribution/version support baseline.
- Real GNOME Wayland, KDE Plasma Wayland, X11, and Linux ARM64 environments.
- Intel and Apple Silicon macOS 13+ environments.
- Windows Authenticode identity, Apple Developer ID/notarization credentials, and an OpenPGP release key.
- User authorization for any tag or release; this delivery authorizes only the main-branch commit, push, and resulting CI.
