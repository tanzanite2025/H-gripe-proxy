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
| TLS | `rustls` — **vendored Watfaq fork in `third_party/`** (see "Vendored TLS + REALITY" below) | Hand-rolled TLS is a classic source of critical CVEs. REALITY needs a ClientHello hook that upstream `rustls` does not expose, so we vendor a superset fork instead of hand-rolling. |
| Cryptographic primitives | `ring` / `aes-gcm` / `chacha20poly1305` | Never implement your own cryptography. |
| TUN device | `tun` | Wraps per-OS syscalls/ioctls we should not duplicate. |
| DNS resolver base | `hickory-dns` | Full resolver/cache/protocol surface; reuse it. |
| Serde / config parsing | `serde`, `serde_yaml`, `serde_json` | Standard, audited. |

Rule of thumb: if getting it wrong produces a *security vulnerability* (TLS,
crypto, OS syscalls), adopt a vetted crate. If it defines *product behavior*
(protocols we speak, how we route, how we configure), build it in `learn-gripe`.

### Vendored TLS + REALITY (`third_party/`)

This is the durable record of **why our `rustls` is vendored and self-maintained**,
so future work does not "simplify" it back to the upstream crate and silently
break REALITY.

- **What is vendored.** `third_party/rustls/` and `third_party/tokio-rustls/`
  hold the full source of the Watfaq `rustls` fork
  (`https://github.com/Watfaq/rustls`, branch `watfaq/0.23.40`) and its matching
  `tokio-rustls` (`watfaq/0.26.4`). The workspace root `Cargo.toml` redirects the
  crates-io `rustls` / `tokio-rustls` to these paths via `[patch.crates-io]`, so
  the **entire** workspace (reqwest, hyper-rustls, `learn-gripe`, …) builds
  against the in-tree copy. The fork is a strict **superset** of upstream
  `rustls`, so all existing plain-TLS users keep working unchanged.
- **Why a fork at all.** REALITY authentication has to embed an x25519-derived
  auth token into the TLS 1.3 ClientHello `session_id`, and client-fingerprint
  shaping has to control ClientHello layout. Upstream `rustls` exposes **no hook**
  for either. Every working Rust REALITY client (clash-rs included) therefore
  rides a patched `rustls`; the Watfaq fork adds exactly that —
  `ClientConfig::builder().with_reality(RealityConfig)` plus a
  `RealityServerCertVerifier`. We are *not* hand-rolling TLS or cryptography;
  the crypto primitives inside the fork are the usual vetted ones
  (`ring` / `x25519-dalek` / HKDF / AES-GCM).
- **Why vendored in-tree (not a git dependency).** The owner wants the kernel to
  be fully offline and self-maintained: no dependency on an external fork repo
  staying alive or unchanged. The source lives in our repo and we own updates.
- **Maintenance model.** We own these copies. To pull upstream security fixes,
  re-sync from the Watfaq branches above (or rebase the REALITY patch onto a newer
  upstream `rustls`) and re-vendor — keep `third_party/rustls/Cargo.toml` and
  `third_party/tokio-rustls/Cargo.toml` (the de-workspaced manifests we wrote) in
  step with any new dependency/feature changes. Verify with
  `cargo check --workspace` and `cargo test -p learn-gripe`.
- **License.** Upstream license files are retained in place
  (`third_party/rustls/LICENSE-{APACHE,ISC,MIT}`,
  `third_party/tokio-rustls/LICENSE-{APACHE,MIT}`). `rustls` is
  `Apache-2.0 OR ISC OR MIT`; `tokio-rustls` is `MIT OR Apache-2.0`.
- **Built on this.** `Security::Reality` in `learn-gripe` (reality-opts auth +
  `servername` masquerade SNI + `client-fingerprint` parsing) wraps this fork's
  `with_reality()` API via `tls::connect_reality`; because Security and Transport
  are orthogonal, VLESS-REALITY works over tcp/grpc/h2/xhttp automatically (proven
  by the REALITY relay tests in `crates/learn-gripe/tests/vless_outbound.rs`).
  The same orthogonality means every outbound protocol gets this for free: the
  `Trojan` and `VMess` outbounds share the security+transport pipeline via
  `transport::build_layers`, so Trojan-/VMess-REALITY/-TLS over any transport
  works too (proven by `crates/learn-gripe/tests/trojan_outbound.rs` and
  `crates/learn-gripe/tests/vmess_outbound.rs`).
  Faithful uTLS-style `client-fingerprint` ClientHello shaping is tracked after
  that; the fingerprint is parsed and retained today but does not yet reshape the
  handshake. The `flow: xtls-rprx-vision` layer is done: it is a VLESS body
  framing (padding of the tunneled bytes), not a security/transport layer, so it
  composes with `none`/`tls`/`reality` over raw TCP without touching `rustls`
  (proven by the Vision relay tests in
  `crates/learn-gripe/tests/vless_outbound.rs`).

### Hard "do not" list

- Do not adopt `clash-rs`, the Mihomo library, or any other whole external proxy
  kernel. The point of `learn-gripe` is that we own it. (Vendoring a *TLS library*
  fork into `third_party/` is **not** adopting a kernel — it is the "adopt a
  vetted crate" rule applied to TLS, with the source pulled in-tree so we own it.)
- Do not implement cryptography or TLS by hand.
- Do not drop the `[patch.crates-io]` redirect or replace the vendored `rustls`
  with the upstream crate — that silently removes the REALITY ClientHello hook.
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

- VMess, VLESS, Trojan outbounds (TLS via `rustls`). All three are done (all
  transports × none/tls/reality, sharing `transport::build_layers`). VMess uses
  the modern AEAD header (`alterId: 0`) with `aes-128-gcm` / `chacha20-poly1305`
  body security; the legacy MD5 (`alterId > 0`) format is rejected. Crypto is
  delegated to vetted RustCrypto crates; only the VMess-specific nested-HMAC KDF
  and on-wire framing are assembled in-crate (KDF cross-checked against an
  independent implementation). VLESS additionally supports `flow:
  xtls-rprx-vision` over raw TCP: the request header carries the Vision flow
  addon and the body is wrapped in the XTLS Vision padding framing
  (`commandPaddingContinue/End/Direct`, TLS-record-aware padding), ported from
  Xray and cross-checked end-to-end against an independent receiver.
- Routing/rule engine: done for the in-kernel data plane. `learn-gripe` now has
  a `Router` (`OutboundMode::Routed`) that selects the outbound per connection
  from an ordered rule list (`DOMAIN` / `DOMAIN-SUFFIX` / `DOMAIN-KEYWORD` /
  `IP-CIDR` v4+v6 / `MATCH`), resolving to named outbounds plus the built-in
  `DIRECT` / `REJECT` policies, with a `fallback` for the no-match case (proven
  by `crates/learn-gripe/tests/router_outbound.rs`). `GEOIP` / `GEOSITE` need
  external mmdb / geosite data and are left for a follow-up.
- UDP relay (SOCKS5 UDP ASSOCIATE): the inbound now answers `UDP ASSOCIATE`,
  binds a relay socket, and relays SOCKS5-wrapped datagrams to/from remote hosts
  with one egress socket per destination; the association lives as long as its
  TCP control connection (RFC 1928). Egress is **Direct** only today — the
  outbound is gated so `Direct` / `Routed` accept the association (the route is
  resolved per datagram) while pure proxy outbounds refuse it with
  `REP_CMD_NOT_SUPPORTED`. Proxy-tunnelled UDP (VLESS / Trojan / VMess UDP
  framing) is the next follow-up. Proven by
  `crates/learn-gripe/tests/udp_relay.rs`.

### Phase 4 — DNS + TUN

- DNS handling inside the kernel (fake-ip + upstream resolution): a UDP DNS
  server (`dns::DnsServer`) answers queries in one of two modes. **Fake-IP**
  hands out a synthetic `A` from a CIDR pool (`FakeIpPool`, default
  `198.18.0.0/16`), keeping a bidirectional `domain <-> ip` map so the routing
  path can recover the hostname from a fake IP (reverse lookup); `AAAA` gets an
  empty `NOERROR` so clients use the fake `A`. **Forward** relays the query to
  an upstream resolver over UDP and returns its answer verbatim. The DNS wire
  format is delegated to `hickory-proto`; pool allocation, mapping and mode
  selection are ours. Proven by `crates/learn-gripe/tests/dns.rs` (fake-IP
  synthesize + reverse, and forward via an independent fake upstream). Wiring
  fake-IP reverse lookup into the SOCKS connect path (route a connection to a
  fake IP by its original domain) is the next follow-up.
- TUN mode: read packets via the `tun` crate, route through `learn-gripe`
  outbounds, with leak-safe rollback. This is the highest-risk phase; keep
  rollback explicit. Still pending — it needs an OS TUN device and elevated
  privileges, so it cannot be exercised in the sandbox/CI and must land with an
  explicit apply + observe + rollback path.

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
