# Go-to-Rust kernel roadmap (learn.gripe)

This is the single source of truth for replacing the Go/Mihomo sidecar with our
own pure-Rust, in-process proxy kernel: **`learn.gripe`** (crate `learn-gripe`,
module `learn_gripe` — Rust identifiers cannot contain a dot, so the dotted form
is the brand/display name only). Use PRs and `git log` for implementation
archaeology.

## Goal

Own the entire data plane in Rust. The app no longer launches, packages, or
depends on the Go/Mihomo sidecar binary. Inbound listeners, outbound dialing,
routing/rules, DNS, and TUN are all implemented inside `learn-gripe`, which we
control end to end: protocol coverage, config format, naming, and upgrade cadence
are ours.

Target ownership chain (no external sidecar anywhere in it):

```text
App registry / policy / node pool / DNS / security profile
  -> Rust-owned runtime plan
  -> learn-gripe config
  -> learn-gripe in-process kernel (inbound -> routing -> outbound)
  -> Rust-observed runtime state
```

### Why we abandoned the previous roadmap shape

The earlier version of this document tracked a "Mihomo stays the production data
plane, prove Rust parity through bounded canary evidence gates" strategy. That
approach is **retired**. It retired Mihomo *startup* without shipping a working
Rust replacement, which left `start_core()` returning an error and the app with
no data plane at all. We are not reviving canary/gate/dry-run/readiness PRs.
Progress is now measured by **real traffic flowing through `learn-gripe`** and by
Mihomo-owned surface being deleted, not by another `*_guard` / `*_evidence`
wrapper.

History from that phase (DNS/adapter/protocol/TUN bounded canaries, fallback
retirement manifests, release closeout) remains useful only as audit history in
PR/git. Do not re-create those documents or re-introduce that PR shape.

## Current state

`learn-gripe` MVP is merged and is the live data plane (PR #357). `start_core()`
boots the Rust kernel; there is no Mihomo startup path left.

| Area | State |
| --- | --- |
| learn-gripe kernel | MVP live. SOCKS5 inbound (no-auth CONNECT, IPv4/IPv6/domain) relaying through a direct or upstream-SOCKS5 outbound via `tokio::io::copy_bidirectional`. Bound + started inside `CoreManager::start_core()`; lifecycle via `GripeHandle` (`local_addr`, graceful `shutdown`). New `RunningMode::Gripe`. Two end-to-end relay tests in `crates/learn-gripe/tests/socks5_relay.rs`. |
| Rust control plane | Mature. Validation, planning, projection artifacts, subscription pipeline, monitor paths, audit, telemetry, and frontend type surfaces are Rust-owned (see "Completed control-plane milestones"). |
| Mihomo sidecar | No longer started or packaged for the supported path. `tauri-plugin-mihomo` is still a dependency (compatibility DTOs / dead code) and the binary is still in the tree; removal is the final phase, not yet done. |

What the MVP intentionally does **not** do yet: protocol ciphers (SS / VMess /
VLESS / Trojan), UDP, routing/rule evaluation, DNS, TUN, system-proxy
integration, and node-aware outbound selection (`start_core` currently always
uses Direct outbound regardless of the selected node).

## Build vs adopt boundary

This section is the durable decision record for **what we implement ourselves and
what we build on top of vetted crates**. It exists so future work does not drift
back into either extreme (re-adopting a whole external kernel, or hand-rolling
cryptography). "Owning the kernel" means owning architecture, protocol logic,
routing, config, and lifecycle — it does **not** mean re-implementing TLS or
ciphers.

### We build ourselves (this is `learn-gripe`)

These are the parts that define the product and where we want full control:

- Inbound listeners and the connection accept/relay loop (SOCKS5 today; HTTP and
  more later).
- Outbound dialing and the proxy protocol framing/handshakes we choose to
  support: Shadowsocks, VMess, VLESS, Trojan wire formats.
- Routing / rule engine and node selection (which outbound a connection takes).
- Config model, subscription ingestion into that model, and runtime lifecycle
  (start / stop / restart, graceful shutdown, hot config swap).
- UDP relay orchestration, connection accounting, and observability hooks.
- TUN packet plumbing *orchestration* (read packets, route, hand to outbound).

### We adopt vetted crates (do NOT hand-roll these)

Hand-writing these is a security and reliability liability with no product
upside. Standing on mature crates is industry-standard and does not cost us
control of the kernel.

| Concern | Crate(s) | Why not hand-roll |
| --- | --- | --- |
| Async runtime | `tokio` | Re-implementing an async reactor is pure cost. |
| TLS | `rustls` (or `boring`) | Hand-rolled TLS is a classic source of critical CVEs. |
| Cryptographic primitives | `ring` / `aes-gcm` / `chacha20poly1305` | Never implement your own cryptography. |
| TUN device | `tun` | Wraps per-OS syscalls/ioctls we should not duplicate. |
| DNS resolver base | `hickory-dns` | Full resolver/cache/protocol surface; reuse it. |
| Serde / config parsing | `serde`, `serde_yaml`, `serde_json` | Standard, audited. |

Rule of thumb: if getting it wrong produces a *security vulnerability* (TLS,
crypto, OS syscalls), adopt a vetted crate. If it defines *product behavior*
(protocols we speak, how we route, how we configure), build it in `learn-gripe`.

### Hard "do not" list

- Do not adopt `clash-rs`, the Mihomo library, or any other whole external proxy
  kernel. The point of `learn-gripe` is that we own it.
- Do not implement cryptography or TLS by hand.
- Do not re-introduce a Go/Mihomo sidecar startup or packaging path.
- Do not add canary / gate / dry-run / readiness / parser-only PRs as migration
  "progress". A PR is progress only if it adds working `learn-gripe`
  functionality or removes Mihomo-owned surface.

## Forward roadmap

Each phase is a small number of cohesive PRs (implementation + tests together),
not a long list of gates. Order is by what unblocks real-world usage fastest.

### Phase 1 — MVP data plane (DONE, PR #357)

SOCKS5 inbound + direct/upstream-SOCKS5 outbound, wired into `start_core()`, with
end-to-end relay tests. Proves the in-process architecture works.

### Phase 2 — Make it usable for real nodes

- HTTP/HTTPS inbound (`CONNECT` + plain proxy) alongside SOCKS5, so the existing
  system-proxy integration keeps working.
- Node-aware outbound selection: read the selected node from app runtime state
  and dial the matching outbound instead of always Direct.
- First real proxy protocol: **Shadowsocks** (AEAD: aes-256-gcm /
  chacha20-poly1305 on top of `ring`/`aes-gcm`). End-to-end test against a local
  SS server.

### Phase 3 — Protocol breadth + routing

- VMess, VLESS, Trojan outbounds (TLS via `rustls`).
- Routing/rule engine: reuse the already-Rust-owned rule matching to pick the
  outbound per connection (DOMAIN / CIDR / GEOIP / GEOSITE / MATCH ...).
- UDP relay (SOCKS5 UDP ASSOCIATE) for the supported protocols.

### Phase 4 — DNS + TUN

- DNS handling inside the kernel (fake-ip + upstream resolution) on top of
  `hickory-dns`.
- TUN mode: read packets via the `tun` crate, route through `learn-gripe`
  outbounds, with leak-safe rollback. This is the highest-risk phase; keep
  rollback explicit.

### Phase 5 — Delete Mihomo

Only after the supported default paths above run on `learn-gripe`:

- Remove the `tauri-plugin-mihomo` dependency and remaining compatibility DTOs
  (replace with Rust-native DTOs).
- Remove the Mihomo binary from the tree and from release packaging.
- Remove dead Mihomo service/sidecar plumbing.

## Definition of done for a roadmap PR

A PR counts as kernel progress only if it does at least one of:

- Adds working `learn-gripe` functionality (a protocol, inbound, routing, DNS,
  TUN, etc.) with an end-to-end test that moves real bytes, **or**
- Removes a concrete Mihomo-owned surface (dependency, binary, packaging, dead
  command/plumbing).

It must also answer:

- What can `learn-gripe` now do that it could not before?
- What did the change build in-house vs delegate to a vetted crate (and why)?
- What is the rollback if it touches the live runtime path?
- What Mihomo-owned surface remains after this PR?

## Non-negotiable boundaries

### 1. Rust is the source of truth for the data plane

There is no external kernel and no sidecar. All inbound/outbound/routing/DNS/TUN
behavior lives in `learn-gripe`. The frontend never talks to an external proxy
controller API.

### 2. Own product logic; delegate security primitives

Follow the "Build vs adopt boundary" above. Protocol/routing/config/lifecycle are
ours; TLS/crypto/TUN-syscalls/DNS-base come from vetted crates.

### 3. Runtime apply stays gated and reversible

Real runtime changes (especially TUN / system proxy in Phase 4) must keep an
explicit apply + observe + rollback path so a bad config cannot strand the
network. This is the one piece of the old roadmap worth keeping.

### 4. Frontend runtime types use a view-model boundary

UI-specific semantics (`IProxyItem.provider`, `fixed`, expanded group `all`
items, etc.) stay in app-owned view models in `src/types/proxy.ts`. As Mihomo
DTOs are removed, replace them with Rust-native field sources behind the same
view-model boundary — do not force raw generated types into the UI.

## Completed control-plane milestones

These predate the kernel work and remain valid; `learn-gripe` consumes them.

| Area | Durable result |
| --- | --- |
| Config validation | Rust native validator replaced the old `verge-mihomo -t` chain. |
| Rule engine | DOMAIN, CIDR, port, NETWORK, MATCH, GEOIP, GEOSITE, ASN, RULE-SET, process, UID, DSCP, inbound, wildcard, logical, and sub-rule paths are Rust-owned. |
| Control diagnostics | Rule explain, config diff, diagnostics summary, latency planner, and node selection planner are Rust-owned. |
| Subscription pipeline | Source config -> artifact -> active artifact -> runtime is transactional and Rust-owned. |
| App-facing monitor path | Connection, traffic, memory, and log views use Rust monitor controllers and Tauri events. |
| App runtime orchestration | Runtime plan, projection artifact, staged activation, runtime-apply decision, verification closeout, and post-apply hold are Rust-owned. |
| Runtime mutation audit | Mode, system proxy, TUN toggle, DNS apply, geo update, sensitive-config edits, TLS rotation, and upgrade actions are audited. |
| Proxy type boundary | Proxy globals moved to app-owned view models backed by Rust field sources. |

## Document maintenance rules

- Keep this file compact and current-state oriented; it tracks `learn-gripe`
  progress and the build-vs-adopt boundary.
- Do not re-add historical per-PR logs or the retired canary/gate framing.
- Do not create parallel Go-to-Rust status documents; update this roadmap.
- Use PR history and `git log` / `git blame` for archaeology.
