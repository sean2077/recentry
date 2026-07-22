# Changelog

## Unreleased

## 0.1.0-beta.4 — 2026-07-22

- Expand the Windows launcher from eight to twelve visible recent-project rows.
- Reduce result-row height from 28 to 24 logical pixels so more entries fit while preserving the compact single-line layout.

## 0.1.0-beta.3 — 2026-07-21

- Publish the second public Windows x64 beta with the NSIS installer, portable ZIP, and SHA-256 checksums.
- Retry removal of the Start Menu shortcut during uninstall when Windows temporarily holds the link, and report a visible failure if retries are exhausted.
- Include the shared-runtime, cross-platform foundation, CI, and documentation work prepared in the tagged but unpublished `v0.1.0-beta.2` snapshot.

## 0.1.0-beta.2 — 2026-07-21 (not published)

- Tag the candidate but stop publication when repeated final package smoke exposed intermittent Start Menu shortcut residue after uninstall.
- Add platform-aware stable VS Code discovery and path identity for Windows, Linux, and macOS fixtures.
- Add owner-only Unix-domain-socket IPC with stale-endpoint recovery, bounded framing, and peer-user verification.
- Extract shared host command/configuration behavior and cross-platform UI-child supervision while preserving Win32 event-thread shortcut registration.
- Add a Unix development host and XDG autostart support; native Linux shortcuts/status UI and macOS AppKit integration remain blocked by native gates.
- Add fail-closed Linux DEB/AppImage, macOS Universal 2 app ZIP/DMG, Authenticode, notarization, checksum, and native smoke command contracts for unverified development packaging.
- Expand native CI and add English platform-status, architecture, troubleshooting, security, contribution, release, and rollback documentation.
- Keep Linux and macOS public support explicitly `in development`; this Windows-only preview does not satisfy the cross-platform release contract.

## 0.1.0-beta.1 — 2026-07-21

- Publish the first Windows x64 beta.
- Open a compact VS Code recent-project launcher from a global hotkey or tray menu.
- Support local folders, workspaces, and remote projects across current application-shared and legacy VS Code recent-data locations.
- Support search, complete keyboard navigation, focus-or-new-window opening, English and Simplified Chinese localization, system themes, and configurable hotkeys.
- Use a rofi-inspired single-layer interface with `[VSCode] project-name path` results and automatic dismissal on focus loss.
- Open the launcher when the executable is run directly while keeping sign-in startup unobtrusive.
- Keep the host lightweight and isolate the on-demand UI process, including crash recovery and 15-minute hidden-process recycling.
- Provide a per-user NSIS installer, portable ZIP, and SHA-256 checksum file.
- Perform no telemetry, cloud synchronization, or background network requests.
