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

| Area | State | Boundary |
| --- | --- | --- |
| Rust control plane | Complete for the current migration phase | Validation, planning, gates, audit, telemetry, upgrade history, sensitive-config audit, TLS rotation, and frontend type sources are Rust-owned or Rust-generated. |
| Production data plane | Still Mihomo-owned | Protocol stacks, adapter runtime, TUN, transparent proxy, DNS default runtime, and real forwarding remain Mihomo-owned by default. |
| Kernel replacement track | Phase 8 R3 in progress | R0/R1 seams, R2 shadow evidence, listener smoke evidence, and loopback DNS preflight are complete. Current step adds loopback DNS smoke evidence; it is still non-default. |
| Next safe batch | `loopback-forwarding-preflight-decision` | Decide whether to preflight a still-isolated forwarding path; TUN/protocol/default cutover remain blocked. |

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

These areas stay owned by Mihomo/Go unless a dedicated high-risk PR series explicitly changes them:

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

Phase 8 is not a direct Go/Mihomo kernel swap. The safe sequence is:

```text
inventory current Mihomo kernel seams
  -> introduce Rust kernel runtime capability boundaries
  -> shadow Rust components without forwarding traffic
  -> opt-in isolated execution
  -> observed verification + rollback drill
  -> expanded opt-in
  -> default cutover only after hold windows pass
```

Default behavior remains Mihomo-backed until a specific phase explicitly changes it.

### Phase 8 status

| Batch | Status | Runtime impact | Commands / evidence |
| --- | --- | --- | --- |
| R0 kernel seam inventory | Complete | Read-only | `get_runtime_kernel_replacement_readiness` reports Mihomo-owned vs Rust-owned areas. |
| R1 kernel runtime seam | Complete | No default behavior change | `KernelRuntime` exists; the only implementation delegates to `MihomoKernelRuntime`. |
| R2 DNS shadow evidence | Complete | Read-only | `get_runtime_kernel_dns_shadow_evidence` wraps existing DNS shadow evidence. |
| R2 rule shadow evidence | Complete | Read-only | `get_runtime_kernel_rule_shadow_evidence` compares app runtime rule projection with Mihomo rule inventory. |
| R2 adapter capability evidence | Complete | Read-only | `get_runtime_kernel_adapter_capability_report` compares proxy/adapter inventory without dialing endpoints. |
| R2 connection/session shape evidence | Complete | Read-only | `get_runtime_kernel_connection_session_shadow` summarizes Mihomo connection shape without closing or migrating sessions. |
| R3 isolated listener preflight | Complete | Read-only | `get_runtime_kernel_isolated_listener_preflight` checks loopback port readiness and runtime-port overlap. |
| R3 loopback test listener opt-in | Complete | Explicit opt-in, non-production only | `start_runtime_kernel_isolated_test_listener`, `get_runtime_kernel_isolated_test_listener_status`, and `stop_runtime_kernel_isolated_test_listener` gate a local 204-only listener. |
| R3 listener smoke evidence | Complete | Bounded local runtime mutation only | `get_runtime_kernel_isolated_test_listener_smoke_evidence` starts the listener, sends a local request, verifies status increment, stops it, and compares system proxy/TUN/runtime config before and after. |
| R3 loopback DNS preflight | Complete | Read-only | `get_runtime_kernel_loopback_dns_preflight` checks loopback UDP/TCP candidate port readiness and reports DNS/TUN/system proxy context without replacing default DNS. |
| R3 loopback DNS smoke evidence | In progress | Bounded local runtime mutation only | `get_runtime_kernel_loopback_dns_smoke_evidence` binds a temporary loopback UDP DNS socket, answers one synthetic query locally, and compares runtime config/system proxy/TUN before and after. |
| R4 expanded opt-in | Blocked | Not allowed yet | Requires loopback DNS smoke evidence or another isolated execution evidence path, rollback drill, leak checks, platform matrix, and hold window. |
| R5 default cutover | Blocked | Not allowed yet | Must be a dedicated PR after all high-risk areas have independent evidence and rollback. |

### Current R3 loopback listener boundary

Allowed behavior:

- bind only `127.0.0.1`
- require preflight before start
- return a local `204 No Content`
- report status, accepted connection count, and safety flags
- stop through an app-owned command

Forbidden behavior:

- no system proxy or TUN mutation
- no Mihomo config patch
- no DNS hijack or default route
- no outbound adapter dialing
- no packet forwarding
- no production traffic ownership

`KernelIsolatedTestListenerStatus` must continue to report:

```text
loopbackOnly=true
defaultRoute=false
forwardsTraffic=false
mihomoFallback=true
```

## Current Rust-owned capability inventory

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
- Kernel runtime readiness, shadow evidence, and loopback-only R3 listener gates.

## Hard blockers before production TUN/protocol replacement

Do not replace TUN, adapter runtime, protocol stacks, or real forwarding until all of these exist:

- Mihomo fallback with no app restart and no connectivity loss.
- Platform-specific rollback drill for Windows service, sidecar, macOS, and Linux paths.
- Leak verification for DNS, TUN, system proxy, and direct/proxy egress.
- Adapter/protocol compatibility matrix.
- Repeated shadow evidence for rules, DNS, adapters, and connection/session shape.
- Opt-in execution history with hold windows and rollback closeout.
- Dedicated PR that does not include unrelated cleanup.

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
- add read-only evidence commands
- improve audit/history rendering

### Option C: Continue high-risk data-plane migration

Allowed only after isolated R3 evidence and explicit decision. The current branch is `loopback-dns-smoke-evidence`; any forwarding path still needs a separate preflight decision. TUN/protocol/default cutover remain blocked.

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
