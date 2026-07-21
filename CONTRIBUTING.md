# Contributing

Recentry accepts focused changes that preserve its low-resource and privacy contracts. Open an issue before selecting a new native UI toolkit, changing a fixed budget, adding an editor provider, or changing the release artifact set.

## Local checks

Use Rust 1.86 or newer:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Run native package commands only on their target operating system. Development artifacts must remain labelled unverified. Never change README/support metadata from `in development` based only on compilation, emulation, or hosted CI.

## Design boundaries

- Keep the host independent of UI frameworks, SQLite, project providers, and search engines.
- Put shared project/configuration/protocol behavior in portable crates.
- Keep platform event loops, shortcuts, status UI, autostart, activation, and rendering in platform adapters.
- Read stable VS Code state only; never write, migrate, or repair it.
- Pass targets through argument arrays and reject unsupported URI schemes.
- Add fixture tests for every compatible data shape or platform path rule.

## Pull requests

Describe the behavior boundary, tests run, native environments actually observed, resource impact, and any remaining acceptance blocker. Do not include project databases, raw paths, signing material, or generated `dist/`, `target/`, or `scratch/` content.
