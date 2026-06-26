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
| learn-gripe kernel | MVP live. **Mixed inbound** on one port: SOCKS5 (no-auth CONNECT + UDP ASSOCIATE) and HTTP proxy (`CONNECT` tunnel + plain absolute-form requests) selected by peeking the first byte, relaying through the configured outbound via `tokio::io::copy_bidirectional`. Bound + started inside `CoreManager::start_core()`; lifecycle via `GripeHandle` (`local_addr`, graceful `shutdown`). New `RunningMode::Gripe`. End-to-end relay tests in `crates/learn-gripe/tests/socks5_relay.rs` and `crates/learn-gripe/tests/http_inbound.rs`. |
| Rust control plane | Mature. Validation, planning, projection artifacts, subscription pipeline, monitor paths, audit, telemetry, and frontend type surfaces are Rust-owned (see "Completed control-plane milestones"). |
| Mihomo sidecar | **Fully retired.** The binary is gone from the tree and from packaging (no tracked binary; `scripts/prebuild.mjs`, `tauri.conf.json`, `tauri.linux.conf.json` `externalBin`, and `.github/workflows/release.yml` reference only the `clash-verge-service` sidecar and local geodata, never Mihomo), **and the `tauri-plugin-mihomo` crate itself is now deleted.** Every former IPC command runs in-process (proxy delay test, connection close/disconnect, `ws_connections`/`ws_logs` streams, obfuscation stats, TLS fingerprint stats + rotation, `update_geo`/`upgrade_geo`, `upgrade_core`/`upgrade_ui` no-ops, the controller-transport probe, provider update/healthcheck), and the remaining live telemetry reads in `runtime_snapshot.rs` were migrated to in-process sources (real data where a source exists, honest `Default::default()` for kernel telemetry not yet emitted). With no caller left, the controller-API IPC client (`Mihomo` / `MihomoExt`, `Handle::mihomo()`, `sync_mihomo_controller_state()`, `probe_mihomo_ipc()`) was removed. The shared compatibility DTOs (`models::*` + their ts-rs TypeScript bindings) moved to the dedicated **`crates/clash-dtos`** crate (+ `clash-dtos` npm package); all Rust and frontend consumers point there. **No Mihomo-owned surface remains.** See Phase 5. |

`start_core()` now selects the outbound from the user's chosen node:
`OutboundMode::from_proxy()` maps a clash `proxies:` entry to the kernel
outbound, and `core/manager/outbound_select.rs` resolves the egress. In
**`rule` mode** the kernel runs the full per-connection rule router
(`OutboundMode::Routed`, built by `routed_outbound` from the generated runtime
config + the persisted per-group selection, fed `GeoLookup` / `RuleSetLookup` /
`ProcessLookup` providers); in **`global`/single-node mode** it resolves the
current selection of the primary `select` group (following nested selectors,
honoring the persisted per-group selection) before falling back to Direct on any
unsupported/unresolvable case. The OS system proxy now points at the kernel:
`start_core()` binds the mixed inbound to the same port the system proxy and PAC
target (`verge_mixed_port`, else clash `mixed-port`) instead of the unrelated
`socks-port`, so enabling the system proxy routes traffic through learn-gripe.
TUN mode is implemented end to end but **off by default** (see Phase 4): the
only piece not yet exercised on real hardware is true global default-route
capture (Windows v4/v6 is compile-checked, macOS capture is not yet written).

## Build vs adopt boundary

This section is the durable decision record for **what we implement ourselves and
what we build on top of vetted crates**. It exists so future work does not drift
back into either extreme (re-adopting a whole external kernel, or hand-rolling
cryptography). "Owning the kernel" means owning architecture, protocol logic,
routing, config, and lifecycle — it does **not** mean re-implementing TLS or
ciphers.

### We build ourselves (this is `learn-gripe`)

These are the parts that define the product and where we want full control:

- Inbound listeners and the connection accept/relay loop (SOCKS5 + HTTP on a
  mixed listener today; more later).
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
| Userspace IP/TCP stack (TUN) | `smoltcp` | Packet wire codec + per-flow TCP state machine; a hand-rolled TCP stack is a reliability minefield. We still own the orchestration (flow demux, bridging, back-pressure). |
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
  uTLS-style `client-fingerprint` ClientHello shaping is **partially landed**:
  the fork seeds the ClientHello cipher order and the TLS extension order from
  the configured fingerprint (Firefox / Safari use fixed per-browser seeds,
  Chromium-family and `random` keep rustls's per-handshake reshuffle), matching
  the two list fields JA3 keys on. Byte-level GREASE-value and record/extension
  padding modeling is **not** done and is deliberately deferred — it requires
  hand-editing the typed ClientHello and can only be verified with real-machine
  JA3 packet capture, which would push against the "do not hand-roll TLS" line.
  The `flow: xtls-rprx-vision` layer is done: it is a VLESS body
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

- HTTP/HTTPS inbound (`CONNECT` + plain proxy) alongside SOCKS5: done. The
  inbound is now a **mixed listener** (the SOCKS5 accept loop peeks the first
  byte — `0x05` is SOCKS5, anything else is an HTTP request line — so both
  protocols share one port, like the app's mixed-port). `CONNECT host:port`
  replies `200 Connection established` then tunnels raw bytes (the HTTPS path);
  a plain absolute-form request (`GET http://host/path`) is dialed at the
  origin after rewriting the request line to origin-form and dropping the
  hop-by-hop `Proxy-Connection` header, then relayed. Only the head is parsed
  in-crate; bodies/responses flow through `copy_bidirectional`. One forward
  target per connection (covers keep-alive to a single host). Proven by
  `crates/learn-gripe/tests/http_inbound.rs` (CONNECT tunnel, origin-form
  rewrite observed by the origin, and `502 Bad Gateway` on a rejected
  outbound).
- OS system-proxy → kernel listener — done. `start_core()` binds the mixed
  inbound to the canonical proxy port (`verge_mixed_port`, else clash
  `mixed-port`) — the exact value `core/sysopt.rs`, the PAC script
  (`utils/server.rs`), and the subscription/probe paths already target —
  instead of the unrelated `socks-port` it used before. Enabling the system
  proxy (global or PAC) now routes OS traffic through learn-gripe rather than a
  dead port.
- Node-aware outbound selection — done. `start_core()` reads the selected node
  from app runtime state and dials the matching outbound instead of always
  Direct. `OutboundMode::from_proxy()` (in `learn-gripe`) maps a clash
  `proxies:` entry to the kernel outbound (direct / reject / socks5 / ss /
  trojan / vmess / vless), erroring on protocols/sub-features without a data
  plane. `core/manager/outbound_select.rs` resolves the current selection of the
  primary `select` group — following nested selector groups and honoring the
  persisted per-group selection (`GLOBAL` in global mode, else the first
  selector; default to the group's first member) — then falls back to Direct
  (with a log) on any unresolvable/unsupported case rather than mis-routing.
  This delivers a single global egress through the selected node;
  per-connection rule routing via `OutboundMode::Routed` is a later step. The
  `from_proxy` dispatcher is covered by `learn-gripe` unit tests; the resolver
  by `outbound_select` unit tests (selection / nested groups / global mode /
  `url-test` + unimplemented-protocol fallback / cycle guard).
- **Shadowsocks** outbound — done. AEAD methods `aes-128-gcm`, `aes-256-gcm` and
  `chacha20-ietf-poly1305` over plain TCP, wired as `OutboundMode::Shadowsocks`.
  The master key is derived with OpenSSL `EVP_BytesToKey` (MD5) and the
  per-session subkey with HKDF-SHA1 (`"ss-subkey"`); each direction sends a salt
  then length-prefixed AEAD chunks (`AEAD(len)(2+16) | AEAD(payload)(len+16)`)
  with a 12-byte little-endian counter nonce. Crypto is delegated to vetted
  RustCrypto crates (`aes-gcm`, `chacha20poly1305`, `md-5`, `sha1`); only the
  key schedule (HKDF cross-checked against RFC 5869, HMAC against RFC 2202) and
  on-wire framing are assembled in-crate. Legacy stream ciphers, the
  `2022-blake3-*` methods and SIP003 plugins are rejected rather than
  mis-framed. **Shadowsocks UDP egress** is landed too (`ShadowsocksUdp`): each
  datagram is framed independently as `salt | AEAD(subkey, nonce=0, socks5_addr |
  payload)` with a fresh per-packet salt and `subkey = HKDF-SHA1(master, salt,
  "ss-subkey")` — no length-prefixing or nonce counter, unlike the TCP stream —
  and it plugs into the SOCKS5/TUN UDP relay as a `UdpEgress::Shadowsocks` variant
  (`resolve_udp_egress`, `run_ss_egress` / `run_udp_ss`). End-to-end relay tests
  for all three ciphers (plus a multi-chunk payload) against an independent fake
  SS server in `crates/learn-gripe/tests/shadowsocks_outbound.rs` (TCP) and
  `shadowsocks_udp.rs` (per-packet UDP).

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
- Routing/rule engine: done for the in-kernel data plane and **wired into
  `start_core()` rule mode** (`resolve_outbound` → `routed_outbound`).
  `learn-gripe`'s `Router` (`OutboundMode::Routed`) selects the outbound per
  connection from an ordered rule list, resolving to named outbounds plus the
  built-in `DIRECT` / `REJECT` policies, with a `fallback` for the no-match case.
  The matcher set now covers the standard Mihomo rule vocabulary:
  - Destination: `DOMAIN` / `DOMAIN-SUFFIX` / `DOMAIN-KEYWORD`, `IP-CIDR`(v4+v6),
    `GEOIP`, `GEOSITE`, `IP-ASN`, `DST-PORT`, `NETWORK` (tcp/udp), `RULE-SET`.
  - Source: `SRC-IP-CIDR`(v4+v6), `SRC-IP-ASN`, `SRC-PORT`, `SRC-IP-RULE-SET`,
    `PROCESS-NAME` / `PROCESS-PATH`, `UID`.
  - Combinators: `AND` / `OR` / `NOT` (arbitrarily nested sub-rules), plus `MATCH`.
  Data-backed matchers query app-provided providers through narrow traits the
  kernel never fetches itself — `GeoLookup` (GeoIP / GeoSite / ASN mmdb, via
  `RuleGeoData`), `RuleSetLookup` (local rule-providers), and `ProcessLookup`
  (OS socket→process / UID). A matcher whose data or source is absent (no
  geodata, unknown rule-set, unresolvable source socket) is skipped rather than
  mis-matched, so configs stay backward-compatible. Proven by
  `crates/learn-gripe/tests/router_outbound.rs` plus per-matcher unit tests in
  `router.rs` (each `router_routes_*_rule` test drives a real `Router::select*`)
  and the app-side `parse_router_rule` / `parse_matcher` parser tests in
  `outbound_select.rs`. (Implemented across PRs #412–#423.)
- UDP relay (SOCKS5 UDP ASSOCIATE): the inbound now answers `UDP ASSOCIATE`,
  binds a relay socket, and relays SOCKS5-wrapped datagrams to/from remote hosts;
  the association lives as long as its TCP control connection (RFC 1928). Each
  destination owns one **egress task** fed by a bounded channel, with the route
  resolved per datagram. Egress is now either **Direct** (a plain OS UDP socket)
  or **proxy-tunnelled** through the protocol's UDP framing over the existing
  (TCP/TLS/REALITY) outbound stream:
  - Trojan: `SOCKS5-addr | len(2 BE) | CRLF | payload` per datagram (command 0x03).
  - VLESS: command 0x02, no Vision addon, `len(2 BE) | payload` per datagram.
  - VMess: command 0x02, one AEAD body chunk per datagram (boundaries preserved).
  - Shadowsocks: a UDP socket to the SS server, each datagram framed as
    `salt | AEAD(nonce=0, socks5_addr | payload)` with a fresh per-packet salt.
  `Direct` / `Trojan` / `VLESS` / `VMess` / `Shadowsocks` / `Routed` accept the
  association; `Reject` and an upstream SOCKS5 proxy refuse it with
  `REP_CMD_NOT_SUPPORTED`, and a datagram routed to a non-UDP-capable outbound is
  dropped rather than leaked. Proven by `crates/learn-gripe/tests/udp_relay.rs`
  (Direct) and `trojan_udp.rs` / `vless_udp.rs` / `vmess_udp.rs` /
  `shadowsocks_udp.rs` (each protocol's tunnel, via independent reverse fake
  servers, over `none` / `tls` security and `Routed`).

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
  synthesize + reverse, and forward via an independent fake upstream).
- Fake-IP routing: the SOCKS inbound (both `CONNECT` and per-datagram UDP
  `ASSOCIATE`) rewrites a target that is a fake IP back to its original domain
  (`dns::unmap_fake_ip`) before routing, so a connection to a synthetic IP is
  routed/dialed by the hostname the rules were written against. Wired via
  `GripeKernel::start_with_fake_ip(config, pool)` (the plain `start` is
  unchanged, so a kernel without DNS behaves exactly as before). Proven by
  `crates/learn-gripe/tests/fakeip_routing.rs`: the DNS server mints two fake
  IPs in the same `/16` for two domains, and connections to them reach
  *different* tagged outbounds purely by hostname.
- TUN mode — userspace stack, OS device binding, and Windows global IPv4 **and
  IPv6** default-route capture all landed (off by default). The
  device-agnostic core is in `crates/learn-gripe/src/tun.rs` (`serve_tun`): it
  consumes/produces raw IP frames over two channels, terminates IPv4/IPv6
  **TCP** flows in a userspace stack (smoltcp adopted purely as the IP/TCP
  wire-codec + per-flow state machine, like rustls for TLS), and relays each
  flow through the normal `OutboundMode` pipeline (with fake-IP unmap), owning
  the flow demux, back-pressure and close handling ourselves. Proven by
  `crates/learn-gripe/tests/tun_inbound.rs`, where an independent second smoltcp
  stack drives real TCP handshakes (small + 256 KiB multi-segment) through an
  in-memory TUN pipe into `serve_tun` and gets the bytes back from a `Direct`
  echo outbound — real bytes across two real TCP state machines, no OS device
  needed. The device pump (`serve_tun_device`) is also landed: it adapts a
  byte-stream device with the `tun` crate's "one IP packet per read/write"
  contract to the `serve_tun` channels (the exact glue an OS binding calls),
  tested end-to-end over a mock packet device via `tun_device_pump_relays_tcp_flow`.
  The OS device binding is also landed (compile-verified, **off by default**):
  `src-tauri/src/core/manager/tun_inbound.rs` creates a real OS TUN interface via
  the `tun` crate (wintun/`/dev/net/tun`/utun), brings it up with an address, and
  feeds it to `serve_tun_device`, gated behind `enable_tun_mode` in `start_core()`
  with every privileged mutation recorded on a `RollbackStack` undone in reverse
  on stop (and on `Drop`). The OS binding now feeds `serve_tun_device` a
  `DnsMode::FakeIp` (no longer `None`), so the app's TUN path answers DNS and
  relays UDP in-stack — not just TCP; the interface/gateway address is held out
  of the pool via the new `FakeIpPool::reserve` so a domain is never mapped onto
  the gateway. **DNS over TUN** is landed too: `serve_tun` intercepts
  UDP datagrams to port 53 and answers them in-stack through the kernel's existing
  DNS logic (`answer_query` / fake-IP allocation), building the reply frame with
  smoltcp's wire codec — so a client resolves names to fake IPs over the TUN and
  then opens TCP to those (already relayed + unmapped, sharing the same pool). This
  is the prerequisite that lets a global default-route capture work without
  black-holing name resolution. Proven by `tun_answers_dns_query_from_fake_ip_pool`
  (an A query fed into the stack comes back as a fake-IP answer, no OS device, no
  upstream). **General UDP over TUN** is landed too: every non-DNS UDP datagram is
  relayed through the normal `OutboundMode` pipeline via a NAT session table keyed
  by the UDP 5-tuple (`relay_udp` / `run_udp_session`), reusing the SOCKS5 UDP
  egress primitives (`resolve_udp_egress`, Direct socket / proxy-tunnel framing for
  Trojan/VLESS/VMess) and rewriting each reply back into an IP frame with swapped
  endpoints; idle sessions are reaped on a timeout, fake IPs are unmapped for
  routing, and destinations with no UDP egress (`Reject`, upstream SOCKS5) are
  dropped rather than leaked. Proven by `tun_relays_udp_datagram_through_direct_outbound`
  (a UDP datagram out a real OS socket to an echo server, reply rewritten back to
  the client). **Windows global default-route capture + DNS redirect** is now wired
  into `TunInbound::start` behind the existing `RollbackStack`, gated on
  `enable_tun_mode` (off by default) and applied only when the selected outbound is
  a single fixed-server proxy (`OutboundMode::supports_global_capture`): it resolves
  the proxy server endpoint(s) and pins each to the physical gateway with a `/32`
  bypass route (`OutboundMode::direct_dial_endpoints`), adds `0.0.0.0/1` + `128.0.0.0/1`
  routes through the TUN (more specific than the untouched `0.0.0.0/0` default, so
  rollback is a clean delete), points the resolver at the in-stack fake-IP DNS, then
  re-reads the route table to confirm the capture took effect — rolling everything
  back and failing start if it did not. `Direct`/`Reject`/`Routed` fall back to the
  on-link subnet (they would loop). The route-parsing/command-building logic has unit
  tests; the actual `route`/`netsh` mutations need admin and a real default route, so
  they are **compile-checked only and must be validated on a real Windows machine**.
  This stays the highest-risk phase.

  **Windows IPv6 default-route capture** is now landed too (`install_global_capture_v6`),
  closing the prior IPv4-only leak gap — without it, AAAA-reachable destinations
  egressed over the physical adapter. It mirrors the IPv4 path inside the same
  `install_global_capture` / `RollbackStack` flow (same `supports_global_capture`
  gate) and is **purely additive**: a host with no IPv6 default route is left
  untouched.
  - Assigns the TUN an `fd00::/8` ULA gateway (`fd00::1`) via `netsh` for an
    on-link v6 next-hop (the `tun` crate can't set a v6 address on Windows), the
    analogue of `198.18.0.1`.
  - Bypass: pins each resolved IPv6 proxy-server address with a `/128` via the
    physical v6 default (interface + gateway parsed from
    `netsh interface ipv6 show route`, handling on-link defaults with no gateway),
    the v6 analogue of the `/32` bypass.
  - Capture: adds `::/1` + `8000::/1` through the TUN (more specific than the
    untouched `::/0` default, so rollback is a clean delete), the analogue of the
    `0.0.0.0/1` + `128.0.0.0/1` split.
  - Observe: re-reads the v6 route table to confirm the `::/1` split took effect,
    rolling back and failing start otherwise.
  - The route-building / table-parsing helpers (`parse_default_gateway_v6`,
    `capture_routes_present_v6`, `v6_route_*_args`, `v6_*_address_args`) have unit
    tests; the `netsh` mutations remain **compile-checked only and must be
    validated on a real Windows machine with admin**.
  The **macOS/iOS utun packet-information header** is now handled: utun always
  prepends a 4-byte address-family header at the kernel (it cannot be disabled),
  and `serve_tun_device` expects raw L3, so the OS binding enables the `tun`
  crate's packet-information handling on macOS/iOS (it strips that header on read
  and prepends `AF_INET`/`AF_INET6` on write) while keeping `IFF_NO_PI` on Linux.
  The earlier code disabled it on all three, which left the 4-byte header in the
  frames on macOS. The binding also stops forcing the `clash-verge` interface
  name on macOS/iOS (utun names must be `utunN`, so the kernel assigns one).
  macOS global default-route capture (the v4/v6 route + DNS takeover, currently
  Windows-only) is the remaining TUN follow-up; the macOS device itself now comes
  up correctly. All of this is compile-verified only and needs validation on a
  real Mac.

### Phase 5 — Delete Mihomo

Only after the supported default paths above run on `learn-gripe`:

- **Done — binary + packaging.** The Mihomo binary is no longer tracked in the
  tree and is not referenced by any packaging or release path (`prebuild.mjs`,
  `tauri.conf.json`, `tauri.linux.conf.json` `externalBin`, `release.yml`). No
  Mihomo sidecar/service startup plumbing remains (`lifecycle.rs` explicitly
  retires the Mihomo service core startup in favor of the Rust runtime path).
- **Done — verified.** `git ls-files` shows no Mihomo binary outside the
  `crates/tauri-plugin-mihomo/` source crate, and the build/CI scripts pull only
  the `clash-verge-service` sidecar and local geodata.
- **Done — in-process geo update (`update_geo` / `upgrade_geo`).** `core/geo_update.rs`
  downloads the GeoIP, GeoSite, and ASN databases (honoring the profile's
  `geox-url` overrides, falling back to the MetaCubeX `meta-rules-dat` releases)
  through the app's `NetworkManager` (proxy-aware), validates each download by
  format (`maxminddb` opens MMDBs; `.dat` files are checked for the V2Ray
  protobuf shape), then atomically replaces the local files in the app data dir.
  `CoreManager::update_geo()` wraps it and restarts the kernel when running so
  the router reloads through `GeoLookup` (the same boundary config changes use).
  `runtime_bridge.rs` `update_runtime_geo`/`upgrade_runtime_geo` now call the
  in-process path instead of the Mihomo IPC client. Unit tests cover URL
  resolution, format validation, and the temp-target/atomic-replace path.
- **Done — in-process TLS fingerprint stats + rotation (`get_tls_fingerprint_stats`
  / `force_tls_rotation`).** The kernel's `obfuscation` module (which already
  counts TLS ClientHello fingerprint-shaped handshakes) now also tracks a
  per-fingerprint usage map and exposes `force_rotation()`. `CoreManager`
  shapes that snapshot into the Mihomo `TLSFingerprintStats` payload
  (`tls_fingerprint_stats_from_obfuscation`) so the telemetry consumer parses it
  unchanged, and `force_runtime_tls_rotation()` records an operator-requested
  rotation in-process. learn-gripe re-rolls `random`/`randomized` ClientHello
  cipher order per dial and pins concrete fingerprints to per-proxy
  `client-fingerprint` config, so a forced rotation has no on-the-wire effect; it
  is counted for telemetry parity with the former Mihomo controller. Both
  `runtime_snapshot.rs` (stats read) and `runtime_bridge.rs` (`force_runtime_tls_rotation`)
  now use the in-process path instead of the Mihomo IPC client. Unit tests cover
  per-fingerprint usage counting and the forced-rotation counter.
- **Done — controller-transport probe + `upgrade_core`/`upgrade_ui`.** The control
  plane runs fully in-process (the kernel is compiled into the app over
  `learn-gripe`), so there is no external Mihomo controller socket to probe:
  `read_runtime_controller_transport()` now returns a fixed `in-process` label
  instead of reading the dead IPC client's protocol, and the now-unused
  `controller_transport_label`/`Protocol` plumbing in `kernel_runtime` is removed.
  `upgrade_core`/`upgrade_ui` became in-process no-ops that record the request for
  history parity — the kernel and dashboard ship with the app and are upgraded
  through the application updater, so there is no external binary or panel to
  download. All three dropped their `Handle::mihomo()` calls.
- **Done — in-process provider update + health-check (`update_proxy_provider`,
  `update_rule_provider`, `healthcheck_proxy_provider`).** The kernel never owned
  provider data: rule providers are consumed from local files by
  `core/rule_engine`, proxy providers parsed from local files by
  `runtime_snapshot`. The former Mihomo `update` call only re-fetched the remote
  list into that local file, so `core/provider_update.rs` does exactly that
  in-process — same shape as the geo update: download the upstream list through
  the proxy-aware `NetworkManager` (with a direct fallback), validate it parses
  (reject empty/garbage so a bad fetch never blanks a working file), then
  atomically swap it in via a `.download.tmp` sibling + rename. File/inline
  providers have nothing remote to fetch and succeed as a no-op.
  `healthcheck_proxy_provider` resolves the provider's nodes to kernel outbounds
  and probes each concurrently with the in-process `learn_gripe::measure_delay`,
  persisting per-node delays through `runtime_snapshot` (replacing the controller
  `/providers/proxies/{name}/healthcheck` call). `CoreManager` wraps all three and
  restarts the kernel after an update so the rule engine / snapshot re-read the
  new file; `runtime_bridge.rs` now calls the in-process path and dropped its last
  three `Handle::mihomo()` calls. Unit tests cover proxy/rule list parsing, the
  empty-list rejection, and the temp-target/atomic-replace path.
- **Done — the `tauri-plugin-mihomo` crate is deleted.** The remaining live
  controller-API IPC reads in `core/runtime_snapshot.rs` (`get_proxies`,
  `get_rules`, `get_version`, `get_base_config`, `get_tls_fingerprint_stats`,
  `get_connections`/`get_egress_status`, plus the kernel-telemetry reads
  `get_dns_metrics`, `get_engine_stats`, `get_perf_stats`, `get_buffer_pool_stats`,
  `get_xdp_status`, `get_hot_reload_status`, `get_rule_traffic`) were the last
  callers of the IPC client. They now run in-process: proxies/rules/version/
  base-config are rebuilt from the runtime config (`build_proxies_from_runtime_config`,
  `build_rules_from_runtime_config`, `build_base_config_from_runtime_config`), TLS
  stats from the in-process obfuscation snapshot, connections/egress from the
  existing in-process sources; the kernel-telemetry reads return honest
  `Default::default()` values (the kernel does not yet emit them, and the old IPC
  reads already failed at runtime since no controller socket exists — so this is
  not a regression). With no caller left, the IPC client (`Handle::mihomo()`,
  `sync_mihomo_controller_state()`, `probe_mihomo_ipc()`) was removed. The shared
  compatibility DTOs (`models::*` + their ts-rs TypeScript bindings, consumed by
  ~22 `src-tauri` files and 13 frontend files) were moved into a new dedicated
  **`crates/clash-dtos`** crate (pure DTOs + ts-rs export + the `clash-dtos` npm
  package); all Rust consumers now `use clash_dtos::*` and all frontend imports
  point at the `clash-dtos` package. `crates/tauri-plugin-mihomo` is deleted
  outright. **No Mihomo-owned surface remains.**
- **Done — kernel telemetry now reports honestly instead of zeroed defaults.**
  The crate-deletion PR temporarily wired the kernel-telemetry reads to
  `Default::default()`, which made the diagnostics panel mark fabricated zeros as
  "available". Those reads now split by whether the userspace Rust kernel has a
  real source:
  - **Real in-process data:** `EngineStats` (`active_connections`/`tracked_conns`)
    comes from the live conntrack table (`runtime_live_connection_count()`);
    `HotReloadStatus` reports `rule_version` as a content hash of the active
    `rules` + `rule-providers` (`rule_version_from_runtime_config()`, stable and
    changes when the rule set does), `protected_conns` from conntrack, and
    `xdp_loaded=false`; `XDPStatus` is `loaded=false`/`enabled=false` — the
    genuine state of a userspace kernel with no eBPF datapath.
  - **No honest source → `Err` → panel shows "不可用":** `PerfStats` (Go-runtime
    goroutines/GOGC/GC/heap) and `BufferPoolStats` (no custom size-classed pool;
    uses tokio buffers). Each `refresh_*_result()` returns
    `anyhow::bail!` with the reason; the frontend's `.catch(() => null)` renders
    the "不可用" chip instead of fake values. `runtime_dns_warmup` became an honest
    no-op success (nothing to warm in an on-demand resolver) rather than surfacing
    the DNS-metrics read error. The panel description text dropped the stale
    "Mihomo" wording. (`DnsMetrics` later gained a real in-process source in TUN
    mode — see below.)

- **Done — per-rule traffic now reports real in-process data.** Every tracked
  connection already records the rule type/payload the router matched
  (`ConnMeta.rule`/`rule_payload`) plus its live upload/download counters, so
  `RuleTrafficSnapshot` is aggregated directly from the conntrack table
  (`rule_traffic_from_kernel()` sums bytes + connection counts by `(rule type,
  payload)`) instead of returning `Err`. This is the same shape the retired Go
  controller reported over `/engine/rules/traffic`, now sourced in-process with
  no new kernel bookkeeping. Connections no rule router matched (empty rule) are
  skipped; when the kernel is not running the read still returns `Err` so the
  panel honestly shows "不可用". `RuleTrafficSnapshot` thus moves out of the
  no-honest-source group above into real in-process data.

- **Done — DNS cache/query metrics now report real in-process data in TUN mode.**
  The only resolver the Rust kernel answers itself is the in-stack fake-IP
  answerer on the TUN datapath (`build_fake_ip_response` in `learn-gripe/dns.rs`);
  outside TUN mode DNS is forwarded verbatim with no instrumentation. That
  answerer now carries a lock-free `DnsStats` (atomic counters, `Relaxed`,
  incremented on the hot path with no extra allocation): total accepted datagrams,
  `A`/`AAAA`/other questions, `A`-question cache hits (the domain already had a
  pool mapping — detected via `FakeIpPool::has_domain` before allocating), and
  parse/serialize errors. `DnsMode::fake_ip()` returns the shared `Arc<DnsStats>`,
  `TunInbound` holds it plus the pool, and `TunInbound::dns_stats()` snapshots the
  counters together with the live fake-IP cache size (`FakeIpPool::len()`).
  `CoreManager::runtime_dns_stats()` exposes the snapshot (parallel to
  `runtime_connections()`), returning `None` unless a TUN inbound is live.
  `refresh_runtime_dns_metrics_result()` shapes it via `dns_metrics_from_stats()`:
  the cache section (hit/miss/size/hit-rate) and query section (total/success =
  total−errors/failed) carry real data, while per-upstream server stats and
  pollution/trust analysis have no honest in-process source and
  stay empty (the panel hides those sections). There is no upstream round-trip to
  time, so latency stays 0. Outside TUN mode the read returns `Err` and the panel
  honestly shows "不可用". Unit tests cover the counter increments
  (`dns_stats_count_queries_hits_and_cache_size`) and the DTO mapping
  (`dns_metrics_map_cache_hits_misses_and_query_totals`). `DnsMetrics` thus moves
  out of the no-honest-source group into real in-process data (TUN mode).

- **Done — DNS recent-query history now reports real in-process data in TUN mode.**
  The in-stack fake-IP answerer already observes every question it serves, so it
  now records each one in a bounded ring (`DnsStats.recent`: a `Mutex<VecDeque<
  DnsRecentQuery>>` capped at `RECENT_QUERY_CAP = 64`, oldest evicted FIFO). Each
  entry keeps only the wire-level facts the answerer actually saw — domain
  (root dot stripped), record type (`A`/`AAAA`/other), success (`NotImp` for
  unsupported types counts as failure), and a `unix_ms` timestamp captured at
  answer time. `DnsStats::snapshot()` reads the ring newest-first into
  `DnsStatsSnapshot.recent`; `dns_metrics_from_stats()` maps those to the
  `DnsQueryEvent` DTO (`server = "fake-ip (in-stack)"`, `protocol = "udp"`,
  `latency_us = 0`). Routing fields (`proxy_name`/`proxy_chain`/`egress`/`rule`/
  `rule_payload`) are **unknown at answer time** and stay `None` — the synchronous
  fake-IP answerer has no routing context or upstream round-trip — so they are not
  fabricated. Outside TUN mode the read returns `Err` and the panel honestly shows
  "不可用". Unit tests cover the ring (capture/order/eviction via
  `dns_stats_count_queries_hits_and_cache_size`) and the DTO mapping
  (`dns_metrics_map_cache_hits_misses_and_query_totals`). Only per-upstream server
  stats and pollution/trust analysis remain without an honest in-process source.

- **Done — DNS servers section now reports real in-process data in TUN mode.**
  In fake-IP TUN mode the in-stack answerer is the *sole* DNS server: every
  question is answered synchronously from the local fake-IP pool, with no
  configured upstream resolver to enumerate. So instead of per-upstream stats
  (which have no honest source here), `dns_metrics_from_stats()` surfaces one
  honest `DnsServerStats` entry for it whenever at least one query has been served
  (`total > 0`): `server = "fake-ip (in-stack)"`, `queries = total`,
  `successes = total − errors`, `failures = errors` — all derived from the same
  counters, no new kernel instrumentation. `last_query` is the newest recorded
  question's timestamp (from the recent ring); `avg_latency_us = 0` (synchronous
  in-memory answer, no round-trip) and `last_error = None` (parse/serialize
  failures are not attributable to a specific upstream). Before any query the
  `servers` vec stays empty. Outside TUN mode the read returns `Err` and the panel
  honestly shows "不可用". The `dns_metrics_map_cache_hits_misses_and_query_totals`
  test asserts the entry mapping. Only pollution/trust analysis now remains without
  an honest in-process source (it needs upstream-resolver modeling and
  leak/poisoning detection the userspace kernel does not perform).

- **Done — DNS resolution-path trust now reports real in-process data in TUN
  mode.** `dns_metrics_from_stats()` is only ever reached with a live in-stack
  snapshot (the read returns `Err` outside TUN mode), so the resolution path is
  always the fake-IP answerer: every question is answered locally and **no
  plaintext DNS leaves the host**, while the real name resolution happens at the
  proxy egress over the encrypted tunnel. That is a genuine, honest privacy
  property, so the `trust` section now carries one `DnsServerClassification` for
  the in-stack answerer (`address = "fake-ip (in-stack)"`, `protocol = "fakeip"`,
  `trust_level = "maximum"`, `encrypted = true` — end-to-end via the tunnel), with
  `total = 1`, `encrypted = 1`, `by_trust_level = {"maximum": 1}` and
  `leak_risk_score = 0.0` (no third-party resolver is ever queried). The property
  holds even before the first query, so it is not gated on query count. Outside
  TUN mode the read returns `Err` and the panel honestly shows "不可用". The
  `dns_metrics_map_cache_hits_misses_and_query_totals` test asserts the
  classification. **Pollution analysis is now the only DNS section without an
  honest in-process source** — it would need to compare answers against a trusted
  baseline (DoH/DoT cross-check or known-good lists), which the fake-IP answerer
  does not perform — so it stays empty and the panel hides it.

### Continuous verification (CI)

`.github/workflows/ci.yml` gates every push to `main` / `devin/**` and every PR
to `main`, locking the completed kernel + app surface against silent regressions
(the repo previously ran Rust tests only locally, so any of the matchers /
protocols above could regress unnoticed):

- **kernel job** (`ubuntu-latest`): `cargo fmt -p learn-gripe --check`,
  `cargo clippy --all-targets -p learn-gripe`, and `cargo test -p learn-gripe`
  (the full unit + protocol / router / dns / tun integration suite — pure logic,
  so it runs cross-platform on Linux).
- **app job** (`windows-latest`, mirroring `release.yml`'s toolchain):
  `cargo fmt` / `clippy` / `cargo test --lib --no-run` for `clash-verge-optimized`
  with the `clippy` feature (skips `tauri_build` / frontend). The app test binary
  cannot *run* in CI (it needs platform GUI/WinRT bindings at runtime), but
  compiling it catches breaking changes to the app-side router / parser bridge.

The release packaging workflow (`release.yml`) is unchanged and still
tag-triggered. (Added in PR #424.)

## Next step

The kernel data plane is now feature-complete for the standard config surface —
all inbound/outbound protocols, the full routing matcher set, DNS, and TUN are
implemented and (as of PR #424) CI-gated. The remaining work is **validation and
polish, not new kernel features**, and the highest-value next step is the one
thing CI cannot cover:

1. **Real-hardware TUN validation (highest priority — needs your machines).**
   Enable `enable_tun_mode` on a real Windows box (admin) and a real Mac and
   confirm global default-route capture actually takes over v4/v6 + DNS without
   stranding the network. The Windows v4/v6 `route` / `netsh` mutations are
   compile-checked only; **macOS global capture is not yet written** — it is the
   one missing TUN feature and should be implemented + validated together on a
   real Mac (the macOS utun device itself already comes up). This is the only
   thing standing between TUN mode and shipping it on by default.
2. **DNS pollution analysis (last telemetry gap — optional).** Every other
   diagnostics panel now has an honest in-process source; pollution/trust
   comparison is the only section left empty because it needs a DoH/DoT
   cross-check the userspace kernel deliberately does not perform. Closing it
   means adding a trusted-baseline resolver path (a real feature, not a stub),
   or it stays intentionally hidden.
3. **uTLS GREASE / padding (deferred — not recommended).** Byte-level
   ClientHello GREASE values and record/extension padding remain; they need
   real-machine JA3 capture to verify and push against the "do not hand-roll
   TLS" boundary.

Recommendation: do (1) when Windows + macOS hardware is available to test on;
(2) and (3) are optional and can stay deferred.

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
