# Architecture

Recentry separates the resident platform host from the on-demand launcher UI so global integration stays small while project discovery and rendering are loaded only when needed.

## Components

- `recentry`: single-instance host, local IPC endpoint, configuration, shortcut/status integration, autostart, command routing, and UI-child supervision.
- `recentry-ui`: on-demand native launcher. It discovers stable VS Code data read-only, owns search/rendering, and exits after remaining hidden for 15 minutes.
- `recentry-core`: project model, provider/opener contracts, VS Code layouts, parsing, ranking, deduplication, and safe launch construction.
- `recentry-protocol`: versioned configuration plus host/UI messages.
- `recentry-ipc`: size-bounded current-user named pipes on Windows and owner-only Unix-domain sockets on Linux/macOS.

`HostRuntime` owns command/configuration semantics behind the small `HostAdapter` boundary. Win32 is the verified adapter. Unix can compile and exercise the shared lifecycle and transport, and Linux has an XDG autostart writer. The development adapters intentionally report missing native global-shortcut/status integration and missing macOS login-item integration instead of pretending support.

`VsCodePlatformLayout` supplies executable, product metadata, database, and window-state candidates. `TargetIdentityPolicy` keeps Windows path identity case-insensitive and POSIX identity case-sensitive. Providers and openers use a compile-time registry; there is no plugin ABI in this release.

## Lifecycle

1. A later invocation forwards `show`, `settings`, `diagnostics`, or `quit` to the current user's host endpoint.
2. The host starts `recentry-ui` on first UI demand and grants activation through the platform adapter.
3. The UI reads VS Code state, clears the query, presents the compact launcher, and acknowledges readiness.
4. Esc, focus loss, or successful opening hides the UI. A hidden child is reused for warm activation and exits after 15 minutes.
5. A dead/broken UI is discarded and restarted once for visible commands; it never terminates the host.
6. Host shutdown asks the UI to quit, waits briefly, then terminates a child that does not exit.

## Security boundaries

- IPC endpoint identifiers are validated; messages are framed and capped at 1 MiB.
- Unix runtime directories and sockets must be owned by the effective user and use owner-only permissions.
- VS Code databases are opened read-only and are never repaired or modified.
- Project targets are passed as process arguments, not through a shell.
- Configuration is validated and atomically replaced. Autostart updates roll back when persistence fails.
- Packaging has separate development and release modes. Release mode requires protected acceptance/signing evidence.
