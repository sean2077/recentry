# ADR 0002: Use a direct Win32 popup UI for the Windows beta

- **Status:** Accepted
- **Date:** 2026-07-21
- **Supersedes:** the unresolved UI choice in [ADR 0001](0001-ui-technology-gate.md)

## Context

The fixed technology gate rejected Tauri 2 and egui/eframe because their complete active process trees exceeded 60 MiB. The lightweight Win32 host itself remained far below its background budget, so a second disposable experiment tested whether a direct Win32 child could preserve the split-process lifecycle without importing a general UI runtime.

The required result row is one compact line: `[VSCode] <project name>  <path>`. The project name remains primary, the path is visually muted and ellipsized when space is insufficient, and hover exposes the full path.

## Decision

Recentry `0.1.0-beta.1` will use a direct Win32 UI child with native controls and an owner-drawn result list. The host remains a separate direct Win32 executable and does not link the UI, SQLite, or search implementation.

The chosen production behavior is rebuilt separately from the disposable experiment. Win32-specific rendering and lifecycle code stays behind platform-facing modules so the core model, provider contracts, ranking, protocol, and tests remain portable.

## Evidence

The compact variant passed 30 process-cold and 30 warm activations on Windows 11 x64:

- host Private Working Set: 0.965 MiB;
- host idle CPU: 0.000%;
- active process-tree Private Working Set maximum: 4.902 MiB;
- cold activation p95: 110.748 ms;
- warm activation p95: 64.517 ms.

See the complete [direct Win32 measurement report](../performance/2026-07-21-direct-win32-ui-gate.md).

## Consequences

- The measured architecture has enough headroom for read-only SQLite discovery and production state while retaining the unchanged hard gates.
- Recentry accepts more Windows-specific code and unsafe FFI than either rejected framework would require.
- UI behavior needs focused state tests plus Windows UI smoke tests; portable logic must not leak into window procedures.
- Tauri and egui remain rejected for this beta. They are not fallbacks if the final production measurement fails; a failure returns the architecture to rework.
