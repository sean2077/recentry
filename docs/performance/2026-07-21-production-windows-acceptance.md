# Production Windows resource acceptance — 2026-07-21

## Scope

The measured binaries are the production `0.1.0-beta.1` release build, using the installed stable VS Code 1.129.1 and its real application-shared recent-project database. The launcher loaded 17 folder/workspace entries and completed a text search before the active-tree memory sample.

The public `recentry.exe show` command was timed end to end: process creation, current-user host IPC, VS Code discovery/database read, UI command handling, synchronous Win32 paint, and the UI `Shown` response. Cold samples killed the UI child before every activation; warm samples reused the hidden child.

## Environment

- Windows 11 Pro for Workstations 10.0.26200, x64
- Intel Core i7-12700H, 20 logical processors
- Rust/Cargo 1.95.0
- `recentry.exe`: 348,672 bytes
- `recentry-ui.exe`: 1,588,736 bytes

## Results

| Measurement | Hard gate | Result | Verdict |
| --- | ---: | ---: | --- |
| Host Private Working Set after 60 s | <=10 MiB | 1.043 MiB | pass |
| Host idle CPU over the next 60 s | <=0.5% | 0.000% | pass |
| Active process-tree Private Working Set | <=60 MiB | 4.168 MiB | pass |
| Cold activation p95, 30 samples | <=600 ms | 174.852 ms | pass |
| Warm activation p95, 30 samples | <=150 ms | 88.725 ms | pass |

Idle CPU is reported as one-core utilization (`process CPU seconds / wall seconds`), which is stricter than normalizing across the machine's 20 logical processors. Private Working Set came from `Win32_PerfFormattedData_PerfProc_Process.WorkingSetPrivate` and was summed across the resident host and UI child.

Cold samples ranged from 83.870 to 186.613 ms. Warm samples ranged from 49.362 to 112.156 ms. The percentile uses the nearest-rank definition.

## Runtime checks performed with the same build

- Loaded 17 real VS Code recent folders/workspaces in VS Code order.
- Visually inspected the compact owner-drawn `[VSCode] project path` single-line row.
- Verified text filtering, queued Up/Down handling, Enter routing, Esc hide, and query reset on the next summon.
- Saved language/configuration through the host and reloaded the atomically replaced JSON.
- Enabled and disabled current-user autostart through Settings, verified the exact Run value, and confirmed cleanup.
- Reserved the configured global hotkey in another process; the host survived the conflict and the command/tray fallback still opened the UI.
- Force-terminated `recentry-ui.exe`; the host survived and the next `show` started a new UI process with all 17 entries.
- Verified a missing UI binary produces a safe host error and does not terminate the host.
- Verified the portable ZIP and silent per-user install, in-place upgrade, and uninstall, including registry/shortcut cleanup.

## Verdict

All fixed production resource gates pass. No budget was relaxed. The production architecture is accepted for packaging and Windows beta installation checks.

## Environment limits

- Windows 10 was not available locally, so the current manual OS result is Windows 11 x64 only.
- The GitHub Actions workflow is syntax-checked but cannot run until the new repository has a remote and its first push.
- The 15-minute hidden-child deadline is implemented and its hide/restart lifecycle was exercised, but this run did not wait the full 15 wall-clock minutes for automatic recycling.
- The installer is intentionally unsigned for this beta; `Get-AuthenticodeSignature` reports `NotSigned` as expected.
