# Platform support

Recentry uses an evidence-gated support model. A platform is supported only when native CI, real graphical acceptance, resource limits, package lifecycle tests, and publisher trust all pass. Cross-compilation is useful engineering evidence but is never a support claim.

## Current matrix

| Target | Core and IPC | Native host/UI | Packages | Trust | Real GUI acceptance | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Windows 10 x64 | Implemented | Win32 implementation | NSIS/ZIP implemented | Historical beta unsigned | Not available locally | Beta scope not fully revalidated |
| Windows 11 x64 | Implemented | Win32 implementation | NSIS/ZIP implemented | Historical beta unsigned | Recorded for `v0.1.0-beta.1`; extraction regression measured | Historical beta available |
| Linux x86_64 | Core, Unix IPC, and development host compile | Production UI toolkit not selected | DEB/AppImage development commands | Release OpenPGP gate prepared | GNOME/KDE/X11 missing | In development |
| Linux ARM64 | Core, Unix IPC, and development host compile | Production UI toolkit not selected | DEB/AppImage development commands | Release OpenPGP gate prepared | Real ARM64 Wayland missing | In development |
| macOS Intel | Core, Unix IPC, and development host compile | AppKit production integration pending | Universal 2 development command | Developer ID/notarization inputs unavailable | Real Intel Mac missing | In development |
| macOS Apple Silicon | Core, Unix IPC, and development host compile | AppKit production integration pending | Universal 2 development command | Developer ID/notarization inputs unavailable | Real Apple-Silicon Mac missing | In development |

The blocked prototype gates are recorded in [Linux native UI gate](performance/2026-07-21-linux-native-ui-gate.md) and [macOS AppKit UI gate](performance/2026-07-21-macos-appkit-ui-gate.md). Both dispositions are `rework`, not production decisions.

## Required release evidence

- Windows 10 and 11 x64 installation, upgrade, removal, shortcut, tray, focus, multi-display, and resource acceptance.
- GNOME Wayland, KDE Plasma Wayland, one X11 session, and one real ARM64 Wayland environment.
- macOS 13 or later on both Intel and Apple Silicon.
- Host resident/private memory at or below 10 MiB and idle CPU at or below 0.5% everywhere.
- Windows UI tree at or below 60 MiB, cold p95 at or below 600 ms, and warm p95 at or below 150 ms.
- Linux and macOS limits locked from native prototypes before production UI implementation.
- Authenticode on Windows; Developer ID, Hardened Runtime, notarization, and stapling on macOS; an OpenPGP-signed complete manifest for Linux/cross-platform assets.
- Native lifecycle smoke for every mandatory installer and portable artifact.

The complete contract is [Recentry Cross-Platform v1](cross-platform-v1-spec.md).
