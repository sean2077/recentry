# Linux native UI gate — 2026-07-21

## Question

Can a GTK4 launcher satisfy the compact Recentry interaction contract on GNOME and KDE Wayland while a direct X11 variant establishes the low-resource baseline for an X11 session?

## Prototype type

UI/interaction gate preflight. The intended variants are GTK4 and direct X11, both using the same 40-item keyboard flow and measurement harness.

## Assumptions and discriminating condition

The meaningful run requires native GNOME Wayland, KDE Plasma Wayland, and X11 sessions. Each variant must support filtering, Up/Down, Enter, Esc, focus-loss dismissal, one-line rows, theme/language behavior, and 30 cold plus 30 warm activations. GTK4 predicts broader Wayland integration with a higher process cost; direct X11 predicts a smaller X11 process but cannot satisfy Wayland by itself.

The current Windows host cannot exercise either graphical stack. WSL compilation would not replace the required native desktop observation, but it would permit an earlier dependency check if the registered distribution could start.

## Entry point / run command

```powershell
pwsh -File scratch/prototypes/linux-ui-gate/run.ps1 -Variant all
```

## Observations

The preflight ran on Windows 11. The registered `Ubuntu-24.04` WSL2 distribution could not create its VM and returned `HCS_E_HYPERV_NOT_INSTALLED`. There was no `WAYLAND_DISPLAY`, `DISPLAY`, GNOME, KDE, X11, or ARM64 graphical target on which to build and run both variants. No UI memory or activation observations were produced.

## Verdict

The toolkit question remains unanswered. Cross-compilation cannot distinguish desktop behavior or resource use, so selecting GTK4 or promoting either candidate would be unsupported.

## Production disposition

**Rework.** Run the switchable 40-item candidates and the common measurement harness on the required native Linux matrix, then lock the Linux UI memory and cold/warm p95 budgets before writing production UI code.
