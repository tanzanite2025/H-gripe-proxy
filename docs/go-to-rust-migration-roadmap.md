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

The migration has too many completed control-plane gates and not enough real data-plane replacement work. Treat the recent `rust-data-plane-hardening-*` IPC commands as safety metadata only; they do **not** mean Rust owns DNS runtime, TUN forwarding, adapter dialing, protocol stacks, or fallback retirement.

| Area | State | Boundary |
| --- | --- | --- |
| Rust control plane | Mature | Validation, planning, projection artifacts, audit, telemetry, and frontend type surfaces are Rust-owned enough to support real data-plane work. Stop adding more read-only gate-only PRs here. |
| DNS runtime | Bounded opt-in parity path in progress | Rust now synthesizes a dns/hosts runtime patch, probes supported resolvers, blocks unsupported fake-ip/fallback-filter/nameserver-policy execution, and applies through an explicit opt-in bridge with rollback. Mihomo still owns default DNS until canary evidence passes. |
| Adapter / egress runtime | Bounded opt-in parity path in progress | Rust now chooses DIRECT/REJECT/proxy-group adapter targets from app runtime state, validates candidate protocol compatibility, patches proxy-groups/rules through an explicit opt-in bridge, and keeps Mihomo fallback/rollback. |
| Protocol forwarding | Bounded Rust subset in progress | Rust now owns an opt-in loopback TCP/HTTP forwarding subset with a real accept loop, byte forwarding, session accounting, smoke evidence, and stop/rollback surface. Mihomo still owns SOCKS, remote adapters, TUN, and default forwarding. |
| TUN / system proxy | Bounded Rust parity in progress | Rust now owns explicit off/system-proxy/TUN route-mode planning, OS system-proxy apply through the Sysopt/sysproxy path, TUN config/restart apply through the existing backend, rollback records, and rollback apply. Mihomo/service still owns packet capture and transparent forwarding. |
| Mihomo fallback retirement | Readiness manifest in progress | Rust now builds/locks a fallback-retirement readiness manifest from concrete parity rollback artifacts and supported/unsupported scope. Execution remains blocked until real canary evidence passes. |
| Next real batch | `rust-runtime-real-canary` | Run the bounded Rust-owned paths under a capped canary profile and collect leak, rollback, health, and unsupported fallback evidence. |

## Acceleration plan

Course correction: the previous roadmap drifted into dozens of IPC/readiness gates. That is no longer useful. From this point forward, roadmap progress is measured by shipped data-plane capability, not by another `*_guard`, `*_dry_run`, or `*_readiness` wrapper.

### Hard stop on gate-only PRs

- Do not create another PR whose only product change is a new read-only evidence/gate command.
- A safety gate may be included only when it protects a real implementation in the same PR.
- Every migration PR must name the concrete Mihomo-owned behavior it reduces: DNS runtime, adapter egress, protocol forwarding, TUN/system proxy, fallback dependency, or removal of Go/Mihomo artifacts.
- Prefer 4-6 large implementation PRs over any new long sequence of numbered gates.

### Real fast-track sequence

| Order | Batch | Required implementation | Default impact |
| --- | --- | --- | --- |
| 1 | `rust-dns-runtime-parity` | Complete: Rust-owned dns/hosts patch synthesis, resolver/upstream selection, controlled resolver probe, unsupported fake-ip/fallback-filter/nameserver-policy blockers, explicit opt-in apply, and one-switch rollback. | Opt-in only; Mihomo remains default DNS until canary evidence passes. |
| 2 | `rust-adapter-egress-parity` | Complete: Rust-owned DIRECT/REJECT/proxy-group target decisions, adapter candidate compatibility checks, explicit opt-in proxy-groups/rules runtime patching, and one-switch rollback. | Opt-in for supported profiles only; Mihomo remains protocol/forwarding fallback. |
| 3 | `rust-protocol-forwarding-subset` | Complete: Rust-owned loopback TCP/HTTP accept loop, bidirectional byte forwarding, connection/session accounting, smoke evidence, stop/rollback surface, and Mihomo fallback for unsupported protocols. | Capped canary only after DNS + adapter parity. |
| 4 | `rust-tun-system-proxy-parity` | Complete: Rust-owned off/system-proxy/TUN route-mode decision, explicit opt-in apply, OS system-proxy path, TUN config/restart bridge, rollback record, and rollback apply. | No broad default until platform rollback passes. |
| 5 | `rust-runtime-real-canary` | Use the above implemented paths for real traffic in a capped canary profile; collect hold-window health, leak, rollback, and unsupported fallback evidence. | Limited default for canary profile. |
| 6 | `mihomo-fallback-retirement-execution` | Only after real parity exists, remove fallback dependence in the supported scope with an execution manifest, emergency rollback checkpoint, and post-execution verification. | Full replacement candidate for supported scope only. |

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
| DNS | Bounded opt-in parity path in progress | Keep collecting canary evidence; do not make DNS default until adapter/protocol/TUN rollback boundaries are ready. |
| Adapter / egress | Bounded opt-in parity path in progress | Keep canarying supported adapter decisions; move next to real protocol forwarding subset. |
| Protocol forwarding | Bounded opt-in Rust loopback TCP/HTTP subset in progress | Keep expanding only after canary evidence; TUN/system proxy remains next. |
| TUN / system proxy | Bounded Rust route-mode parity in progress | Packet capture still uses Mihomo/service; collect platform rollback/leak evidence before claiming replacement. |
| Mihomo fallback retirement | Readiness manifest in progress | Execution remains blocked until the manifest has canary evidence and emergency rollback artifacts. |

### Retained historical value

Keep the existing gate commands as safety/audit surfaces, but stop treating them as the main roadmap. They are prerequisites and evidence channels, not deliverables by themselves.

## Current Rust-owned capability inventory

These are control-plane and evidence capabilities unless explicitly called out as execution paths. They should not be counted as DNS/TUN/adapter/protocol replacement.

- Config schema validation and rule engine.
- Geodata, ASN, RULE-SET, and process metadata interpretation.
- Subscription artifact pipeline and active version management.
- App runtime state document, runtime plan, and projection artifact.
- Staged activation, active marker, and rollback boundary.
- Runtime-apply decision, preflight, audit, observed verification, closeout, and hold.
- DNS readiness, shadow evidence, limited opt-in execution, and repeated reverify history.
- Connection, traffic, memory, and log monitor app-facing event paths.
- Runtime upgrade gates and history.
- Lifecycle audit log.
- Sensitive-config audit.
- TLS fingerprint telemetry and rotation audit.
- Provider health, delay, and runtime wrapper result cache.
- Proxy and provider view-model boundary.
- Kernel runtime readiness, shadow evidence, and loopback-only R3 listener/platform evidence gates.

## Remaining blockers and acceleration boundaries

The next blocker is not another readiness gate; it is missing implementation. Do not retire Mihomo fallback or claim Rust data-plane replacement until all of these have landed as real code and tests:

- Rust DNS runtime parity for the supported subset, including leak tests and resolver/upstream behavior.
- Rust adapter/egress execution for supported DIRECT/REJECT/proxy paths.
- Rust protocol forwarding for a bounded real traffic subset, not only loopback smoke listeners.
- Platform TUN/system-proxy rollback and route restoration drills for Windows, macOS, and Linux.
- Connection/session accounting parity for traffic handled by Rust.
- Mihomo fallback that preserves connectivity without app restart for every unsupported path.
- Post-canary hold evidence that covers DNS leaks, fallback triggers, rollback, and health telemetry.

These blockers allow one useful next PR: `rust-runtime-real-canary`. They block fallback retirement execution, full protocol replacement, and any claim that packet capture is Rust-owned.

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

Allowed only through the corrected real fast-track sequence above. The current next batch is `rust-runtime-real-canary`; do not open fallback-retirement execution until DNS, adapter, protocol, and TUN/system-proxy opt-in parity have canary evidence.

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
