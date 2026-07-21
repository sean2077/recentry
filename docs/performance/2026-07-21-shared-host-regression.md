# Shared host extraction Windows regression — 2026-07-21

## Scope

This local check measured the release binaries after extracting `HostRuntime`, moving UI supervision onto the platform-neutral local transport interface, retaining the Win32 adapter, and routing configuration-triggered hotkey registration back to the hidden-window thread. It is regression evidence for the implementation branch, not cross-platform release acceptance.

Environment: Windows 11 Pro for Workstations 10.0.26200 x64, Intel Core i7-12700H, Rust 1.86.0. The test used an isolated configuration root with autostart disabled and a non-default test shortcut.

## Results

| Measurement | Fixed Windows gate | Result | Verdict |
| --- | ---: | ---: | --- |
| Host Private Working Set after 60 seconds | <=10 MiB | 0.746 MiB | pass |
| Host idle CPU over the next 60 seconds | <=0.5% | 0.000% | pass |
| Active host/UI Private Working Set | <=60 MiB | 2.785 MiB | pass |
| Cold activation p95, 30 samples | <=600 ms | 160.932 ms | pass |
| Warm activation p95, 30 samples | <=150 ms | 93.065 ms | pass |

Private Working Set came from `Win32_PerfFormattedData_PerfProc_Process`. Idle CPU is process CPU seconds divided by 60 wall-clock seconds, expressed as one-core utilization. The percentile uses nearest rank. Every cold sample terminated only the exact release `recentry-ui.exe`; the next public `recentry show` call had to recover the child and acknowledge the painted window. Warm samples reused the same child and sent Esc between summons.

## Additional evidence

- Thirteen `recentry-host` tests pass, including command routing, configuration rollback, main-thread hotkey reconfiguration, hotkey conflict behavior, UI endpoint generation, restart policy, XDG autostart rendering, and atomic configuration behavior.
- Four Unix target test binaries compile for Linux x86_64/ARM64 and macOS Intel/Apple Silicon.
- The final Rust 1.86 release binaries were repackaged locally with portable NSIS 3.12. The portable ZIP and NSIS installer passed launch, silent install, same-directory upgrade, clean shutdown, and uninstall cleanup. A new remote CI result is still required after an authorized commit/push.

## Verdict

The shared extraction preserves the fixed Windows resource, UI-recovery, and package-lifecycle gates on the measured Windows 11 environment. Remote CI and Windows 10 acceptance remain external gates.
