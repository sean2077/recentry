# Release and rollback

The first cross-platform release is an atomic distribution set. Windows NSIS/ZIP, Linux x86_64 and ARM64 DEB/AppImage, macOS Universal 2 app ZIP/DMG, the complete SHA-256 manifest, platform signatures, native package smoke, resource evidence, and real graphical acceptance must all be green together.

## Development artifacts

The manually dispatched `Unverified development packages` workflow builds and smoke-checks package structure on native hosted runners. Every artifact name contains `unverified-development`. These outputs may validate a packaging change but do not prove shortcuts, status items, focus, real desktop behavior, resources, publisher trust, or support.

## Promotion gate

Release mode is fail-closed:

1. Protected native acceptance supplies `RECENTRY_NATIVE_ACCEPTANCE=green` only after the complete matrix is reviewed.
2. Windows packaging requires an Authenticode tool and publisher thumbprint.
3. macOS packaging requires a Developer ID identity and notarytool keychain profile; it verifies Hardened Runtime signatures, notarization, and stapling.
4. The distribution manifest refuses any missing mandatory asset and requires an OpenPGP signing key.
5. A protected publication job must verify every signature and checksum again before creating a GitHub release.

No release publication job is enabled while the native UI/prototype, credential, and real-machine gates remain incomplete.

## Atomic rollback rehearsal

For a rehearsal, use a draft release or local manifest and simulate one missing/invalid mandatory asset. Confirm that promotion fails, README/support metadata remains `in development`, and immutable source/tag/evidence is retained.

If post-release evidence invalidates a mandatory target:

1. Withdraw the whole cross-platform release from normal promotion.
2. Return public support text to `in development`.
3. Preserve the tag, changelog, acceptance evidence, and withdrawal reason.
4. Publish a corrected patch only after the complete matrix passes again.
5. Delete assets or revoke credentials only for compromise or another security risk.
