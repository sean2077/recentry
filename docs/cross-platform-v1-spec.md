# Recentry Cross-Platform v1 — approved

## Goal

Deliver a genuinely supported, low-resource Recentry release that opens or focuses stable VS Code recent projects through the same launcher contract on Windows, Linux, and macOS, with native host integration, installable and portable artifacts, verified publisher trust, and evidence-backed support claims.

## Topology

| Component | Done means | Clarity |
| --- | --- | --- |
| Cross-platform core | Stable VS Code discovery, recent-project parsing, ranking, deduplication, safe opening, configuration, and protocol behavior work across all supported platforms through explicit layout and path-identity seams. | Goal 0.95 · Constraints 0.92 · Criteria 0.93 · Context 0.95 |
| Platform host | Windows, Linux, and macOS provide native shortcuts, status UI, single-instance handling, current-user IPC, explicit-consent autostart, and supervised UI-child lifecycle behind one shared host state machine. | Goal 0.96 · Constraints 0.93 · Criteria 0.93 · Context 0.95 |
| Launcher UI | Each platform provides a native compact popup with identical interaction and information-layout semantics. | Goal 0.95 · Constraints 0.93 · Criteria 0.92 · Context 0.90 |
| Platform distributions | Every mandatory installer and portable artifact is reproducibly built, signed as required, smoke-tested, checksummed, and published as one atomic release set. | Goal 0.96 · Constraints 0.94 · Criteria 0.94 · Context 0.92 |
| Acceptance matrix | Native CI and real GUI environments prove functionality, resources, performance, installation, upgrade, removal, and publisher trust for every support target. | Goal 0.94 · Constraints 0.92 · Criteria 0.93 · Context 0.88 |
| Documentation and positioning | English project documentation, GitHub metadata, installation guidance, support status, and release presentation never claim more support than the recorded evidence proves. | Goal 0.94 · Constraints 0.90 · Criteria 0.91 · Context 0.86 |

## Constraints

- Supported platforms are Windows 10 and 11 x64, macOS 13 or later on Intel and Apple Silicon, and Linux on x86_64 and ARM64.
- Linux desktop integration must cover GNOME Wayland, KDE Plasma Wayland, one X11 session, and at least one real ARM64 Wayland environment before release.
- The resident host must stay at or below 10 MiB using the platform's documented resident/private-memory measure and at or below 0.5% idle CPU on every platform.
- Windows retains the accepted UI-process-tree limit of 60 MiB, cold activation p95 of 600 ms, and warm activation p95 of 150 ms. Linux and macOS UI memory and activation limits must be measured with disposable native prototypes and locked before production UI implementation. A failed gate stops that platform lane; limits are not relaxed automatically.
- The host remains independent of UI frameworks, SQLite, search engines, and editor providers. The UI remains an on-demand child process and exits after 15 minutes hidden.
- Shared crates own the domain model, launcher state, configuration schema, protocol, VS Code parsing, ranking, deduplication, and safe launch-request construction.
- Platform adapters own native event loops, shortcut backends, tray or menu-bar integration, autostart, single-instance guards, current-user transport, child activation, and UI rendering.
- Windows retains the direct Win32 UI. macOS uses AppKit. Linux production UI technology is selected only by the Linux native prototype gate.
- IPC is current-user only, size-bounded, and local. Windows uses named pipes; Linux and macOS use owner-only Unix-domain sockets unless a platform requirement proves that transport unsuitable.
- The default shortcut remains `Ctrl+Alt+R` where representable. A registration conflict must preserve another launcher entry point and provide a diagnostic instead of terminating the host.
- A missing Linux status-notifier host is a declared desktop capability absence, not an application failure; `recentry show` remains available.
- Autostart is enabled only after explicit user confirmation and uses HKCU Run on Windows, SMAppService on macOS, and the applicable XDG mechanism on Linux.
- Stable VS Code data is read only. Recentry never modifies VS Code databases or guesses through an incompatible format.
- Local targets and supported VS Code URIs are launched with argument arrays, never shell-composed commands.
- There is no telemetry, cloud synchronization, background network traffic, or transmission of project paths. Diagnostics hide or hash sensitive paths.
- Repository documentation and public-facing release text are written in English. The product UI supports English and Simplified Chinese.
- The independently versioned Windows-only `v0.1.0-beta.N` preview line, including `v0.1.0-beta.2`, does not satisfy this specification or change its support gates.

## Non-goals

- Cursor, VSCodium, Code OSS, JetBrains IDEs, Zed, or any editor other than stable VS Code.
- Ordinary files, manual project entries, filesystem scanning, custom commands, or treating search text as an executable target.
- A plugin ABI, plugin marketplace, runtime provider loading, or editor-provider expansion in the first cross-platform release.
- Cloud sync, telemetry, accounts, remote configuration, or an update service.
- Microsoft Store, Mac App Store, Homebrew, Flatpak, Snap, or hosted APT-repository distribution in this release.
- Windows ARM64, Linux architectures other than x86_64 and ARM64, or macOS versions earlier than 13.
- Identical platform-native visual chrome. Behavior, keyboard flow, and row semantics are invariant; platform conventions may shape rendering details.
- Declaring a platform supported from compilation, emulation, or hosted CI alone.
- Silently omitting a failed architecture, package format, signing step, desktop session, or resource gate from a cross-platform release.

## Decision Boundaries

### Locked choices

| Choice | Rationale or evidence | Owner | Revisit trigger |
| --- | --- | --- | --- |
| The first cross-platform release supports stable VS Code only. | Platform expansion and provider expansion are independent; the existing provider/opener seams remain the future extension point. | User | The complete three-platform VS Code matrix is green and a new provider has compatibility fixtures and an opener contract. |
| The host budget is uniform; UI budgets are platform-calibrated and then immutable for the release. | The host is intentionally simple, while native UI process accounting differs by operating system. | User | A shared runtime demonstrably passes every locked platform gate without behavior loss. |
| UI implementations are platform native. | Existing Windows evidence rejected Tauri and egui under the fixed gates and accepted direct Win32. | User | One shared toolkit passes the full resource and interaction matrix on every platform and materially reduces maintenance risk. |
| The shared host is a lifecycle state machine rather than a platform event loop. | Current code mixes portable command/lifecycle behavior with Win32 APIs; extracting explicit adapters preserves the verified behavior. | Project | A platform requires a lifecycle invariant that cannot be represented safely by shared host events. |
| Linux Wayland uses the XDG GlobalShortcuts portal; X11 has a native fallback; status UI uses StatusNotifierItem when a host exists. | These are the selected desktop capability contracts; command-line `recentry show` is the universal fallback. | User | The relevant desktop protocol is deprecated or replaced. |
| Every listed artifact is mandatory. | A partial artifact set is not the requested cross-platform release. | User | A vendor deprecates a format or an architecture loses supported tooling. |
| A support claim requires native CI plus real GUI acceptance. | Hosted CI cannot prove global shortcuts, tray/menu behavior, focus dismissal, multi-monitor placement, or real process budgets. | User | An automated environment is proven equivalent to the required real GUI evidence. |
| Supported artifacts participate in platform publisher-trust paths. | macOS distribution requires Developer ID/notarization for the intended trust experience; Windows publisher identity uses Authenticode; Linux assets use a signed checksum manifest. | User | Distribution moves to a trusted store or repository that supplies an equivalent verified chain. |
| The first supported cross-platform release and its rollback are atomic; Windows-only betas use a separate preview contract. | Missing one mandatory target blocks the cross-platform release. The owner accepted an independently versioned Windows preview track on 2026-07-21 without changing Linux/macOS support claims. | User | A platform beta is proposed for supported or stable status. |
| Public positioning follows evidence. | Code may merge before validation, but README and GitHub metadata must say `in development` until the entire matrix passes. | User | ReleaseReadiness becomes green or an atomic withdrawal occurs. |

### Open choices

| Choice | Owner | Decision trigger |
| --- | --- | --- |
| Exact Linux native UI toolkit. | Project proposes; user accepts the gate result. | Disposable Linux candidates have comparable interaction, resource, desktop-integration, and package evidence. |
| Exact Linux and macOS UI memory, cold-p95, and warm-p95 limits. | User | Native prototypes produce repeatable 30-sample measurements on the required real environments. |
| Exact supported Linux distribution names, versions, and minimum runtime baseline. | User | Before production Linux packaging and before any Linux support claim. The chosen set must cover the required GNOME, KDE, X11, x86_64, and ARM64 environments. |
| Signing identities and protected release credentials. | User | Before enabling signed release jobs; absence blocks release but not unsigned development builds. |
| Cross-platform release version and date. | User | All implementation, prototype, credential, CI, and real-machine gates are green. |

### Atomic rollback contract

If post-release evidence invalidates any mandatory asset, architecture, desktop session, signature, security property, functional behavior, resource budget, or activation gate:

1. Mark the whole cross-platform release as withdrawn and remove it from normal download promotion.
2. Return README, GitHub metadata, and the support matrix to `in development`.
3. Preserve the Git tag, changelog, acceptance evidence, and withdrawal reason.
4. Publish a corrected patch only after the complete matrix passes again.
5. Delete binaries and revoke credentials only when compromise, malicious content, signing-key exposure, or another security risk makes continued availability harmful.

## Acceptance Criteria

- [ ] Stable VS Code is discovered from supported PATH and platform installation layouts on Windows, Linux, and macOS, including a configured path override.
- [ ] Current application-shared and legacy recent-project databases are opened read-only; incompatible, locked, missing, or corrupt data fails safely with redacted diagnostics.
- [ ] Local folders, `.code-workspace` files, and supported VS Code remote URIs are included; ordinary files and unsupported schemes are excluded.
- [ ] Empty search preserves VS Code order; non-empty search ranks name, path, and recency deterministically and deduplicates with platform-correct path identity semantics.
- [ ] Opening an already-open target requests focus; another target explicitly opens a new VS Code window; all arguments are passed without a shell.
- [ ] Windows, macOS, GNOME Wayland, KDE Plasma Wayland, X11, and Linux ARM64 satisfy their shortcut, status UI or fallback, single-instance, IPC, autostart, child-crash recovery, and shutdown contracts.
- [ ] Every summon clears the query, focuses search, and centers the popup on the active display.
- [ ] Every row remains one compact line: `[VSCode] <project name>  <path>`, with a primary name, muted and ellipsized path, and full path available on hover or the platform accessibility equivalent.
- [ ] Up, Down, Enter, and Esc work without a mouse; focus loss and successful opening hide the launcher immediately.
- [ ] System theme and language are followed by default, with manual English and Simplified Chinese selection.
- [ ] The resident host meets the 10 MiB and 0.5% idle CPU limits on every supported platform.
- [ ] Windows repeats the existing 30-cold/30-warm UI measurements and passes 60 MiB active-tree, 600 ms cold-p95, and 150 ms warm-p95 limits.
- [ ] Linux and macOS production builds pass their pre-locked native prototype limits using the same measurement definitions and sample counts.
- [ ] The UI process exits after 15 minutes hidden, restarts on demand, and may crash without terminating the host.
- [ ] Native CI builds, tests, packages, smoke-tests, checksums, and uploads the complete artifact matrix: Windows x64 NSIS and ZIP; Linux x86_64 and ARM64 DEB and AppImage; macOS Universal 2 DMG and app ZIP.
- [ ] A SHA-256 manifest covers every binary asset.
- [ ] Windows binaries and installer pass Authenticode verification; the macOS app and DMG pass Developer ID, Hardened Runtime, notarization, and stapling checks; the Linux checksum manifest passes OpenPGP verification.
- [ ] Installation, portable launch, in-place upgrade, removal, autostart changes, and configuration retention/removal behavior are tested for every applicable format.
- [ ] Real GUI acceptance passes on Windows 10 and 11 x64, macOS 13 or later on Intel and Apple Silicon, Linux x86_64 on GNOME Wayland, KDE Plasma Wayland, and X11, and Linux ARM64 on at least one Wayland desktop.
- [ ] README, GitHub description/topics, installation guidance, release notes, support matrix, and troubleshooting text are English, accurate, and remain `in development` until every required test and credential gate is green.
- [ ] Diagnostics and release logs contain no raw project paths, secrets, signing material, or telemetry.
- [ ] A rehearsed withdrawal verifies the atomic rollback contract without deleting immutable release evidence.

## Ontology

| Term | Meaning and relationships |
| --- | --- |
| `RecentProject` | A stable VS Code folder, workspace, or supported remote target produced by `VsCodeProvider` and consumed by the opener and launcher state. |
| `VsCodePlatformLayout` | Supplies per-platform executable, product metadata, database, window-state, configuration-root, and target-identity rules while shared parsing and ranking remain platform neutral. |
| `TargetIdentityPolicy` | Produces deduplication keys with platform-correct path case and separator semantics plus URI normalization. |
| `HostRuntime` | Shared command, configuration, consent, diagnostics, UI-supervision, and shutdown state machine that consumes `HostEvent` values. |
| `HostAdapter` | Platform service for native shortcuts, status UI, event-loop integration, and capability detection; it remains within `HostBudget`. |
| `CurrentUserTransport` | Size-bounded owner-only IPC: named pipes on Windows and Unix-domain sockets on Linux/macOS. |
| `SingleInstanceGuard` | Ensures one `HostRuntime` per user and forwards commands from later invocations. |
| `UiChildPlatform` | Owns native child spawning, foreground activation, exit detection, graceful stop, and forced termination. |
| `AutostartService` | Implements explicit-consent login startup through the platform mechanism. |
| `LauncherState` | Shared headless query, result, selection, and command state consumed by every `PlatformUiAdapter`. |
| `LauncherContract` | Invariant placement, focus, reset, keyboard, dismissal, theme, language, and result-row behavior. |
| `PlatformUiAdapter` | Win32, AppKit, or selected Linux-native renderer that implements `LauncherContract` without owning project operations. |
| `PrototypeGate` | Produces a locked `UiBudget` from comparable native measurements and blocks production when no candidate passes. |
| `DistributionSet` | The complete atomic set of mandatory `ReleaseAsset` values for all supported platforms and architectures. |
| `PackagePipeline` | Native build, bundle, smoke-test, sign, checksum, and upload workflow that produces a `DistributionSet`. |
| `ArtifactSigningPolicy` | Windows Authenticode, macOS Developer ID/notarization/stapling, and Linux OpenPGP checksum-signature requirements. |
| `AcceptanceMatrix` | Verifies each platform, architecture, desktop session, package format, behavior, resource gate, and trust chain in CI and real GUI environments. |
| `ReleaseReadiness` | Atomic gate requiring all assets, CI, real GUI, resources, signatures, and positioning checks to be green. |
| `ReleaseRollbackPolicy` | Withdraws the complete support claim when any post-release mandatory gate fails while preserving immutable release lineage. |

## Open Assumptions

- Default: keep the existing Windows behavior and accepted measurements as the reference contract; revisit if cross-platform extraction reveals a verified Windows regression risk.
- Default: use owner-only Unix-domain sockets for Linux and macOS IPC; revisit if a native sandbox or application-bundle constraint requires another local current-user transport.
- Default: configuration uses the platform-standard per-user application-data location and atomic replacement; revisit if a package sandbox supplies a mandatory container location.
- Default: a desktop without a status-notifier host remains supported through the global shortcut and `recentry show`; revisit if neither entry point is available in a declared supported environment.
- Default: development CI may produce unsigned artifacts for testing, but they are never promoted or described as supported releases; revisit when the complete signing credential set is available.
- Default: independently versioned Windows-only betas may remain available under their preview contract unless a security issue requires withdrawal.
