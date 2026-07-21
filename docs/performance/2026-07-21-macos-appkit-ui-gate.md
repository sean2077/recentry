# macOS AppKit UI gate — 2026-07-21

## Question

Can standard AppKit controls or a compact owner-drawn AppKit row satisfy Recentry's launcher contract within repeatable macOS memory and cold/warm activation limits?

## Prototype type

UI/interaction gate preflight. The intended switchable variants share the 40-item search/keyboard/focus flow and differ only in standard versus compact owner-drawn result rendering.

## Assumptions and discriminating condition

The gate must run natively on macOS 13 or later on Intel and Apple Silicon. Both variants must be measured over 30 cold and 30 warm activations with the same host, IPC, discovery fixture, focus-loss behavior, accessibility inspection, theme, and display placement. The owner-drawn variant predicts a denser one-line row; the standard variant predicts stronger default accessibility with potentially different layout and resource costs.

## Entry point / run command

```powershell
pwsh -File scratch/prototypes/macos-ui-gate/run.ps1 -Variant all
```

## Observations

The preflight ran on Windows 11 and stopped immediately because no native macOS system, AppKit runtime, macOS SDK, Intel Mac, or Apple-Silicon Mac is available. Cross-target Rust compilation cannot render AppKit, inspect accessibility, or measure process resources and activation.

## Verdict

No macOS UI budget or rendering variant can be selected from this environment.

## Production disposition

**Rework.** Run both AppKit variants and the common measurement harness on Intel and Apple Silicon, lock the macOS UI limits, then rebuild the winning behavior under production contracts.
