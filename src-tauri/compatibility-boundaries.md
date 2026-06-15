# Tauri Compatibility Boundaries

This file is the authoritative boundary note for compatibility behavior that is intentionally retained in Tauri packaging and deep-link configuration.

It exists because `tauri.conf.json` and `tauri.linux.conf.json` are part of strict JSON-based build and test flows, so inline comments or ad-hoc metadata are risky. Keep the runtime values in those JSON files clean, and document retention windows here instead.

## Scope

This document covers only the compatibility layer still exposed through:

- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.linux.conf.json`
- `src-tauri/src/utils/init.rs`
- `src-tauri/src/utils/resolve/scheme.rs`

It does not define branding, dependency, or service identity policy outside those boundaries.

## Desktop Deep-Link Schemes

Current retained schemes:

- `clash://`
- `clash-verge://`

Current config/runtime touchpoints:

- `src-tauri/tauri.conf.json` -> `plugins.deep-link.desktop.schemes`
- `src-tauri/src/utils/init.rs` -> Linux handler registration
- `src-tauri/src/utils/resolve/scheme.rs` -> runtime scheme acceptance

Why they are still kept:

- Existing subscription ecosystems still emit `clash://` links.
- Some historical desktop integrations still point at `clash-verge://`.
- Removing them now would break inbound subscription import for upgraded users before a replacement migration has been proven.

Retention window:

- Keep through at least one stable release cycle after the surrounding migration cleanup work is fully shipped.
- Keep longer if real subscription sources used by the project ecosystem still emit either legacy scheme.

Exit conditions:

1. A replacement scheme strategy exists, or the project explicitly decides to keep `clash://` as the permanent ecosystem entrypoint.
2. Release notes announce deprecation before removal.
3. Runtime handling, packaging registration, and import UX are verified on supported desktop platforms after the deprecation window.
4. The removal PR deletes only the legacy scheme path and includes migration verification notes.

Removal rule:

- Do not remove `tauri.conf.json` scheme entries without removing the matching runtime acceptance path in `resolve/scheme.rs`.
- Do not remove runtime acceptance without also confirming platform registration behavior in `init.rs`.

## Linux Package Replacement Fields

Current retained compatibility fields:

- `deb.provides = ["clash-verge"]`
- `deb.conflicts = ["clash-verge"]`
- `deb.replaces = ["clash-verge"]`
- `rpm.provides = ["clash-verge"]`
- `rpm.conflicts = ["clash-verge"]`
- `rpm.obsoletes = ["clash-verge"]`

Current config touchpoint:

- `src-tauri/tauri.linux.conf.json`

Why they are still kept:

- They preserve smoother upgrade and replacement behavior for systems that still have legacy `clash-verge` packages installed.
- They keep package-manager conflict semantics explicit during the migration window instead of failing later at install time.

Retention window:

- Keep through at least one stable Linux packaging cycle after the renamed package identity is confirmed in real upgrade scenarios.
- Keep longer if `.deb` or `.rpm` users still need direct upgrade continuity from old `clash-verge` installs.

Exit conditions:

1. Upgrade tests from legacy `clash-verge` packages to the current package pass on both `.deb` and `.rpm`.
2. Release notes announce the retirement of legacy package replacement behavior.
3. The project no longer needs in-place replacement of old package names for supported Linux distributions.
4. The removal PR is isolated to package-compatibility retirement and documents test coverage.

Removal rule:

- Do not remove these fields before the matching post-install and pre-remove migration scripts have already been reviewed for the same release window.

## Ownership Rule

If future work needs to change compatibility behavior in these Tauri config files, update this document in the same PR. This keeps the compatibility strategy auditable instead of spreading hidden assumptions across config, scripts, and runtime code.
