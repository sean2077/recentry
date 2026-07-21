# ADR 0003: Cross-platform package tooling boundaries

- Status: accepted for implementation
- Date: 2026-07-21

## Context

The repository already owns two directly invoked Windows commands under `tools/`: one builds the Windows distribution and one smoke-tests it. GitHub Actions and developers call those paths directly. The cross-platform specification adds Linux DEB/AppImage, macOS app ZIP/DMG, complete checksums, platform trust verification, and an atomic release-readiness decision. These jobs have different native tools, credentials, failure domains, and smoke tests.

No command inventory or general-purpose task runner exists. `tools/` is the observed project-owned command root; `packaging/` owns declarative package inputs rather than operator entry points. Existing Windows paths are active contracts and stay unchanged.

## Governance decision

### Selected method cards

- **Task or journey:** package and smoke-test are separate invocations because a reproducible artifact can exist even when installation/runtime verification fails.
- **State or artifact:** Windows, Linux, macOS, and the final manifest own distinct artifact sets and native staging state.
- **Invoker or entry:** developers and CI invoke the same authoritative commands; private helpers are not public entries.
- **Hazard, recovery, and verification:** staging is disposable, `dist/` replacement is opt-in, signing credentials are never accepted as positional data, and release mode fails closed when trust evidence is absent.
- **Distribution contract:** existing Windows tool paths remain stable; new paths are documented before CI consumes them.
- **Implementation form:** PowerShell remains the Windows-native entry; POSIX shell is used only on Linux/macOS native runners.

### Rejected method cards

- **Domain/team hierarchy:** one small repository has no independent packaging teams or durable subdomains that justify nested command roots.
- **Lifecycle directories:** development versus release is an explicit mode and evidence state, not a directory taxonomy for commands.
- **One aggregate release script:** credentials, native toolchains, smoke tests, and rollback differ too much for a safe cross-OS local aggregate. GitHub Actions is the coordinator, while each native command remains independently verifiable.

## Job boundaries and contract profiles

| Job | Authoritative entry | Artifact/state owner | Mutation and recovery | Verification |
| --- | --- | --- | --- | --- |
| Build Windows package | `tools/package-windows.ps1` | NSIS, ZIP, Windows checksums | isolated staging; refuses replacement without `-Force` | binary presence, NSIS result, checksum coverage |
| Smoke Windows package | `tools/test-windows-package.ps1` | disposable install roots and per-user package records | exact scratch root; silent uninstall in cleanup | portable launch, install, upgrade, uninstall |
| Build Linux package | `tools/package-linux.sh` | architecture-specific DEB and AppImage | isolated staging; refuses replacement without `--force` | metadata, payload, native launch smoke |
| Smoke Linux package | `tools/test-linux-package.sh` | extracted DEB/AppImage runtime | exact temporary directory and process cleanup | command/IPC lifecycle plus package structure |
| Build macOS package | `tools/package-macos.sh` | Universal 2 app ZIP and DMG | isolated staging; refuses replacement without `--force` | architectures, bundle metadata, signatures when required |
| Smoke macOS package | `tools/test-macos-package.sh` | mounted/copied app runtime | temporary mount/copy cleanup | command/IPC lifecycle, Gatekeeper/signature evidence |
| Assemble distribution manifest | `tools/package-manifest.sh` | complete cross-platform SHA-256 manifest and detached OpenPGP signature | refuses partial mandatory asset set and replacement | exact asset coverage and signature verification |

All new entries reject unknown arguments, require an explicit `--development` or `--release` mode, keep diagnostics on stderr, and never print credential material. Development mode may create unsigned native test artifacts but cannot produce release-readiness evidence. Release mode must fail if any mandatory signature, notarization, architecture, package, or smoke result is absent.

## Placement and naming

Direct commands remain flat under `tools/` because callers search by one packaging outcome and the repository has only seven entries. Declarative inputs live under `packaging/<platform>/`. Generated staging, mounted images, extracted packages, and measurements live under ignored `target/`, `scratch/`, or `dist/` surfaces according to their lifecycle.

## Coordination and verification

The workflow will invoke each native entry by its stable path and upload development artifacts only with an explicit `unverified-development` label. Release publication is a separate protected job gated by the complete signed manifest and native/real-GUI acceptance evidence. No workflow or script changes the historical `v0.1.0-beta.1` release.

Minimum verification is shell/PowerShell syntax, invalid-argument rejection, overwrite refusal, native package inspection, native runtime smoke, checksum completeness, and the existing repository domain tests. Real GUI, resources, publisher trust, and rollback remain mandatory external release gates.
