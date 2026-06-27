# Tauri Compatibility Boundaries

This file is the authoritative boundary note for compatibility behavior that is intentionally retained in Tauri packaging and deep-link configuration.

It exists because `tauri.conf.json` is part of strict JSON-based build and test flows, so inline comments or ad-hoc metadata are risky. Keep the runtime values in that JSON file clean, and document retention windows here instead.

## Scope

This document covers only the compatibility layer still exposed through:

- `src-tauri/tauri.conf.json`
- `src-tauri/src/utils/init.rs`
- `src-tauri/src/utils/resolve/scheme.rs`

It does not define branding, dependency, or service identity policy outside those boundaries.

## Desktop Deep-Link Schemes

Current retained schemes:

- `clash://`
- `clash-verge://`

Current config/runtime touchpoints:

- `src-tauri/tauri.conf.json` -> `plugins.deep-link.desktop.schemes`
- `src-tauri/src/utils/init.rs` -> Windows registry scheme registration
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

## Ownership Rule

If future work needs to change compatibility behavior in these Tauri config files, update this document in the same PR. This keeps the compatibility strategy auditable instead of spreading hidden assumptions across config, scripts, and runtime code.
