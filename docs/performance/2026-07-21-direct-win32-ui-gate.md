# Direct Win32 UI gate — 2026-07-21

## Question

Can a direct Win32 launcher, under the same lightweight host architecture, implement the compact `[VSCode] <project name>  <path>` row and satisfy the fixed 60 MiB active process-tree plus cold/warm latency gates?

## Prototype type

UI/interaction. One disposable release executable exposed two variants: a system-standard single-line list and a compact owner-drawn single-line list with a muted, ellipsized path and full-path hover text.

## Assumptions and discriminating condition

The experiment modelled the resident host, tray, `Ctrl+Alt+R` registration, named-pipe request/response, reusable UI child, 40 in-memory projects, filtering, keyboard selection, hide/reuse, UI crash recovery, and post-paint readiness acknowledgement. It intentionally omitted VS Code database access, real project launching, settings, localization, installation, and production hardening.

Both variants ran in the same release binaries and host. Each received 30 process-cold activations and 30 warm hide/show activations. Memory is Windows Private Working Set summed over the complete two-process tree after the list was loaded and filtered. The host stabilized for 60 seconds before its memory sample, followed by a 60-second idle CPU sample.

## Entry point / run command

The disposable experiment was run with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scratch/ui-gate/measure.ps1 -Iterations 30 -StabilizeSeconds 60 -IdleSeconds 60 -SkipBuild
```

Environment: Windows 11 Pro for Workstations 10.0.26200, Intel Core i7-12700H, 20 logical processors, Rust 1.95.0.

## Observations

| Measurement | Gate | Standard list | Compact owner-drawn list |
| --- | ---: | ---: | ---: |
| Host Private Working Set | <=10 MiB | 0.965 MiB | 0.965 MiB |
| Host idle CPU | <=0.5% | 0.000% | 0.000% |
| Process-tree Private Working Set maximum | <=60 MiB | 3.895 MiB | 4.902 MiB |
| Cold activation p95 | <=600 ms | 260.599 ms | 110.748 ms |
| Warm activation p95 | <=150 ms | 81.706 ms | 64.517 ms |

The release binaries were 232,960 bytes for the host and 162,304 bytes for the UI. All four sample groups contained exactly 30 observations. A rendered-window inspection confirmed that the compact variant keeps `[VSCode]`, project name, and path on one row without a secondary line.

The cold measurements are process-cold, not machine-cache-cold. The variants ran sequentially, so the second variant may benefit from operating-system code-page caching. This does not affect the gate decision: both variants pass every limit with large memory and latency margins.

## Verdict

Direct Win32 passes the fixed contract. The compact owner-drawn list also satisfies the required single-line information layout. Confidence is high for feasibility on the measured Windows target; final production binaries must still repeat the same acceptance test after VS Code discovery, configuration, localization, and opening behavior are integrated.

## Production disposition

**Hand off.** Rebuild the compact direct Win32 behavior under production contracts. The disposable source, binaries, screenshot, and raw outputs are not production inputs and are discarded after this report. Retain this measurement report and [ADR 0002](../adr/0002-direct-win32-ui.md).
