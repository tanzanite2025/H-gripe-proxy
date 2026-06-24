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

Status is current through Go-to-Rust migration final review reconciliation. The migration has now moved past the earlier gate-only detour and has real bounded Rust data-plane implementations for DNS, adapter policy, loopback forwarding, HTTP CONNECT, encrypted framing, scoped Shadowsocks AEAD execution, AEAD canary evidence, multi-chunk encrypted TCP session evidence, bounded transparent IPv4/TCP packet execution, wider fallback retirement manifest/checkpoint execution, default-scope closeout ownership reconciliation, loopback-only SOCKS5 UDP ASSOCIATE datagram forwarding, bounded SOCKS UDP non-loopback fragment forwarding, deterministic one-domain DNS fake-ip allocation, bounded fake-ip cache/reverse lookup, bounded DNS policy/cache/upstream bundle canaries, bounded VMess/VLESS/Trojan loopback TCP canary sessions, bounded VMess/VLESS/Trojan non-loopback local TCP canary sessions, bounded fallback-filter domain/ipcidr evaluation, bounded nameserver-policy exact/suffix dispatch, bounded SOCKS5 username/password negotiation, bounded SOCKS5 TCP CONNECT forwarding, bounded SOCKS5 BIND forwarding, bounded SOCKS5 UDP two-fragment reassembly, bounded SOCKS UDP fragment queue timeout/reap evidence, bounded DNS fallback-filter geoip/geoip-code evaluation, bounded UDP/plugin transport bundle evidence, bounded TUN/packet-capture hold evidence, bounded Mihomo fallback retirement bundle evidence, final review reconciliation evidence, sidecar-independent rollback archive evidence, DNS default-path blocker reduction evidence, and route/packet-capture blocker reduction evidence, and protocol default-path blocker reduction evidence, and plugin process supervision evidence, and QUIC/UDP profile blocker evidence, and default forwarding hold evidence, and DNS cutover hold evidence, and DNS system-resolver leak blocker evidence, and TUN device lifecycle blocker evidence, and route mutation rollback blocker evidence, and packet leak hold blocker evidence, and GeoIP database blocker evidence, and SOCKS UDP default blocker evidence, and encrypted protocol default blocker evidence, and plugin binary compatibility blocker evidence. The old `rust-data-plane-hardening-*` IPC commands remain safety metadata only; ownership claims below are limited to the explicitly named bounded paths.

| Area | State | Boundary |
| --- | --- | --- |
| Rust control plane | Mature | Validation, planning, projection artifacts, audit, telemetry, and frontend type surfaces are Rust-owned enough to support real data-plane work. Stop adding more read-only gate-only PRs here. |
| DNS runtime | Bounded opt-in parity path in progress | Rust now synthesizes a dns/hosts runtime patch, probes supported resolvers, executes bounded one-domain fake-ip allocation, bounded fake-ip cache/reverse lookup, DNS policy/cache/upstream bundle canaries, fallback-filter domain/ipcidr evaluation, fallback-filter geoip/geoip-code evaluation, and nameserver-policy exact/suffix/geosite/rule-provider/wildcard dispatch canaries with rollback/leak evidence, blocks unsupported default DNS cutover, read-only GeoIP/ASN/GeoSite candidate discovery, bounded Rust geodata lookup matrix evidence, and production resolver handoff, reduces live resolver/cache/geodata-refresh blockers and bounded production DNS cutover hold plus read-only system-resolver snapshot/leak observation through Rust-owned evidence, and applies supported patches through an explicit opt-in bridge with rollback. Mihomo still owns default DNS until canary evidence passes. |
| Adapter / egress runtime | Bounded opt-in parity path in progress | Rust now chooses DIRECT/REJECT/proxy-group adapter targets from app runtime state, validates candidate protocol compatibility, patches proxy-groups/rules through an explicit opt-in bridge, and keeps Mihomo fallback/rollback. |
| Protocol forwarding | Unsupported protocol expansion in progress | Rust now owns loopback TCP/HTTP forwarding, DIRECT/REJECT policy, bounded remote transport, HTTP CONNECT tunneling, encrypted framing preflight, scoped Shadowsocks AEAD adapter execution, AEAD canary evidence, multi-chunk encrypted TCP session evidence, bounded SOCKS5 UDP ASSOCIATE datagram forwarding, bounded SOCKS5 username/password negotiation, bounded SOCKS5 TCP CONNECT forwarding, bounded SOCKS5 BIND forwarding, bounded SOCKS5 UDP two-fragment reassembly, bounded SOCKS UDP fragment queue timeout/reap evidence, bounded VMess/VLESS/Trojan loopback TCP canary sessions, bounded VMess/VLESS/Trojan non-loopback local TCP canary sessions, bounded UDP/plugin transport bundle evidence, non-loopback TCP canary evidence, multiplex frame evidence, plugin lifecycle manifest evidence, and plugin process health/crash/restart supervision evidence, plugin binary startup/stdout/health/crash compatibility contract evidence, non-loopback QUIC-like UDP transcript evidence, QUIC/UDP profile matrix evidence, and bounded default-forwarding hold evidence, plus committed operator approval and guarded apply/rollback/post-apply hold manifests for production default-forwarding cutover. Mihomo still owns real remote encrypted/QUIC peer compatibility, operator-approved real plugin binary compatibility, and system-wide packet capture. |
| TUN / system proxy | Bounded packet-capture hold evidence in progress | Rust now owns explicit off/system-proxy/TUN route-mode planning, OS system-proxy apply through the Sysopt/sysproxy path, TUN config/restart apply through the existing backend, rollback records, rollback apply, bounded transparent IPv4/TCP packet parsing/execution evidence, repeated platform route rollback hold replay, bounded packet-capture canary parsing, route snapshot/restore-plan evidence, synthetic packet-capture hold evidence, loopback-only DNS leak telemetry, bounded TUN lifecycle state-machine evidence, TUN rollback ordering evidence, read-only route mutation replay evidence, platform route apply/rollback plan evidence, bounded packet leak hold evidence, synthetic external-interface leak observation evidence, and guarded TUN/packet-capture apply plus rollback checkpoint evidence across the TUN lifecycle, route mutation, packet-capture hold, and packet leak gates. Mihomo/service TUN and packet-capture fallback is now demoted to checkpoint restore until fallback retirement closeout. |
| Mihomo fallback retirement | Bounded fallback-retirement bundle complete | Rust now writes wider execution manifests, emergency rollback checkpoints, default data-plane closeout manifests, unsupported-path fallback continuity evidence, hold telemetry, sidecar source/binary dependency audit evidence, sidecar-independent rollback archive evidence, and selective supported-scope fallback retirement evidence across the bounded DNS/adapter/protocol/UDP/plugin/TUN packet-capture inventory. Operator-approved production SOCKS UDP default forwarding cutover and real-profile UDP hold, operator-approved production DNS cutover/privileged system resolver apply-restore, operator-approved production geodata refresh and file availability, real remote encrypted/QUIC peer compatibility, operator-approved real plugin binary compatibility, fallback-retirement closeout manifests now consume guarded default-forwarding and TUN/packet-capture apply evidence while retaining rollback checkpoint evidence. Final Mihomo binary removal gate manifests now consume fallback-retirement closeout, sidecar-independent rollback, final review, release audit, and operator cutover evidence. Distribution cleanup remains blocked until release closeout. |
| Next blocker | `go-to-rust-migration-release-closeout` | Final Mihomo binary removal gate now consumes fallback-retirement closeout, sidecar-independent rollback, final review, release audit, and operator cutover evidence into one removal manifest with rollback checkpoint evidence. The next useful work is release closeout/packaging cleanup, not another final-removal readiness wrapper. |

## Acceleration plan

Course correction: the previous roadmap drifted into dozens of IPC/readiness gates.
That is no longer useful. From this point forward, roadmap progress is measured
by retired Mihomo-owned runtime surface, not by another `*_guard`, `*_dry_run`,
`*_readiness`, parser-only, or one-canary wrapper.

### Final cutover target

If the project chooses high-risk data-plane migration, the final target is one
Rust-owned production runtime path for supported profiles:

```text
Rust DNS + adapter policy + protocol forwarding + UDP/plugin transport +
TUN/packet capture/default routing
  -> observed health / leak checks / rollback history
  -> selective Mihomo fallback deprecation
  -> Mihomo sidecar binary removal when no supported default path depends on it
```

The current bounded evidence paths are not the final state. They are only proof
needed before broadening default ownership.

### Hard stop on small-step migration PRs

- Do not create another PR whose only product change is a new read-only evidence,
  gate, dry-run, readiness, parser, generated type, or one-command wrapper.
- A safety gate may be included only when it protects a real implementation in
  the same PR.
- Every migration PR must delete, bypass, or demote a concrete Mihomo-owned
  runtime surface: default DNS, adapter egress, protocol forwarding, UDP/plugin
  transport, TUN/packet capture, fallback dependency, or Go/Mihomo artifacts.
- Do not append one numbered row per canary. Update the owning bundle checklist
  instead.
- The three accelerated implementation bundles, bundled manual default-path removal review, operator-approved default-path cutover manifest path, production default-forwarding cutover approval, Go/Mihomo final removal gate linkage, guarded retirement execution linkage, post-execution verification linkage, rollback-surface retirement linkage, completion closeout linkage, Rust data-plane hardening preflight linkage, boundary audit linkage, opt-in execution guard linkage, opt-in dry-run linkage, opt-in execution linkage, opt-in execution verification linkage, controlled rollout guard linkage, controlled rollout dry-run linkage, controlled rollout readiness closeout linkage, controlled rollout canary execution linkage, controlled rollout canary verification linkage, supported default promotion chain linkage, expanded default rollout chain linkage, Mihomo fallback retirement readiness linkage, fallback retirement execution linkage, Rust default data-plane closeout linkage, unsupported protocol execution linkage, route/packet-capture privileged hold linkage, TUN lifecycle linkage, route mutation rollback linkage, packet leak hold linkage, protocol default linkage, plugin process supervision linkage, QUIC/UDP profile linkage, default forwarding hold linkage, and production default-forwarding cutover approval, guarded production apply linkage, guarded TUN/packet-capture apply linkage, and fallback retirement closeout linkage, and final Mihomo binary removal gate linkage are complete; future work must target release closeout and packaging cleanup rather than more fallback hold wrappers.

### Completed implementation bundles

This table is the completed accelerated batch map. Prior completed canaries remain audit
history; future progress must not add new synthetic implementation bundles.

| Order | Bundle | Status | Must ship together | Success condition |
| --- | --- | --- | --- | --- |
| 1 | `rust-udp-and-plugin-transport-bundle` | Complete | SOCKS non-loopback UDP policy gates, Shadowsocks UDP canary forwarding, plugin transport preflight/execution evidence, fragment queue timeout/eviction canary, and fallback continuity without app restart. | A supported UDP/plugin path runs through Rust with leak evidence and one-switch Mihomo fallback. |
| 2 | `rust-tun-packet-capture-hold-bundle` | Complete | Repeated Windows/macOS/Linux TUN/system-proxy rollback drills, route restoration checks, transparent-routing default boundaries, packet-capture canaries, and DNS leak/health telemetry after rollback. | Rust can hold platform routing/packet-capture ownership through repeated rollback/reapply cycles without default traffic leaks. |
| 3 | `rust-mihomo-fallback-retirement-bundle` | Complete | Unsupported-path fallback continuity, emergency rollback, hold telemetry, sidecar source/binary dependency audit, and selective Mihomo dependency deprecation/removal. | Mihomo is removed only for surfaces with Rust execution evidence, default-path coverage, rollback history, and post-cutover hold proof. |

### Completed work is inventory, not future sequencing

Completed DNS, adapter, loopback forwarding, HTTP CONNECT, encrypted framing,
Shadowsocks AEAD, VMess/VLESS/Trojan loopback, fake-ip, fallback-filter,
nameserver-policy, SOCKS auth/CONNECT/BIND/UDP fragment, and bounded transparent
IPv4/TCP work should be treated as evidence inventory. Do not split follow-up work
by those old batch names unless the change removes a final-review blocker above.

### Definition of done for future PRs

A future migration PR is done only if it answers all of these with code or
persisted evidence:

- Which Mihomo-owned default or fallback surface became smaller?
- Which Rust path executed real traffic or real runtime mutation?
- What explicit opt-in, audit, verification, hold, and rollback evidence protects
  it?
- What remains Mihomo-owned after this PR?
- Which final-review blocker or retained fallback boundary did it reduce?

### Accelerated PR sizing for future work

- Combine adjacent runtime blockers into one PR when they share the same rollback
  and evidence path.
- Prefer one cohesive implementation plus tests over multiple preparatory PRs.
- Keep UI/reporting changes inside the same PR only when they expose the runtime
  evidence needed for that implementation.
- Reject new roadmap steps that do not reduce a final-review blocker or retained fallback boundary.

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

Do not mix these changes with UI cleanup, unrelated type cleanup, unrelated documentation cleanup, or telemetry-only PRs. Larger migration PRs should bundle related runtime blockers, not unrelated maintenance.

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

Phase 8 should no longer be managed as a long list of synthetic gates. The prior R0-R7 artifacts are useful as audit history, but they do not replace production DNS, adapter, protocol, or TUN behavior. Treat the table below as boundary inventory, not as permission to create more small standalone PRs.

### Corrected Phase 8 status

| Track | Current status | Next useful work |
| --- | --- | --- |
| Seam inventory / runtime selection | Complete enough | Do not add more inventory-only gates unless required by a real implementation PR. |
| DNS | Bounded opt-in parity path in progress | Keep current bounded evidence as input to the remaining bundles; do not create isolated DNS-only expansion PRs unless they retire default-DNS fallback inside a bundle. |
| Adapter / egress | Bounded opt-in parity path in progress | Fold any remaining adapter work into UDP/plugin transport or fallback-retirement bundles; do not add adapter-only readiness gates. |
| Protocol forwarding | Unsupported protocol expansion in progress | Existing loopback TCP/HTTP/CONNECT/SOCKS/encrypted and UDP/plugin transport canaries are evidence inventory for the fallback-retirement bundle; do not add protocol-only readiness gates. |
| TUN / system proxy | Bounded packet-capture hold evidence in progress | The TUN/packet-capture hold bundle now supplies repeated rollback, route restoration, packet-capture canary, DNS leak telemetry evidence, bounded TUN lifecycle state-machine evidence, TUN rollback ordering evidence, read-only route mutation replay evidence, platform route apply/rollback plan evidence, bounded packet leak hold evidence, and synthetic external-interface leak observation evidence. Keep system-wide capture/default routing on Mihomo until the fallback-retirement bundle. |
| Mihomo fallback retirement | Manual default-path removal review in progress | The accelerated implementation bundles and final review are complete. Rust now archives rollback evidence without invoking the Mihomo sidecar and reviews DNS, protocol, route/packet-capture, and sidecar removal surfaces as one cutover bundle; sidecar binary removal remains blocked until operator-approved default paths have execution, rollback, hold, and leak evidence. |

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
- Bounded DNS fake-ip execution for deterministic in-range one-domain allocation, rollback checkpoint, fallback retention, and DNS leak evidence without owning persistent fake-ip cache lifecycle, filters, policy dispatch, or default DNS.
- Bounded DNS fallback-filter execution for one domain/IP answer, domain suffix/exact rules, ipcidr matching, rollback checkpoint, fallback retention, and DNS leak evidence without owning geoip, upstream fallback execution, nameserver-policy, or default DNS.
- Bounded DNS nameserver-policy execution for one domain, exact and +.suffix matcher parsing, selected nameserver evidence, rollback checkpoint, fallback retention, and DNS leak evidence without owning geosite/rule-provider matching, wildcard/multi-token dispatch, upstream execution, health checks, or default DNS.
- Bounded SOCKS5 username/password auth execution for method negotiation, RFC1929 credential-frame validation, loopback CONNECT preflight, rollback checkpoint, fallback retention, and leak evidence without owning TCP data forwarding at that batch boundary, BIND, non-loopback UDP, fragments, plugin transports, or packet capture.
- Bounded SOCKS5 TCP CONNECT execution for username/password method negotiation, loopback target validation, one request/response forwarding exchange, rollback checkpoint, fallback retention, and leak evidence without owning BIND at that batch boundary, non-loopback UDP, fragments, plugin transports, VMess/VLESS/Trojan, or packet capture.
- Bounded SOCKS5 BIND execution for username/password method negotiation, first and second BIND success replies, loopback peer validation, one request/response forwarding exchange, rollback checkpoint, fallback retention, and leak evidence without owning non-loopback UDP, fragments at that batch boundary, plugin transports, VMess/VLESS/Trojan, or packet capture.
- Bounded SOCKS5 UDP fragment execution for two-fragment loopback reassembly, final-fragment validation, one request/response UDP forwarding exchange, rollback checkpoint, fallback retention, and leak evidence without owning non-loopback UDP, fragment queues/timeouts, plugin transports, VMess/VLESS/Trojan, or packet capture.
- Bounded DNS fallback-filter geoip execution for one candidate answer, geoip-code canary CIDR matching, fallback decision evidence, rollback checkpoint, fallback retention, and DNS leak evidence without owning full GeoIP databases, fallback upstream execution, wildcard/default DNS integration, or policy cache.
- Bounded DNS fake-ip cache execution for one normalized domain, forward fake-ip cache insertion, reverse fake-ip lookup evidence, rollback checkpoint, fallback retention, and DNS leak evidence without owning persistent cache lifecycle, eviction, wildcard filters, or default DNS.
- Bounded DNS policy/cache/upstream bundle execution for one normalized domain and candidate answer, including fake-ip lifecycle/eviction canary, fake-ip-filter wildcard evidence, loopback-only fallback upstream execution, nameserver-policy geosite/rule-provider/wildcard canaries, rollback checkpoint, fallback retention, and DNS leak evidence without owning default DNS, live resolver replacement, full GeoIP databases, production persistent cache storage, or geodata refresh.
- Bounded VMess/VLESS/Trojan loopback TCP canary execution with shared framing/session accounting, request/response byte evidence, rollback checkpoint, fallback retention, and leak evidence without owning non-loopback encrypted forwarding, QUIC/UDP variants, multiplexing, plugin transports, or default forwarding.
- Bounded UDP/plugin transport bundle execution with SOCKS non-loopback UDP policy gates, Shadowsocks UDP canary forwarding, plugin transport shim evidence, fragment queue timeout/eviction canary, rollback checkpoint, fallback retention, and leak evidence without owning broad non-loopback default UDP, external plugin process lifecycle, QUIC/multiplexed transports, packet capture, or default forwarding.

## Remaining blockers and acceleration boundaries

The next blocker is Go-to-Rust migration release closeout, not another default-forwarding, packet-capture, fallback-retirement, or final-removal readiness gate. Work through one cohesive release-closeout slice while preserving explicit rollback evidence:

1. Release closeout: consume the final binary removal manifest, packaging audit, and rollback checkpoint evidence before removing distribution-side Mihomo artifacts.
2. Final closeout bundle: rollback-surface retirement, release blocker audit, and final Mihomo binary removal evidence linkage.

Full protocol replacement and default DNS ownership remain blocked until release closeout proves packaging cleanup and release safety.

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

Sidecar-independent rollback archive, DNS default-path blocker reduction, route/packet-capture blocker reduction, and protocol default-path blocker reduction now reduce final-review blockers by adding Rust-owned rollback archives, bounded live resolver evidence, persistent cache migration evidence, geodata refresh ownership evidence, route snapshot/restore planning, synthetic packet-capture hold evidence, non-loopback TCP canary evidence, multiplex frame coverage, plugin lifecycle manifests, and plugin process health/crash/restart supervision evidence, plugin binary startup/stdout/health/crash compatibility contract evidence, non-loopback QUIC-like UDP transcript evidence, QUIC/UDP profile matrix evidence, and bounded default-forwarding hold evidence, and bounded DNS cutover hold evidence, and DNS system-resolver leak blocker evidence, and TUN device lifecycle blocker evidence, and route mutation rollback blocker evidence, and packet leak hold blocker evidence, and GeoIP database blocker evidence, and SOCKS UDP default blocker evidence, and encrypted protocol default blocker evidence, and plugin binary compatibility blocker evidence. Keep execution scoped to supported canary evidence and retain unsupported fallback until a specific default-path blocker is removed with new proof.

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
