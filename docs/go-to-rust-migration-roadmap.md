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
| Production data plane | Rust default cutover supported for the safe profile; fallback retirement gated | The supported profile can select Rust by default after R6/R7 evidence; Mihomo fallback retirement remains blocked unless protocol/TUN/adapter/DNS parity, rollback drills, soak evidence, and emergency rollback all pass. |
| Kernel replacement track | Go/Mihomo post-execution verification complete | `get_runtime_kernel_loopback_go_mihomo_retirement_post_execution_verification` requires execution evidence, Rust-only boundary verification, retained rollback checkpoint, source/artifact removal verification, fallback IPC absence verification, and final verification decision before rollback surface retirement. |
| Next safe batch | `go-mihomo-retirement-rollback-surface-retirement` | Only after post-execution verification passes, plan retirement of rollback surfaces without weakening recovery boundaries. |

## Acceleration plan

The prior R3-R5 path intentionally used many small safety gates. That proved safety, but it is now too slow for the actual Go-to-Rust switch. From this point forward, stop creating standalone PRs that only add another read-only evidence command unless that PR also closes a phase or introduces a real Rust cutover surface.

### Accelerated completion target

The first "fully cut to Rust" milestone means the app selects a Rust-owned kernel runtime by default for the supported safe data-plane subset, while unsupported protocol/TUN/adapter paths still fall through to Mihomo without app restart or connectivity loss. Full replacement of every Mihomo protocol stack is a later hardening phase, not a blocker for the first Rust-default milestone.

Required properties for that first Rust-default milestone:

```text
RustKernelRuntime selected by default
  -> Rust-owned rule / DNS / adapter decision path for supported traffic
  -> MihomoFallbackRuntime for unsupported protocols, TUN, and emergency rollback
  -> explicit audit + health + rollback state
  -> one-switch rollback to Mihomo default
```

### Batch-size rule from now on

- No more single-gate roadmap PRs.
- Each PR must either add a real Rust runtime/cutover capability or close multiple remaining gates at once.
- Prefer 3-5 large PRs over another long chain of evidence-only batches.
- Keep safety booleans explicit, but remove duplicate "readiness of readiness" steps.
- If a gate only restates already-captured R5 evidence, fold it into the next implementation PR.

### Fast-track PR sequence

| Order | Batch | Purpose | Default impact |
| --- | --- | --- | --- |
| 1 | `r5-closeout-r6-rust-runtime-scaffold` | Bundle the R5 closeout report with `RustKernelRuntime`/runtime-selection scaffolding, fallback boundary, and frontend/IPC types. | No default change. |
| 2 | `r6-opt-in-rust-runtime-mvp` | Implement the Rust-owned supported subset behind explicit opt-in: rule/DNS/adapter decision path, direct/local forwarding surface, health telemetry, and Mihomo fallback. | Complete; opt-in only. |
| 3 | `r6-rust-default-canary` | Make Rust runtime default for a capped safe canary profile with automatic fallback on health/rollback triggers. | Complete; limited default for canary profile. |
| 4 | `r7-rust-default-cutover` | Complete: promote Rust runtime to default for the supported profile after canary closeout; keep Mihomo fallback for unsupported protocols/TUN until parity is complete. | Complete; Rust default for supported profile. |
| 5 | `r7-mihomo-fallback-retirement` | Complete: gate fallback dependence removal behind protocol/TUN/adapter/DNS parity, cross-platform rollback drills, soak evidence, explicit retirement decision, and emergency rollback. | Full replacement candidate, blocked by default. |
| 6 | `full-rust-runtime-hardening` | Complete: gate full Rust runtime hardening behind R7 fallback retirement readiness, extended soak, rollback telemetry, platform hardening follow-up, and explicit final decision. | Full Rust hardening candidate, blocked by default. |
| 7 | `go-mihomo-retirement-audit` | Complete: inventory remaining Go/Mihomo source, bundled artifacts, fallback IPC, docs/runbooks, and retained emergency rollback before any removal plan. | Audit only; no removal. |
| 8 | `go-mihomo-retirement-plan` | Complete: plan source removal, bundled artifact deprecation, IPC fallback replacement, emergency rollback preservation, and release rollout before any execution guard. | Plan only; no removal. |
| 9 | `go-mihomo-retirement-execution-guard` | Complete: require removal manifest, abort plan, staged rollout guard, emergency rollback drill, operator acknowledgement, and final guard decision before any dry-run. | Guard only; no removal. |
| 10 | `go-mihomo-retirement-dry-run` | Complete: replay removal manifest and verify no source/artifact mutations, rollback rehearsal, archived evidence, and final dry-run decision before closeout. | Dry-run only; no removal. |
| 11 | `go-mihomo-retirement-closeout` | Complete: review dry-run evidence, archive closeout report, verify rollback checkpoint, freeze artifact inventory, prove no removal mutations, and require final closeout decision. | Closeout only; no removal. |
| 12 | `go-mihomo-retirement-final-removal-gate` | Complete: accept closeout evidence, lock rollback boundary, lock removal scope, pass release blocker review, require final operator approval, and explicit final removal decision. | Final gate only; no removal. |
| 13 | `go-mihomo-retirement-execution` | Complete: require final removal gate, rollback checkpoint, execution manifest application, source/artifact removal records, post-execution validation, and final execution decision. | Execution evidence only; no direct runtime mutation. |
| 14 | `go-mihomo-retirement-post-execution-verification` | Complete: verify execution evidence, Rust-only boundary, retained rollback checkpoint, source/artifact removal, fallback IPC absence, and final verification decision. | Verification only; no rollback retirement. |

### Completed R7 PR scope

The R7 cutover PR does not retire Mihomo fallback or TUN/protocol boundaries. It includes:

- Canary closeout summary from `get_runtime_kernel_loopback_r6_rust_default_canary`.
- Wider default selection only for the supported profile after canary health and rollback hold pass.
- A one-switch rollback path that restores Mihomo default selection without app restart.
- IPC/TypeScript types for querying cutover readiness and fallback state.
- Roadmap advancement directly to `r7-rust-default-cutover` (complete).

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

Default behavior remains Mihomo-backed until a specific phase explicitly changes it. The remaining migration now follows the acceleration plan above: one closeout/scaffold PR, then Rust runtime MVP, canary default, and production default.

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
| R3 loopback DNS smoke evidence | Complete | Bounded local runtime mutation only | `get_runtime_kernel_loopback_dns_smoke_evidence` binds a temporary loopback UDP DNS socket, answers one synthetic query locally, and compares runtime config/system proxy/TUN before and after. |
| R3 loopback forwarding preflight | Complete | Read-only | `get_runtime_kernel_loopback_forwarding_preflight` checks candidate listener/target loopback TCP ports and reports that future smoke evidence must not use outbound adapters. |
| R3 loopback forwarding smoke evidence | Complete | Bounded local runtime mutation only | `get_runtime_kernel_loopback_forwarding_smoke_evidence` forwards one synthetic HTTP request from a temporary 127.0.0.1 listener to a temporary 127.0.0.1 target and compares runtime config/system proxy/TUN before and after. |
| R3 loopback forwarding rollback drill | Complete | Bounded local runtime mutation only | `get_runtime_kernel_loopback_forwarding_rollback_drill` runs forwarding smoke evidence, then re-runs preflight to prove the loopback ports are released and runtime/TUN/system proxy state is unchanged. |
| R3 loopback forwarding leak check | Complete | Read-only | `get_runtime_kernel_loopback_forwarding_leak_check` checks candidate loopback ports are free and no isolated listener state remains running after rollback evidence. |
| R3 loopback platform matrix | Complete | Read-only | `get_runtime_kernel_loopback_platform_matrix` wraps loopback forwarding leak evidence with Windows/macOS/Linux matrix rows and records the current platform without allowing expanded opt-in. |
| R3 loopback hold window | Complete | Read-only | `get_runtime_kernel_loopback_hold_window` wraps platform matrix evidence with a time-window observation row while keeping expanded opt-in blocked. |
| R3 loopback platform rollback drills | Complete | Bounded local runtime mutation only | `get_runtime_kernel_loopback_platform_rollback_drills` wraps rollback drill evidence with Windows/macOS/Linux matrix rows while keeping expanded opt-in blocked. |
| R4 expanded opt-in preflight | Complete | Read-only | `get_runtime_kernel_loopback_r4_expanded_opt_in_preflight` checks hold-window, supplied platform rollback evidence, and explicit decision without enabling execution. |
| R4 expanded opt-in execution plan | Complete | Read-only | `get_runtime_kernel_loopback_r4_expanded_opt_in_execution_plan` returns a loopback-only execution sequence while keeping execution disabled. |
| R4 expanded opt-in execution guard | Complete | Read-only | `get_runtime_kernel_loopback_r4_expanded_opt_in_execution_guard` bundles guard checks plus verification and rollback plans while keeping default cutover disabled. |
| R4 synthetic execution closeout | Complete | Synthetic loopback only | `get_runtime_kernel_loopback_r4_expanded_opt_in_synthetic_execution` runs only guarded 127.0.0.1 rollback-drill evidence and immediate leak closeout when all guard inputs pass. |
| R4 post-execution hold | Complete | Synthetic loopback only | `get_runtime_kernel_loopback_r4_expanded_opt_in_post_execution_hold` requires a second hold window after synthetic closeout before any wider decision. |
| R4 decision readiness | Complete | Readiness only | `get_runtime_kernel_loopback_r4_expanded_opt_in_decision_readiness` combines post-execution hold and explicit wider decision while keeping expanded opt-in disabled. |
| R4 limited rollout gate | Complete | Readiness only | `get_runtime_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate` checks decision readiness, explicit limited-rollout decision, loopback-only canary scope, and session cap without starting rollout. |
| R4 rollout audit | Complete | Readiness only | `get_runtime_kernel_loopback_r4_expanded_opt_in_rollout_audit` records gate, rollback binding, and default-cutover boundary audit rows. |
| R4 closeout readiness | Complete | Readiness only | `get_runtime_kernel_loopback_r4_expanded_opt_in_closeout_readiness` combines rollout audit with an explicit closeout decision before the closeout report. |
| R4 closeout report | Complete | Readiness only | `get_runtime_kernel_loopback_r4_expanded_opt_in_closeout_report` summarizes R4 evidence while keeping production cutover blocked. |
| R4 completion summary | Complete | Readiness only | `get_runtime_kernel_loopback_r4_expanded_opt_in_completion_summary` records completed R4 batches and open production boundaries. |
| R4 next-phase handoff | Complete | Readiness only | `get_runtime_kernel_loopback_r4_expanded_opt_in_next_phase_handoff` requires explicit handoff before entering R5 preflight. |
| R4 expanded opt-in | Complete | Readiness only | R4 closes with synthetic loopback evidence only; default cutover remains blocked until a separate R5 phase. |
| R5 default cutover preflight | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_preflight` requires R4 handoff and explicit R5 preflight decision while keeping default cutover disabled. |
| R5 default cutover risk matrix | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_risk_matrix` catalogs default route, system proxy, TUN, protocol handler, and real adapter risks as blocked. |
| R5 rollback/abort plan | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_rollback_abort_plan` records abort criteria and rollback boundaries before any R5 execution plan. |
| R5 default cutover execution plan | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_execution_plan` defines dry-run-only execution order after rollback/abort planning. |
| R5 default cutover execution guard | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_guard` gates dry-run readiness behind execution plan and explicit guard decision. |
| R5 default cutover dry-run readiness | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_dry_run_readiness` scopes the next evidence batch to in-memory dry-run only. |
| R5 default cutover dry-run evidence | Complete | Synthetic only | `get_runtime_kernel_loopback_r5_default_cutover_dry_run_evidence` validates the R5 cutover path as in-memory intent only. |
| R5 default cutover dry-run closeout | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_dry_run_closeout` verifies the synthetic dry run left runtime and fallback state unchanged. |
| R5 default cutover post-dry-run hold | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_post_dry_run_hold` requires bounded post-dry-run observation before decision readiness. |
| R5 default cutover decision readiness | Complete | Readiness only | `get_runtime_kernel_loopback_r5_default_cutover_decision_readiness` summarizes post-dry-run hold evidence before final gate evaluation. |
| R5 default cutover final gate | Complete | Readiness only | `get_runtime_kernel_loopback_r5_default_cutover_final_gate` keeps default cutover blocked while permitting only final hold/rollback validation. |
| R5 default cutover next-step handoff | Complete | Readiness only | `get_runtime_kernel_loopback_r5_default_cutover_next_step_handoff` advances the safe batch to final hold evidence without enabling live default cutover. |
| R5 default cutover final hold | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_final_hold` requires a final observation window after final gate handoff. |
| R5 default cutover independent rollback validation | Complete | Read-only | `get_runtime_kernel_loopback_r5_default_cutover_independent_rollback_validation` verifies platform-complete rollback evidence after final hold. |
| R5 default cutover closeout readiness | Complete | Readiness only | `get_runtime_kernel_loopback_r5_default_cutover_closeout_readiness` prepares report-only closeout while keeping live default cutover blocked. |
| R5 closeout report + R6 Rust runtime scaffold | Complete | No default change | `get_runtime_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold` bundles final R5 closeout with `RustKernelRuntime` scaffolding and runtime selection boundaries; next batch is R6 MVP. |
| R6 opt-in Rust runtime MVP | Complete | Explicit opt-in only | `get_runtime_kernel_loopback_r6_opt_in_rust_runtime_mvp` selects Rust only after explicit opt-in, supported-subset decision ownership, loopback forwarding rollback evidence, health state, and Mihomo fallback readiness. |
| R6 Rust default canary | Complete | Limited default canary | `get_runtime_kernel_loopback_r6_rust_default_canary` selects Rust only inside the capped safe canary profile after R6 opt-in, canary decision, health, rollback, and automatic Mihomo fallback checks pass. |
| R7 Rust default cutover | Complete | Rust default for supported profile | `get_runtime_kernel_loopback_r7_rust_default_cutover` promotes Rust after canary closeout and rollback hold; Mihomo remains fallback for unsupported protocols/TUN and one-switch rollback. |
| R7 fallback retirement | Gate complete | Full replacement candidate | `get_runtime_kernel_loopback_r7_mihomo_fallback_retirement` retires fallback readiness only after R7 cutover, protocol/TUN/adapter/DNS parity, cross-platform rollback drills, soak evidence, explicit decision, and emergency rollback all pass. |
| Full Rust runtime hardening | Gate complete | Full Rust hardening candidate | `get_runtime_kernel_loopback_full_rust_runtime_hardening` closes the hardening gate only after R7 fallback retirement readiness, extended soak, rollback telemetry closeout, OS-specific hardening follow-up, and explicit final hardening decision all pass. |
| Go/Mihomo retirement audit | Gate complete | Audit only | `get_runtime_kernel_loopback_go_mihomo_retirement_audit` requires full Rust runtime hardening plus source/artifact/IPC/docs inventory and retained emergency rollback before advancing to a removal plan. |
| Go/Mihomo retirement plan | Gate complete | Plan only | `get_runtime_kernel_loopback_go_mihomo_retirement_plan` requires the audit plus source removal, artifact deprecation, IPC fallback replacement, rollback preservation, release rollout, and explicit final plan decisions before any execution guard. |
| Go/Mihomo retirement execution guard | Gate complete | Guard only | `get_runtime_kernel_loopback_go_mihomo_retirement_execution_guard` requires the plan plus removal manifest, abort plan, staged rollout guard, emergency rollback drill, operator acknowledgement, and final guard decisions before dry-run removal. |
| Go/Mihomo retirement dry run | Gate complete | Dry-run only | `get_runtime_kernel_loopback_go_mihomo_retirement_dry_run` requires the execution guard plus manifest replay, clean mutation checks, rollback rehearsal, archived evidence, and final dry-run decision before closeout. |
| Go/Mihomo retirement closeout | Gate complete | Closeout only | `get_runtime_kernel_loopback_go_mihomo_retirement_closeout` requires the dry run plus evidence review, archived closeout report, rollback checkpoint, frozen inventory, clean mutation checks, and final closeout decision before any final removal gate. |
| Go/Mihomo final removal gate | Gate complete | Final gate only | `get_runtime_kernel_loopback_go_mihomo_retirement_final_removal_gate` requires closeout plus accepted evidence, locked rollback boundary, locked removal scope, release blocker review, final operator approval, and explicit final removal decision before an execution batch. |
| Go/Mihomo retirement execution | Batch complete | Execution evidence only | `get_runtime_kernel_loopback_go_mihomo_retirement_execution` requires the final removal gate plus rollback checkpoint, execution manifest application, source/artifact removal records, post-execution validation, and explicit final execution decision before verification. |
| Go/Mihomo post-execution verification | Gate complete | Verification only | `get_runtime_kernel_loopback_go_mihomo_retirement_post_execution_verification` requires execution plus Rust-only boundary verification, retained rollback checkpoint, source/artifact removal verification, fallback IPC absence verification, and final verification decision before rollback surface retirement. |

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
- Kernel runtime readiness, shadow evidence, and loopback-only R3 listener/platform evidence gates.

## Remaining blockers and acceleration boundaries

The first Rust-default milestone can ship before full TUN/protocol replacement by keeping Mihomo fallback. Do not retire Mihomo fallback or claim full replacement until all of these exist:

- Mihomo fallback with no app restart and no connectivity loss.
- Platform-specific rollback drill for Windows service, sidecar, macOS, and Linux paths.
- Leak verification for DNS, TUN, system proxy, and direct/proxy egress.
- Adapter/protocol compatibility matrix.
- Repeated shadow evidence for rules, DNS, adapters, and connection/session shape.
- Opt-in execution history with hold windows and rollback closeout.
- Dedicated PR that does not include unrelated cleanup.

These blockers do not prevent the accelerated R6 Rust-default canary for the supported subset; they prevent fallback retirement and full protocol/TUN replacement.

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

Allowed only through the accelerated sequence above. The current branch is `go-mihomo-retirement-rollback-surface-retirement`; the next implementation may plan rollback surface retirement only after post-execution verification proves the Rust-only boundary, retained rollback checkpoint, source/artifact removal, fallback IPC absence, and final verification approval.

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
