# Changelog

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
