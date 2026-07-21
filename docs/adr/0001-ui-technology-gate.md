# ADR 0001: Windows UI technology gate requires rework

- **Status:** Rework
- **Measured:** 2026-07-21
- **Decision owner:** Recentry v1 Windows-first plan

## Question

Can either Tauri 2 or egui/eframe provide the same 40-item keyboard launcher through the same complete Win32 host while meeting all four release gates: background memory at or below 10 MiB, active process-tree memory at or below 60 MiB, warm activation p95 at or below 150 ms, and cold activation p95 at or below 600 ms?

## Prototype type

UI/interaction comparison with a common disposable host.

## Assumptions and discriminating condition

Both variants used release builds, identical in-memory project data, the same search and keyboard flow, and the same request-response named-pipe lifecycle. The host included a named mutex, `RegisterHotKey`, `Shell_NotifyIcon`, named-pipe child supervision, and explicit hide/show/quit commands. Each cold run kept the window alive for one second so descendant-process memory had time to stabilize.

The comparison intentionally excluded VS Code discovery and production settings because neither changes the UI-runtime baseline. Tauri exercised WebView2's process model; egui exercised a native `winit`/OpenGL process. Measurements used Windows Private Working Set for the complete host process tree.

Environment:

- Windows 11 Pro for Workstations 10.0.26200, x64
- Intel Core i7-12700H, 20 logical processors
- Rust 1.95.0
- WebView2 138.0.3351.83
- 30 cold and 30 warm activations per variant

## Entry point / run command

The disposable prototype was run with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scratch/ui-gate/measure.ps1 -Iterations 30 -IdleSeconds 60 -SkipBuild
```

The prototype source and build output were discarded after this report, as required by the experiment boundary.

## Observations

| Measurement | Gate | Win32 host | Tauri 2 | egui/eframe |
| --- | ---: | ---: | ---: | ---: |
| Background Private Working Set | <=10 MiB | 0.996 MiB | n/a | n/a |
| Background CPU | <=0.5% | 0.000% | n/a | n/a |
| Cold process-tree Private Working Set | <=60 MiB | n/a | 111.500 MiB | 72.363 MiB |
| Warm process-tree Private Working Set | <=60 MiB | n/a | 118.355 MiB | 72.695 MiB |
| Cold activation p95 | <=600 ms | n/a | 929.696 ms | 197.605 ms |
| Warm activation p95 | <=150 ms | n/a | 0.225 ms* | 57.590 ms |

Release binary sizes were 290,304 bytes for the host, 3,215,360 bytes for the Tauri UI, and 5,340,672 bytes for the egui UI.

\* Tauri's warm acknowledgement was emitted when the window API accepted `show` and focus, not after a painted frame, so this number is optimistically biased. Tauri already fails both active-memory and cold-latency gates, so the limitation does not affect the rejection.

The synchronous named-pipe spike also demonstrated that cloning one handle and blocking reads/writes on separate threads can deadlock. The validated prototype protocol used one request-response I/O owner. Production IPC must preserve that invariant or use overlapped I/O.

## Verdict

Neither candidate satisfies the fixed release contract:

- Tauri fails active memory and cold activation latency.
- egui passes both latency gates but exceeds the active-memory ceiling by 12.695 MiB at its measured maximum.
- The split-process Win32 host comfortably passes its background memory and CPU gates, so the failed boundary is the UI runtime rather than the resident architecture.

Confidence is high for the rejection because both memory failures are well above the hard ceiling and were observed across the full 30-sample run.

## Production disposition

**Rework.** Do not hand either prototype into production and do not begin the remaining implementation slices.

The next discriminating question compared a direct Win32 popup against the same 40-item flow and measurement harness. Its result contract was one compact line per item: `[VSCode] <project name>  <path>`, with no secondary row; the path is visually muted, ellipsized when needed, and shown in full on hover.

That rework passed every unchanged gate. [ADR 0002](0002-direct-win32-ui.md) records the production choice and the retained [measurement report](../performance/2026-07-21-direct-win32-ui-gate.md).
