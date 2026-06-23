# Go-to-Rust migration roadmap

This is the single source of truth for the Go/Mihomo to Rust migration. The old Phase 8 standalone plan was folded back into this roadmap so status, safety gates, and next batches do not drift across two documents. Use PRs and git history for implementation archaeology.

## Goal

Move app control-plane ownership out of the Go/Mihomo sidecar and into the Tauri Rust layer, while keeping production packet forwarding safe until each data-plane replacement step has evidence, opt-in gates, rollback, and hold history.

Target ownership chain:

```text
App registry / policy / node pool / DNS / security profile
  -> Rust-owned runtime plan
  -> Rust-generated projection artifact
  -> explicit gate / audit / rollback boundary
  -> Mihomo production data-plane apply bridge
  -> Rust-observed runtime state
```

## Current state

Status is current through bounded DNS fallback-filter geoip execution. The migration has now moved past the earlier gate-only detour and has real bounded Rust data-plane implementations for DNS, adapter policy, loopback forwarding, HTTP CONNECT, encrypted framing, scoped Shadowsocks AEAD execution, AEAD canary evidence, multi-chunk encrypted TCP session evidence, bounded transparent IPv4/TCP packet execution, wider fallback retirement manifest/checkpoint execution, default-scope closeout ownership reconciliation, loopback-only SOCKS5 UDP ASSOCIATE datagram forwarding, deterministic one-domain DNS fake-ip allocation, bounded fallback-filter domain/ipcidr evaluation, bounded nameserver-policy exact/suffix dispatch, bounded SOCKS5 username/password negotiation, bounded SOCKS5 TCP CONNECT forwarding, bounded SOCKS5 BIND forwarding, bounded SOCKS5 UDP two-fragment reassembly, and bounded DNS fallback-filter geoip/geoip-code evaluation. The old `rust-data-plane-hardening-*` IPC commands remain safety metadata only; ownership claims below are limited to the explicitly named bounded paths.

| Area | State | Boundary |
| --- | --- | --- |
| Rust control plane | Mature | Validation, planning, projection artifacts, audit, telemetry, and frontend type surfaces are Rust-owned enough to support real data-plane work. Stop adding more read-only gate-only PRs here. |
| DNS runtime | Bounded opt-in parity path in progress | Rust now synthesizes a dns/hosts runtime patch, probes supported resolvers, executes bounded one-domain fake-ip allocation, fallback-filter domain/ipcidr evaluation, fallback-filter geoip/geoip-code evaluation, and nameserver-policy exact/suffix dispatch with rollback/leak evidence, blocks unsupported fallback-filter full GeoIP database/upstream and nameserver-policy geosite/wildcard execution, and applies supported patches through an explicit opt-in bridge with rollback. Mihomo still owns default DNS until canary evidence passes. |
| Adapter / egress runtime | Bounded opt-in parity path in progress | Rust now chooses DIRECT/REJECT/proxy-group adapter targets from app runtime state, validates candidate protocol compatibility, patches proxy-groups/rules through an explicit opt-in bridge, and keeps Mihomo fallback/rollback. |
| Protocol forwarding | Unsupported protocol expansion in progress | Rust now owns loopback TCP/HTTP forwarding, DIRECT/REJECT policy, bounded remote transport, HTTP CONNECT tunneling, encrypted framing preflight, scoped Shadowsocks AEAD adapter execution, AEAD canary evidence, multi-chunk encrypted TCP session evidence, bounded SOCKS5 UDP ASSOCIATE datagram forwarding, bounded SOCKS5 username/password negotiation, bounded SOCKS5 TCP CONNECT forwarding, bounded SOCKS5 BIND forwarding, and bounded SOCKS5 UDP two-fragment reassembly. Mihomo still owns VMess/VLESS/Trojan, Shadowsocks UDP/plugin transports, SOCKS non-loopback UDP plus fragment queues/timeouts, TUN packet capture, and default forwarding. |
| TUN / system proxy | Bounded Rust transparent routing execution in progress | Rust now owns explicit off/system-proxy/TUN route-mode planning, OS system-proxy apply through the Sysopt/sysproxy path, TUN config/restart apply through the existing backend, rollback records, rollback apply, and bounded transparent IPv4/TCP packet parsing/execution evidence. Mihomo/service still owns system-wide packet capture and transparent forwarding defaults. |
| Mihomo fallback retirement | Bounded closeout complete | Rust now writes a wider execution manifest, emergency rollback checkpoint, default data-plane closeout manifest, SOCKS UDP associate evidence for bounded loopback scopes, SOCKS UDP two-fragment evidence for bounded loopback scopes, fake-ip allocation evidence for one-domain DNS scope, fallback-filter domain/ipcidr evidence for one-answer DNS scope, fallback-filter geoip/geoip-code evidence for one-answer DNS scope, and nameserver-policy exact/suffix evidence for one-domain DNS scope. Unsupported SOCKS non-loopback UDP plus fragment queues/timeouts, unsupported DNS geosite/wildcard/policy-cache/upstream semantics, unsupported encrypted protocols, OS route install, packet capture, transparent proxy defaults, and full Mihomo binary removal remain fallback-owned. |
| Next real batch | `unsupported-protocol-and-packet-capture-implementation` | Continue with one remaining unsupported Mihomo-owned protocol, DNS policy, or packet-capture blocker and implement a bounded Rust execution path with canary, rollback, hold, and fallback evidence. |

## Acceleration plan

Course correction: the previous roadmap drifted into dozens of IPC/readiness gates. That is no longer useful. From this point forward, roadmap progress is measured by shipped data-plane capability, not by another `*_guard`, `*_dry_run`, or `*_readiness` wrapper.

### Hard stop on gate-only PRs

- Do not create another PR whose only product change is a new read-only evidence/gate command.
- A safety gate may be included only when it protects a real implementation in the same PR.
- Every migration PR must name the concrete Mihomo-owned behavior it reduces: DNS runtime, adapter egress, protocol forwarding, TUN/system proxy, fallback dependency, or removal of Go/Mihomo artifacts.
- Prefer 4-6 large implementation PRs over any new long sequence of numbered gates.

### Real fast-track sequence

This table is the authoritative batch map. Completed rows are real implementation PRs, not synthetic gates. Future rows should stay large enough to retire meaningful Mihomo surface area.

| Order | Batch | Status | Required implementation / evidence | Default impact |
| --- | --- | --- | --- | --- |
| 1 | `rust-dns-runtime-parity` | Complete | Rust-owned dns/hosts patch synthesis, resolver/upstream selection, controlled resolver probe, unsupported fake-ip/fallback-filter/nameserver-policy blockers, explicit opt-in apply, and one-switch rollback. | Opt-in only; Mihomo remains default DNS until canary evidence passes. |
| 2 | `rust-adapter-egress-parity` | Complete | Rust-owned DIRECT/REJECT/proxy-group target decisions, adapter candidate compatibility checks, explicit opt-in proxy-groups/rules runtime patching, and one-switch rollback. | Opt-in for supported profiles only; Mihomo remains protocol/forwarding fallback. |
| 3 | `rust-protocol-forwarding-subset` | Complete | Rust-owned loopback TCP/HTTP accept loop, bidirectional byte forwarding, connection/session accounting, smoke evidence, stop/rollback surface, and Mihomo fallback for unsupported protocols. | Capped canary only after DNS + adapter parity. |
| 4 | `rust-tun-system-proxy-parity` | Complete | Rust-owned off/system-proxy/TUN route-mode decision, explicit opt-in apply, OS system-proxy path, TUN config/restart bridge, rollback record, and rollback apply. | No broad default until platform rollback passes. |
| 5 | `rust-runtime-real-canary` | Complete | Bounded canary evidence across loopback DNS, Rust protocol forwarding, TUN/system-proxy route preflight, fallback readiness, and persisted evidence.yaml. | Limited default for canary profile. |
| 6 | `mihomo-fallback-retirement-execution` | Complete | Scoped execution manifest plus emergency rollback checkpoint for the bounded canary scope; unsupported fallback remains retained. | Supported canary scope only. |
| 7 | `rust-protocol-adapter-forwarding-expansion` | Complete | Rust forwards traffic through adapter policy: DIRECT listener -> target relay with 204 evidence, REJECT listener with 403 evidence, byte accounting, and fallback for unsupported remote paths. | Loopback adapter policy only; no remote encrypted protocol ownership. |
| 8 | `rust-remote-adapter-transport-expansion` | Complete | Rust executes a bounded TCP remote-adapter transport over loopback, parses a target authority, dials the target, forwards HTTP bytes, records byte evidence, and keeps unsupported proxy protocols on fallback. | Evidence path only for bounded TCP transport; no full proxy protocol ownership. |
| 9 | `rust-http-connect-proxy-adapter` | Complete | Rust accepts HTTP CONNECT, validates authority/Host, establishes a target TCP stream, tunnels bytes bidirectionally, and records target 204 evidence. | HTTP CONNECT TCP only; encrypted outbound protocols and UDP remain Mihomo-owned. |
| 10 | `rust-encrypted-proxy-protocol-preflight` | Complete | Rust runs Shadowsocks-style AES-256-GCM address-frame evidence and Trojan SHA224 auth-frame evidence over loopback, including decrypt/validate/forward/response checks. | Framing/auth preflight only; full encrypted sessions stay Mihomo fallback. |
| 11 | `rust-shadowsocks-aead-adapter-execution` | Complete | Rust executes a scoped Shadowsocks AEAD adapter path: decrypt address frame, validate loopback target, dial target, forward HTTP request, encrypt response, and write rollback checkpoint. | Scoped loopback TCP Shadowsocks AEAD only; UDP, plugin transports, VMess/VLESS/Trojan, and packet capture stay Mihomo fallback. |
| 12 | `rust-shadowsocks-aead-adapter-canary` | Complete | Run canary evidence for the scoped AEAD adapter across rollback checkpoint, fallback trigger, byte accounting, and post-run health boundaries. | Still opt-in; do not broaden default routing. |
| 13 | `rust-encrypted-proxy-session-expansion` | Complete | Expand from one scoped AEAD execution into larger encrypted-session handling with one encrypted address frame, multiple AEAD request chunks, encrypted target responses, fallback evidence, and persisted evidence.yaml. | Keep VMess/VLESS/Trojan, Shadowsocks UDP/plugin transports, and packet capture on fallback until separately implemented. |
| 14 | `rust-tun-transparent-routing-execution` | Complete | Implement bounded transparent IPv4/TCP packet parsing, destination extraction, loopback target execution, rollback checkpoint, and leak evidence before claiming TUN replacement. | High risk; system-wide packet capture still remains Mihomo/service fallback. |
| 15 | `mihomo-fallback-retirement-wider-scope` | Complete | Retire Mihomo fallback only for scopes with repeated passed canary, rollback, and hold evidence; retain fallback for all unsupported protocols. | Explicit opt-in and rollback required. |
| 16 | `rust-default-data-plane-closeout` | Complete | Close out bounded Rust-owned data-plane scope, reconcile evidence ownership, write a closeout manifest, and list the remaining unsupported Mihomo removal blockers. | No default ownership claims beyond passed evidence. |
| 17 | `rust-socks-udp-associate-execution` | Complete | Implement bounded Rust SOCKS5 UDP ASSOCIATE datagram parsing, loopback UDP forwarding, rollback checkpoint, fallback retention, and byte/leak evidence. | SOCKS auth, TCP command negotiation, fragments, non-loopback UDP, and packet capture remain Mihomo-owned. |
| 18 | `rust-dns-fake-ip-runtime` | Complete | Implement bounded Rust fake-ip allocation for one domain, including CIDR parsing, deterministic in-range answer synthesis, rollback checkpoint, fallback retention, and DNS leak evidence. | fake-ip cache/reverse mapping, fake-ip-filter, fallback-filter, nameserver-policy, and default DNS runtime remain Mihomo-owned. |
| 19 | `rust-dns-fallback-filter-runtime` | Complete | Implement bounded Rust fallback-filter evaluation for one domain/IP answer, including domain suffix/exact rules, ipcidr matching, rollback checkpoint, fallback retention, and DNS leak evidence. | Full GeoIP database coverage, fallback upstream execution, nameserver-policy, fake-ip cache/reverse mapping, and default DNS runtime remain Mihomo-owned. |
| 20 | `rust-dns-nameserver-policy-runtime` | Complete | Implement bounded Rust nameserver-policy dispatch for one domain, including exact and +.suffix matcher parsing, selected nameserver evidence, rollback checkpoint, fallback retention, and DNS leak evidence. | geosite/rule-provider, wildcard/multi-token matchers, upstream execution, DNS health checks, and default DNS runtime remain Mihomo-owned. |
| 21 | `rust-socks-auth-execution` | Complete | Implement bounded Rust SOCKS5 username/password negotiation and loopback CONNECT preflight with rollback checkpoint, fallback retention, and leak evidence. | SOCKS TCP data forwarding, BIND, non-loopback UDP, fragments, Shadowsocks UDP/plugin transports, and packet capture remain Mihomo-owned. |
| 22 | `rust-socks-tcp-connect-execution` | Complete | Implement bounded Rust SOCKS5 TCP CONNECT data forwarding over loopback, including username/password method negotiation, loopback target validation, request/response byte evidence, rollback checkpoint, fallback retention, and leak evidence. | SOCKS BIND, non-loopback UDP, fragments, Shadowsocks UDP/plugin transports, VMess/VLESS/Trojan, and packet capture remain Mihomo-owned. |
| 23 | `rust-socks-bind-execution` | Complete | Implement bounded Rust SOCKS5 BIND forwarding over loopback, including username/password method negotiation, first/second BIND replies, peer validation, request/response byte evidence, rollback checkpoint, fallback retention, and leak evidence. | SOCKS non-loopback UDP, fragments, Shadowsocks UDP/plugin transports, VMess/VLESS/Trojan, and packet capture remain Mihomo-owned. |
| 24 | `rust-socks-udp-fragments-execution` | Complete | Implement bounded Rust SOCKS5 UDP two-fragment reassembly over loopback, including RFC1928 FRAG sequencing, final-fragment validation, request/response byte evidence, rollback checkpoint, fallback retention, and leak evidence. | SOCKS non-loopback UDP, fragment queues/timeouts, Shadowsocks UDP/plugin transports, VMess/VLESS/Trojan, and packet capture remain Mihomo-owned. |
| 25 | `rust-dns-fallback-filter-geoip-runtime` | Complete | Implement bounded Rust fallback-filter geoip/geoip-code evaluation for one DNS answer, including canary CIDR matching, fallback decision evidence, rollback checkpoint, fallback retention, and DNS leak evidence. | Full GeoIP database coverage, fallback upstream execution, wildcard/default DNS integration, nameserver-policy geosite/rule-provider/wildcards, fake-ip cache/reverse mapping, and packet capture remain Mihomo-owned. |
| 26 | `unsupported-protocol-and-packet-capture-implementation` | Next | Continue with one remaining unsupported Mihomo-owned protocol, DNS policy, or packet-capture blocker and implement a bounded Rust execution path with canary, rollback, hold, fallback, and byte/leak evidence. | Unsupported fallback remains retained until each blocker has equivalent Rust evidence. |

### Definition of done for future PRs

A PR counts as migration progress only if it contains at least one of:

- Rust code that handles real DNS, adapter, protocol, TUN/system-proxy, or fallback execution behavior.
- Tests/fixtures that prove parity for one of those real behaviors.
- Removal or deprecation of a Mihomo dependency after equivalent Rust behavior exists.

Documentation-only PRs are allowed only to correct this roadmap or remove misleading status.

## Non-negotiable boundaries

### 1. Rust is the control-plane source of truth

Do not add paths that bypass Rust state or Rust gates.

Forbidden patterns:

- UI writes Mihomo YAML directly.
- UI calls Mihomo mutation APIs directly.
- App policy, node pool, DNS, or security profile logic is assembled ad hoc in the frontend.
- Runtime mutation happens without an app-owned Rust command.
- Runtime mutation is not recorded in audit, history, closeout, or rollback state when it affects production runtime.

### 2. Mihomo remains the production data plane

These areas stay owned by Mihomo/Go unless a dedicated high-risk PR series explicitly changes them. The accelerated Rust default may still route unsupported paths through Mihomo fallback; fallback retirement is the high-risk change, not the first Rust-default switch:

- outbound / inbound protocol stacks
- adapter runtime
- TUN / transparent proxy
- real packet forwarding
- default DNS runtime
- OS-level per-app network isolation / sandboxing

Do not mix these changes with UI cleanup, type cleanup, documentation cleanup, or telemetry-only PRs.

### 3. Runtime apply must stay explicitly gated

Any real production runtime apply must preserve this chain:

```text
staged artifact
  -> checksum / boundary manifest
  -> explicit allow decision
  -> preflight guard
  -> runtime apply audit
  -> observed verification
  -> closeout / hold / rollback readiness
```

Readiness, shadow evidence, smoke evidence, verification, or closeout records are not automatic rollout permission.

### 4. DNS runtime remains opt-in

Default DNS runtime must not silently replace Go/Mihomo DNS. The only allowed path is:

```text
readiness gate
  -> shadow evidence
  -> explicit opt-in switch guard
  -> executor preflight
  -> limited execution
  -> observed verification
  -> rollback drill
  -> expanded gate / hold / repeated reverify history
```

Do not expand DNS impact or remove rollback boundaries before the evidence chain is complete.

### 5. Frontend runtime types use a view-model boundary

Rust-generated bindings are the field source for Mihomo payloads, but UI-specific semantics stay in app-owned view models.

Current boundary:

```text
tauri-plugin-mihomo-api
  -> generated Proxy / ProxyProvider / Rule / RuleProvider / BaseConfig / ...
  -> src/types/proxy.ts app-owned view models
  -> UI components and services
```

`IProxyItem`, `IProxyGroupItem`, and `IProxyProviderItem` must not be force-replaced with raw generated types because they preserve UI semantics such as `provider`, `fixed`, and expanded group `all` items.

## Completed control-plane milestones

| Area | Status | Durable result |
| --- | --- | --- |
| Config validation | Complete | Rust native validator replaced the old `verge-mihomo -t` validation chain. |
| Rule engine | Complete | DOMAIN, CIDR, port, NETWORK, MATCH, GEOIP, GEOSITE, ASN, RULE-SET, process, UID, DSCP, inbound, wildcard, logical, and sub-rule paths are Rust-owned. |
| Control diagnostics | Complete | Rule explain, config diff, diagnostics summary, latency planner, and node selection planner are Rust-owned. |
| DNS planning | Complete | DNS explain and controlled probe planning exist in Rust; default DNS runtime is still protected by opt-in gates. |
| Subscription pipeline | Complete | Source config -> artifact -> active artifact -> runtime is transactional and Rust-owned. |
| App-facing monitor path | Complete | Connection, traffic, memory, and log views use Rust monitor controllers and Tauri events instead of frontend Mihomo WebSocket ownership. |
| App runtime orchestration | Complete to hold milestone | Runtime plan, projection artifact, staged activation, runtime-apply decision, verification closeout, and post-apply hold are Rust-owned. |
| Runtime mutation audit | Complete | Mode, system proxy, TUN toggle, DNS apply, geo update, sensitive-config edits, TLS rotation, and upgrade actions are audited. |
| Runtime telemetry | Complete | Engine, perf, buffer, hot-reload, XDP, rule traffic, TLS fingerprint, provider health, delay, and runtime wrapper result cache are Rust-observed. |
| Proxy type boundary | Complete | Proxy globals moved to app-owned view models backed by Rust-generated field sources. |

## Phase 8 kernel replacement track

Phase 8 should no longer be managed as a long list of synthetic gates. The prior R0-R7 artifacts are useful as audit history, but they do not replace production DNS, adapter, protocol, or TUN behavior.

### Corrected Phase 8 status

| Track | Current status | Next useful work |
| --- | --- | --- |
| Seam inventory / runtime selection | Complete enough | Do not add more inventory-only gates unless required by a real implementation PR. |
| DNS | Bounded opt-in parity path in progress | Fallback-filter domain/ipcidr, fallback-filter geoip/geoip-code canary evaluation, fake-ip allocation, and nameserver-policy exact/suffix dispatch are Rust-owned only for bounded opt-in evidence; do not make DNS default until adapter/protocol/TUN rollback boundaries are ready. |
| Adapter / egress | Bounded opt-in parity path in progress | Keep canarying supported adapter decisions; move next to real protocol forwarding subset. |
| Protocol forwarding | Unsupported protocol expansion in progress | DIRECT/REJECT, bounded remote transport, HTTP CONNECT, encrypted framing preflight, scoped Shadowsocks AEAD adapter execution, AEAD canary evidence, multi-chunk encrypted TCP session evidence, bounded SOCKS5 UDP ASSOCIATE datagram forwarding, SOCKS5 username/password negotiation, SOCKS5 TCP CONNECT loopback forwarding, SOCKS5 BIND loopback forwarding, and SOCKS5 UDP two-fragment loopback reassembly are Rust-owned; VMess/VLESS/Trojan, Shadowsocks UDP/plugin transports, SOCKS non-loopback UDP plus fragment queues/timeouts, and packet capture remain Mihomo fallback. |
| TUN / system proxy | Bounded Rust transparent routing execution in progress | Rust has route-mode parity plus bounded transparent IPv4/TCP packet parsing/execution evidence; system-wide packet capture still uses Mihomo/service. |
| Mihomo fallback retirement | Bounded closeout complete | DNS, adapter, protocol, encrypted-session, bounded transparent-route, and loopback SOCKS UDP associate scopes have evidence; unsupported fallback and packet capture remain Mihomo-owned. |

### Retained historical value

Keep the existing gate commands as safety/audit surfaces, but stop treating them as the main roadmap. They are prerequisites and evidence channels, not deliverables by themselves.

## Current Rust-owned capability inventory

These are the current Rust-owned surfaces. Items marked "bounded execution" reduce Mihomo-owned data-plane behavior only for the named scope; everything outside that scope stays Mihomo fallback.

- Config schema validation and rule engine.
- Geodata, ASN, RULE-SET, and process metadata interpretation.
- Subscription artifact pipeline and active version management.
- App runtime state document, runtime plan, projection artifact, staged activation, rollback boundary, runtime-apply audit, observed verification, closeout, and hold.
- App-facing connection, traffic, memory, and log monitor event paths.
- DNS readiness, shadow evidence, bounded opt-in dns/hosts runtime patching, resolver probe, and rollback.
- Adapter/egress bounded execution for DIRECT/REJECT/proxy-group decisions and supported runtime patching.
- Loopback TCP/HTTP forwarding bounded execution with bidirectional byte accounting.
- DIRECT/REJECT adapter-aware forwarding bounded execution.
- Bounded remote adapter transport evidence for TCP target dialing and response accounting.
- HTTP CONNECT bounded execution for CONNECT authority validation, target dialing, and tunnel byte forwarding.
- Encrypted proxy preflight evidence for Shadowsocks-style AES-256-GCM address framing and Trojan SHA224 auth framing.
- Scoped Shadowsocks AEAD adapter bounded execution for loopback TCP address frames, encrypted response handling, evidence.yaml, rollback-checkpoint.yaml, canary readback across fallback/rollback/health evidence, and multi-chunk encrypted TCP session evidence.
- Bounded TUN transparent routing execution for synthetic IPv4/TCP packet parsing, destination extraction, loopback target execution, rollback checkpoint, and leak evidence without owning system-wide packet capture.
- TUN/system-proxy bounded parity for route-mode planning, system proxy apply, TUN config/restart bridge, rollback records, and rollback apply.
- Wider scoped Mihomo fallback retirement manifest/checkpoint covering DNS, adapter forwarding, bounded remote transport, HTTP CONNECT, Shadowsocks AEAD TCP session, and bounded transparent IPv4/TCP route evidence.
- Default data-plane closeout manifest that reconciles bounded Rust-owned evidence ownership and preserves unsupported Mihomo-owned blockers.
- Bounded SOCKS5 UDP ASSOCIATE execution for RSV/FRAG/ATYP/DST.PORT parsing, loopback UDP forwarding, rollback checkpoint, fallback retention, and byte/leak evidence without owning SOCKS auth at that batch boundary, non-loopback UDP, fragments at that batch boundary, or packet capture.
- Bounded DNS fake-ip execution for deterministic in-range one-domain allocation, rollback checkpoint, fallback retention, and DNS leak evidence without owning fake-ip cache/reverse mapping, filters, policy dispatch, or default DNS.
- Bounded DNS fallback-filter execution for one domain/IP answer, domain suffix/exact rules, ipcidr matching, rollback checkpoint, fallback retention, and DNS leak evidence without owning geoip, upstream fallback execution, nameserver-policy, or default DNS.
- Bounded DNS nameserver-policy execution for one domain, exact and +.suffix matcher parsing, selected nameserver evidence, rollback checkpoint, fallback retention, and DNS leak evidence without owning geosite/rule-provider matching, wildcard/multi-token dispatch, upstream execution, health checks, or default DNS.
- Bounded SOCKS5 username/password auth execution for method negotiation, RFC1929 credential-frame validation, loopback CONNECT preflight, rollback checkpoint, fallback retention, and leak evidence without owning TCP data forwarding at that batch boundary, BIND, non-loopback UDP, fragments, plugin transports, or packet capture.
- Bounded SOCKS5 TCP CONNECT execution for username/password method negotiation, loopback target validation, one request/response forwarding exchange, rollback checkpoint, fallback retention, and leak evidence without owning BIND at that batch boundary, non-loopback UDP, fragments, plugin transports, VMess/VLESS/Trojan, or packet capture.
- Bounded SOCKS5 BIND execution for username/password method negotiation, first and second BIND success replies, loopback peer validation, one request/response forwarding exchange, rollback checkpoint, fallback retention, and leak evidence without owning non-loopback UDP, fragments at that batch boundary, plugin transports, VMess/VLESS/Trojan, or packet capture.
- Bounded SOCKS5 UDP fragment execution for two-fragment loopback reassembly, final-fragment validation, one request/response UDP forwarding exchange, rollback checkpoint, fallback retention, and leak evidence without owning non-loopback UDP, fragment queues/timeouts, plugin transports, VMess/VLESS/Trojan, or packet capture.
- Bounded DNS fallback-filter geoip execution for one candidate answer, geoip-code canary CIDR matching, fallback decision evidence, rollback checkpoint, fallback retention, and DNS leak evidence without owning full GeoIP databases, fallback upstream execution, wildcard/default DNS integration, or policy cache.

## Remaining blockers and acceleration boundaries

The next blocker is not another readiness gate; it is one of the remaining unsupported protocol, DNS, adapter, TUN, packet-capture, or fallback-retention implementations after bounded DNS fallback-filter geoip execution. Do not retire Mihomo fallback or claim broad Rust data-plane replacement until all of these have landed as real code and tests:

- DNS fallback-filter full GeoIP database coverage, fallback upstream execution, nameserver-policy geosite/rule-provider/wildcard execution, plus fake-ip cache/reverse mapping and wildcard filter semantics.
- Connection/session accounting parity for traffic handled by Rust, including encrypted adapter bytes.
- Repeated platform TUN/system-proxy rollback and route restoration hold evidence for Windows, macOS, and Linux.
- Full encrypted-session implementations for VMess/VLESS/Trojan separately; current Shadowsocks AEAD ownership is still bounded to loopback TCP session evidence.
- SOCKS non-loopback UDP, Shadowsocks UDP/plugin transports, transparent routing defaults, and packet capture execution paths.
- Mihomo fallback that preserves connectivity without app restart for every unsupported path.
- Post-canary hold evidence that covers fallback triggers, rollback, DNS leaks, and health telemetry.

These blockers allow the next useful implementation PR after `rust-dns-fallback-filter-geoip-runtime`: `unsupported-protocol-and-packet-capture-implementation`. They block full protocol replacement and any claim that packet capture is Rust-owned.

## Removed from this document

The migration history used to contain long per-batch logs and a separate Phase 8 document. Those details are intentionally not duplicated here anymore.

Use:

- PR history for exact implementation details.
- `git log` / `git blame` for archaeology.
- This roadmap for current boundaries, allowed next work, and stop conditions.

## Future decision points

### Option A: Stop after current R3 evidence

Accept Rust-owned control plane plus bounded loopback listener evidence. Mihomo remains the production data plane.

### Option B: Continue low-risk maintenance

Allowed cleanup:

- remove dead frontend paths
- keep generated types and app view models aligned
- improve diagnostics wording
- improve existing evidence commands when required by an implementation PR
- improve audit/history rendering

### Option C: Continue high-risk data-plane migration

Allowed only through the corrected real fast-track sequence above. The current next batch is `unsupported-protocol-and-packet-capture-implementation` after bounded SOCKS UDP fragment execution; keep execution scoped to supported canary evidence and retain unsupported fallback.

## PR checklist for future changes

Every PR in this area must state:

- Does it mutate runtime?
- Does it touch Mihomo config, controller APIs, process lifecycle, system proxy, TUN, DNS, or adapter forwarding?
- What is the rollback path?
- What evidence proves the change is app-owned and gated?
- Which boundary in this roadmap allows the change?
- If it is Phase 8 work, which batch does it belong to and what is the next safe batch?

## Document maintenance rules

- Keep this file compact and current-state oriented.
- Do not re-add historical per-PR logs.
- Do not create parallel Go-to-Rust status documents; update this roadmap instead.
- Preserve the production data-plane boundary unless a dedicated cutover PR changes it.
