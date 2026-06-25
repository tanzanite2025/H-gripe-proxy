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
| Mihomo sidecar | No longer started or packaged for the supported path. `tauri-plugin-mihomo` is still a dependency (compatibility DTOs / dead code) and the binary is still in the tree; removal is the final phase, not yet done. |

`start_core()` now selects the outbound from the user's chosen node:
`OutboundMode::from_proxy()` maps a clash `proxies:` entry to the kernel
outbound, and `core/manager/outbound_select.rs` resolves the current selection
of the primary `select` group (following nested selectors, honoring the
persisted per-group selection) before falling back to Direct on any
unsupported/unresolvable case. This is a single global egress; per-connection
rule routing through `OutboundMode::Routed` is not wired into `start_core()`
yet. The OS system proxy now points at the kernel: `start_core()` binds the
mixed inbound to the same port the system proxy and PAC target
(`verge_mixed_port`, else clash `mixed-port`) instead of the unrelated
`socks-port`, so enabling the system proxy routes traffic through learn-gripe.
What the live path still does **not** do: TUN.

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
- Routing/rule engine: done for the in-kernel data plane. `learn-gripe` now has
  a `Router` (`OutboundMode::Routed`) that selects the outbound per connection
  from an ordered rule list (`DOMAIN` / `DOMAIN-SUFFIX` / `DOMAIN-KEYWORD` /
  `IP-CIDR` v4+v6 / `MATCH`), resolving to named outbounds plus the built-in
  `DIRECT` / `REJECT` policies, with a `fallback` for the no-match case (proven
  by `crates/learn-gripe/tests/router_outbound.rs`). `GEOIP` / `GEOSITE` need
  external mmdb / geosite data and are left for a follow-up.
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
- TUN mode — userspace stack, OS device binding, and Windows global IPv4
  default-route capture all landed (off by default); IPv6 capture is the next
  step (see below). The
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
  The macOS utun 4-byte packet-information header codec is the remaining TUN
  follow-up (deferred — Windows first).

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
