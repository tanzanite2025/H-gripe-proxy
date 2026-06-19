# Go-to-Rust migration roadmap

This document is the compact operating roadmap for the Go/Mihomo to Rust migration. It intentionally keeps only durable architecture boundaries, current status, and future decision points. Per-batch implementation logs were removed; use the linked PRs and git history for detailed archaeology.

## Goal

Move control-plane ownership out of the Go/Mihomo sidecar and into the Tauri Rust layer. Rust should own validation, planning, gates, audit, telemetry, runtime-history records, and frontend type sources.

The target control chain is:

```text
App registry / policy / node pool / DNS / security profile
  -> Rust-owned runtime plan
  -> Rust-generated projection artifact
  -> explicit gate / audit / rollback boundary
  -> Mihomo data-plane apply bridge
  -> Rust-observed runtime state
```

## Current conclusion

The control-plane migration is effectively closed for the current phase:

- Config schema, rule parsing, subscription artifacts, app-runtime plans, staged artifacts, runtime gates, audit, upgrades, telemetry, sensitive-config audit, TLS rotation, and frontend type bindings are Rust-owned or Rust-generated.
- Mihomo remains the data plane for protocol stacks, TUN, transparent proxy, and real packet forwarding.
- Further work should be treated as a new decision: either stop at the current Rust-owned control plane, do low-risk cleanup, or open a separate high-risk data-plane migration plan.

## Non-negotiable boundaries

### 1. Rust is the control-plane source of truth

Do not add paths that bypass Rust state or Rust gates.

Forbidden patterns:

- UI writes Mihomo YAML directly.
- UI calls Mihomo mutation APIs directly.
- App policy, node pool, DNS, or security profile logic is assembled ad hoc in the frontend.
- Runtime mutation happens without an app-owned Rust command.
- Runtime mutation is not recorded in audit, history, closeout, or rollback state.

### 2. Mihomo remains the data plane

These areas stay owned by Mihomo/Go unless a dedicated high-risk design and PR series is approved:

- outbound / inbound protocol stacks
- TUN / transparent proxy
- real packet forwarding
- adapter / protocol runtime
- OS-level per-app network isolation / sandboxing

Do not mix these changes with UI cleanup, type cleanup, documentation cleanup, or telemetry-only PRs.

### 3. Runtime apply must stay explicitly gated

Any real runtime apply must preserve this chain:

```text
staged artifact
  -> checksum / boundary manifest
  -> explicit allow decision
  -> preflight guard
  -> runtime apply audit
  -> observed verification
  -> closeout / hold / rollback readiness
```

Readiness, shadow evidence, verification, or closeout records are not automatic rollout permission.

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

## Completed milestones

| Area | Status | Durable result |
| --- | --- | --- |
| Config validation | Complete | Rust native validator replaced the old `verge-mihomo -t` validation chain. |
| Rule engine | Complete | DOMAIN, CIDR, port, NETWORK, MATCH, GEOIP, GEOSITE, ASN, RULE-SET, process, UID, DSCP, inbound, wildcard, logical, and sub-rule paths are Rust-owned. |
| Control diagnostics | Complete | Rule explain, config diff, diagnostics summary, latency planner, and node selection planner are Rust-owned. |
| DNS planning | Complete | DNS explain and controlled probe planning exist in Rust; default DNS runtime is still protected by opt-in gates. |
| Subscription pipeline | Complete | Source config -> artifact -> active artifact -> runtime is transactional and Rust-owned. |
| App-facing monitor path | Complete | Connection, traffic, memory, and log views use Rust monitor controllers and Tauri events instead of frontend Mihomo WebSocket ownership. |
| App runtime orchestration | Complete to hold milestone | Runtime plan, projection artifact, staged activation, runtime-apply decision, verification closeout, and post-apply hold are Rust-owned. |
| Data-plane replacement | Not started | Protocol stacks, TUN, transparent proxy, and real forwarding remain Mihomo-owned. |

## Phase-2 data-plane control closeout

| Batch | Status | Result |
| --- | --- | --- |
| B1 | Complete | Lifecycle audit covers runtime mutations: mode, system proxy, TUN toggle, DNS apply, and geo update. |
| B2 | Cancelled | sub-rules commands were unused frontend dead code and did not need migration. |
| B3 | Complete | Proxy globals moved to app-owned view models backed by Rust-generated field sources. |
| B4 | Complete | Core, UI, and geo upgrades run through Rust gates and write upgrade history. |
| B5 | Complete | Engine, perf, buffer, hot-reload, XDP, and rule-traffic telemetry are exposed through Rust commands and diagnostics UI. |
| B6 | Complete | Sensitive config changes such as `secret` and `external-controller` are lifecycle-audited. |
| B7 | Complete | TLS fingerprint stats and forced rotation go through Rust commands, audit, and telemetry UI. |

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
- Frontend Mihomo runtime types via Rust-generated bindings plus app-owned UI view models.

## Removed from this document

The old roadmap contained detailed logs for many completed batches. Those details were useful while work was active, but they made the document harder to use as a current boundary reference.

Removed categories:

- Batch F through AI implementation details for artifacts, activation, runtime apply, and verification.
- Batch AJ through AL implementation details for closeout history and post-apply hold.
- Aggressive replacement track batch-by-batch notes.
- B1 through B7 per-file implementation notes.

These are now considered historical implementation records. Use PRs, commit history, or release notes when detailed archaeology is needed.

## Future decision points

### Option A: Stop here

Default recommendation. Treat the Rust-owned control plane as complete for this phase. Continue only with bug fixes, test coverage, observability polish, and small type-boundary cleanup.

### Option B: Low-risk maintenance cleanup

Allowed without a new high-risk design:

- Documentation and naming cleanup.
- Legacy migration block retention-window notes.
- UI type-boundary completion.
- Audit and telemetry display polish.
- Refactors that keep `mutatesRuntime=false`.

### Option C: High-risk data-plane migration

Replacing protocol stacks, TUN, transparent proxy, or real forwarding requires a separate design document and staged PR series. The active Phase 8 design starts in `docs/phase-8-rust-kernel-replacement.md`.

Minimum design requirements:

1. Exact runtime mutation scope.
2. Rollback strategy.
3. Mihomo sidecar coexistence or cutover boundary.
4. Observability metrics and failure criteria.
5. Hold window and rollback readiness.
6. One PR per real data-plane mutation.

## PR checklist for future changes

Every future PR touching this area must state:

- `mutatesRuntime=true/false`
- Whether it touches DNS, TUN, protocol, adapter, or forwarding paths.
- Whether it adds or bypasses a Rust-owned command gate.
- Whether it writes audit, history, closeout, or rollback records.
- Rollback path.
- Local verification commands.

If `mutatesRuntime=true`, also document preflight, observed verification, hold window, and rollback readiness.

## Document maintenance rules

- Keep this file as a boundary and decision document, not a PR log.
- Add completed work as a one-line milestone only.
- Put detailed implementation plans in separate design docs while active.
- Link separate high-risk design docs from here instead of appending large batch logs.
- Never remove the boundaries for Rust-owned gates, DNS opt-in, runtime apply verification, or Mihomo-owned TUN/protocol/forwarding without an explicit architectural decision.
