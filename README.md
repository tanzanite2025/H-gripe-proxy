# Clash Verge Optimized

Clash Verge Optimized is the privately maintained Rust-led implementation in `tanzanite2025/clash-verge-optimized`. It is a cross-platform desktop proxy client built with Tauri 2, Rust, React, TypeScript, and the in-repository `mihomo/` runtime kernel.

The project keeps Mihomo compatibility while progressively moving verifiable runtime/data-plane paths from Go to Rust.

- Maintained repository: <https://github.com/tanzanite2025/clash-verge-optimized>
- Releases: <https://github.com/tanzanite2025/clash-verge-optimized/releases>
- License: GPL-3.0-only, see [LICENSE](./LICENSE)

## Project focus

This repository is maintained around five boundaries:

- **Rust/Tauri control plane**: configuration validation, rule explanation, subscription artifacts, runtime projection, diagnostics, audit trails, and typed frontend IPC.
- **In-repo Mihomo kernel**: `mihomo/` remains the fallback owner for production runtime paths that have not yet been migrated to Rust.
- **Go to Rust migration**: runtime capabilities are moved in bounded, opt-in, evidence-producing batches with rollback checkpoints and explicit fallback scope.
- **Security boundary hardening**: high-risk shell/fs paths, CSP, WebDAV/TLS, external URL handling, and runtime apply surfaces are kept explicit and auditable.
- **Reproducible packaging**: builds prefer repository-owned sidecars, resources, and scripts instead of dynamically pulling a latest kernel at package time.

## Current capability overview

### Desktop and control plane

- Tauri 2 desktop shell with a Rust backend IPC surface.
- React 19 / TypeScript frontend using React Query, React Router, Monaco, and the app component system.
- Config schema validation, rule parsing, rule explanation, config diffing, and diagnostics summaries.
- Subscription pipeline: remote profile fetch, immutable artifacts, active markers, and runtime projection.
- App runtime state: node pool, DNS/security profiles, policy binding, session observation/evaluation, and leak planning.
- Connection, traffic, and log information exposed through Rust monitors and Tauri events.

### Runtime and data-plane migration

Rust currently owns bounded opt-in execution/canary paths. Production defaults still retain Mihomo fallback unless the roadmap says otherwise.

Completed Rust evidence scopes include:

- **DNS runtime**: fake-ip allocation, fake-ip cache/reverse lookup, fallback-filter domain/ipcidr, fallback-filter geoip/geoip-code, nameserver-policy exact/suffix/geosite/rule-provider/wildcard, and the DNS policy/cache/upstream bundle canary.
- **Protocol forwarding**: loopback TCP/HTTP forwarding, DIRECT/REJECT policy, remote transport, HTTP CONNECT, Shadowsocks AEAD framing/session evidence, SOCKS5 auth, SOCKS5 TCP CONNECT, SOCKS5 BIND, SOCKS5 UDP ASSOCIATE, and SOCKS5 UDP fragment evidence.
- **Encrypted protocols**: bounded VMess/VLESS/Trojan loopback TCP canary sessions with shared framing, byte accounting, rollback/evidence artifacts, leak proof, and fallback proof.
- **TUN / system proxy**: route-mode planning, system-proxy apply, TUN config/restart apply, rollback record/apply, and bounded transparent IPv4/TCP packet parsing/execution evidence.
- **Mihomo fallback retirement evidence**: default data-plane closeout manifests, emergency rollback checkpoints, and fallback continuity proof.

Still fallback-owned by Mihomo or the system service:

- Default DNS ownership, live resolver replacement, full GeoIP database loading, production persistent cache storage, and geodata refresh.
- Non-loopback encrypted forwarding, QUIC/UDP variants, multiplexing, plugin transports, and default forwarding.
- SOCKS non-loopback UDP, full fragment queue/timeouts, and Shadowsocks UDP/plugin transports.
- System-wide packet capture, transparent proxy default paths, and full Mihomo binary removal.

The full migration plan is maintained in [docs/go-to-rust-migration-roadmap.md](docs/go-to-rust-migration-roadmap.md).

## System requirements

Primary validation target:

- Windows x64 with Microsoft WebView2 Runtime.

Also supported as build targets:

- Linux x64/arm64 with the standard Tauri WebKitGTK/appindicator dependencies.
- macOS Intel / Apple Silicon, macOS 11+ recommended.

Development toolchain:

- Node.js >= 18.0
- pnpm 10.33.0, as declared by `packageManager`
- Rust 1.95.0, as declared by `rust-toolchain.toml`
- Go >= 1.21 when editing `mihomo/`

## Quick start

```bash
pnpm install
pnpm dev
```

Common commands:

```bash
# Type-check frontend code
pnpm typecheck

# Lint frontend code
pnpm lint

# Build only the web frontend
pnpm web:build

# Start Tauri development mode
pnpm dev

# Build release packages
pnpm build

# Build with the faster local packaging profile
pnpm build:fast
```

Rust backend checks:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
```

Compile a focused Rust runtime test target:

```bash
cargo test --manifest-path src-tauri/Cargo.toml encrypted_protocols_bundle --lib --no-run
```

## Mihomo sidecar build

If you modify Go code under `mihomo/`, rebuild the sidecar before creating a production package. `scripts/prebuild.mjs` checks that the `src-tauri/sidecar/verge-mihomo*` binary is newer than the `mihomo/` source tree before `pnpm build` continues.

Windows x64 example:

```powershell
Set-Location .\mihomo
$env:CGO_ENABLED = '0'
$env:GOARCH = 'amd64'
$env:GOOS = 'windows'
$env:GOAMD64 = 'v2'
go build -tags with_gvisor -trimpath -ldflags "-w -s -buildid=" -o bin/mihomo-windows-amd64-v2.exe
Set-Location ..

New-Item -ItemType Directory -Force -Path .\src-tauri\sidecar | Out-Null
Copy-Item .\mihomo\bin\mihomo-windows-amd64-v2.exe .\src-tauri\sidecar\verge-mihomo-x86_64-pc-windows-msvc.exe -Force
pnpm build
```

Repository helper:

```bash
pnpm mihomo:sidecar
```

## Local IP metadata

IP geolocation, ASN, and timezone diagnostics use local MMDB databases. During development or packaging, place supported files in `src-tauri/resources/`; installed builds can also read them from the application data directory.

Supported names:

- `GeoLite2-City.mmdb` or `City.mmdb`: country, region, city, and timezone.
- `GeoLite2-ASN.mmdb` or `ASN.mmdb`: ASN and organization.
- `Country.mmdb`, `country.mmdb`, or `GeoLite2-Country.mmdb`: country-level fallback.

Windows app data directory:

```text
%APPDATA%/io.github.tanzanite2025.clash-verge-optimized/
```

Notes:

- City data is required for the most accurate timezone result.
- Country-only data keeps the app usable but falls back to country-level timezone inference.

## Project structure

```text
clash-verge-optimized/
├── src/                         # React / TypeScript frontend
│   ├── components/              # UI components
│   ├── hooks/                   # React hooks
│   ├── locales/                 # i18n resources
│   ├── pages/                   # Pages
│   ├── providers/               # React context providers
│   ├── services/                # Tauri IPC wrappers and API services
│   └── utils/                   # Frontend utilities
├── src-tauri/                   # Tauri / Rust backend
│   ├── src/cmd/                 # Tauri command handlers
│   ├── src/config/              # Configuration management
│   ├── src/core/                # Runtime, DNS, rule, and kernel logic
│   ├── resources/               # Packaged resources
│   └── sidecar/                 # Mihomo sidecar binaries
├── mihomo/                      # In-repo Go runtime kernel
├── crates/tauri-plugin-mihomo/  # Rust to Mihomo bridge plugin
├── scripts/                     # Build, release, migration, and verification scripts
├── docs/                        # Roadmaps and architecture docs
└── tests/                       # Test resources
```

## Development rules

- Go to Rust data-plane work should be real bounded implementation work, not another readiness-only or gate-only PR.
- Every runtime bundle should keep explicit evidence, rollback, leak proof, and Mihomo fallback scope.
- New bundle modules should be split by responsibility, such as constants, protocol/framing, execution, evidence, parsing, and tests.
- Do not claim default DNS, packet capture, transparent forwarding, or Mihomo binary removal is Rust-owned without repeated evidence and rollback history.
- If `mihomo/` is changed, update the sidecar binary or clearly state why the PR does not package a new kernel.

## Verification checklist

Before submitting changes, run at least:

```bash
pnpm typecheck
pnpm lint
pnpm web:build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
```

When changing a specific Rust bundle, also compile its focused test target, for example:

```bash
cargo test --manifest-path src-tauri/Cargo.toml <bundle_or_module_name> --lib --no-run
```

## License

GPL-3.0-only. See [LICENSE](./LICENSE).
