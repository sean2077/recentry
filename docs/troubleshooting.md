# Troubleshooting

## The shortcut does not open Recentry

Another application may own `Ctrl+Alt+R`. Open Recentry from the Windows tray or run `recentry show`, then choose another shortcut in Settings. A conflict is diagnostic; it does not stop the host.

## No projects appear

Recentry supports stable VS Code only. Open a folder or `.code-workspace` in stable VS Code first, then summon Recentry again. If VS Code was installed in a nonstandard location, set its executable in Settings. Diagnostics report provider error codes without exposing raw project paths.

Recentry excludes ordinary recent files and unsupported URI schemes by design. A locked, corrupt, missing, or incompatible VS Code database fails safely and is never modified.

## The launcher stays open after clicking elsewhere

The Windows launcher should hide after focus moves away. If it does not, include the Windows version, display layout/scaling, reproduction steps, and the redacted Diagnostics output in an issue. Do not attach `state.vscdb`, `storage.json`, or project paths.

## `recentry-ui` is missing

The host and UI must stay together in a portable directory. Re-extract the complete ZIP or reinstall. The host remains alive after a missing/crashed UI and tries a fresh child on the next visible command.

## Reset Windows configuration

Quit Recentry, then remove `%APPDATA%\Recentry\config.json`. Uninstall intentionally retains this file so settings can survive reinstall. Removing the whole `%APPDATA%\Recentry` directory performs a complete settings reset.

## Linux or macOS development artifacts do not provide a complete launcher

Those artifacts are explicitly unverified engineering builds. Linux's production UI toolkit and macOS AppKit integration have not passed their native gates. Use the current Windows beta or contribute native evidence; do not report an unverified artifact as a supported release regression.

## Reporting safely

Use GitHub issues for ordinary defects. Hash or replace usernames and project paths. For vulnerabilities, follow [Security policy](../SECURITY.md) and avoid a public issue until coordinated disclosure is complete.
