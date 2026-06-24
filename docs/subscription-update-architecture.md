# Subscription Update Architecture

## Goal

Replace the current ad hoc profile update flow with a first-class subscription subsystem that is:

- deterministic
- observable
- transactional
- format-aware
- transport-aware
- safe to evolve

This document describes the target architecture for fixing subscription update issues at the root instead of adding more retry branches and UI patches.

## Current Problems

The current implementation mixes several responsibilities into one flow:

- source definition
- network transport selection
- remote fetch
- payload parsing
- profile state persistence
- runtime config generation
- runtime activation
- user notification

As a result:

- source configuration and fetched artifact state are mixed in `PrfItem`
- transient network choices are encoded as profile option mutations
- fetch success and activation success are not modeled separately
- rollback is compensating logic instead of part of the update model
- frontend only receives coarse string notifications
- failures are hard to diagnose and easy to misreport

Relevant current code:

- `src-tauri/src/config/prfitem.rs`
- `src-tauri/src/config/profiles.rs`
- `src-tauri/src/app/subscription.rs`
- `src-tauri/src/core/notification.rs`
- `src/pages/_layout/utils/notification-handlers.ts`

## Implementation Progress

Current status:

| Area | Status | Notes |
| --- | --- | --- |
| Typed source config | Done | `SubscriptionSourceConfig` is stored in `subscriptions/state.yaml` and profile commands keep it current. |
| Transport planning | Done | `subscription::transport::TransportPlan` explains direct / local Mihomo / system proxy candidates. |
| Fetch / decode / artifact materialization | Done | Clash YAML payloads are fetched, format-detected, normalized, diagnosed, and written under artifact directories. |
| Artifact readers / cleanup | Done | Diagnostics, metadata, content, summaries, event timeline, and retention cleanup are exposed. |
| Typed executor/state machine | Done | `SubscriptionUpdateExecutor` owns source config → transport → fetch retry → decode → materialize artifact. |
| Runtime candidate validation | Done | Current subscription updates validate a generated runtime candidate from artifact + source config before publish. |
| Artifact publish pointer | Done | `PublishArtifact` cuts `active_artifact_version` after validation and rolls it back if activation later fails. |
| Active artifact consumption | Done | Current subscription updates validate candidate artifacts and activate runtime from `active_artifact_version` / `normalized.yaml`. |
| Runtime activation replacement | Done for update path | The success path feeds `runtime_lifecycle::update_runtime_config_with_restart_boundary(...)` through an active-artifact-backed runtime source. |
| Redundant storage removal | In progress | Remote update source-of-truth is now subscription state; `profiles.yaml` remains a UI selection/list adapter only. |

## Architecture Principles

1. Separate source-of-truth configuration from downloaded artifacts.
2. Make update execution a typed state machine.
3. Treat activation as a publish step, not a side effect of fetch.
4. Move transport selection into a dedicated capability layer.
5. Emit structured events and structured diagnostics.
6. Prefer one canonical fact chain over duplicate writes.

## Domain Model

### 1. Subscription Source

Represents a user-managed remote subscription definition.

Suggested shape:

```rust
pub struct SubscriptionSource {
    pub id: String,
    pub display_name: String,
    pub url: String,
    pub enabled: bool,
    pub auto_update: AutoUpdatePolicy,
    pub fetch_policy: FetchPolicy,
    pub expected_format: SubscriptionFormatHint,
    pub metadata: SubscriptionMetadata,
}
```

`FetchPolicy` should include:

- `transport_preference`
- `timeout_seconds`
- `user_agent`
- `accept_invalid_certs`
- `auth`
- `redirect_policy`

`transport_preference` should be declarative, not expressed by mutating `with_proxy` and `self_proxy` on every run.

### 2. Subscription Artifact

Represents the most recent fetched result, independent from the source definition.

```rust
pub struct SubscriptionArtifact {
    pub source_id: String,
    pub version: String,
    pub fetched_at: i64,
    pub content_hash: String,
    pub detected_format: SubscriptionFormat,
    pub response_meta: ResponseMetadata,
    pub artifact_paths: ArtifactPaths,
}
```

This allows the app to answer:

- what was last fetched
- when it was fetched
- which transport succeeded
- which payload format was returned
- which artifact is currently active

### 3. Update Attempt

Represents one execution of the pipeline.

```rust
pub struct SubscriptionUpdateAttempt {
    pub attempt_id: String,
    pub source_id: String,
    pub trigger: UpdateTrigger,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub stages: Vec<StageRecord>,
    pub final_status: UpdateFinalStatus,
}
```

This is the basis for diagnostics, history, and future UI.

## Execution Model

Subscription update is becoming an explicit pipeline:

1. `ResolveSource`
2. `ResolveTransportPlan`
3. `FetchPayload`
4. `DecodePayload`
5. `MaterializeArtifact`
6. `GenerateRuntimeConfigCandidate`
7. `ValidateRuntimeCandidate`
8. `PublishArtifact`
9. `ActivateRuntime`
10. `EmitFinalResult`

`ValidateSubscriptionPayload` remains a useful conceptual boundary, but the current implementation folds Clash YAML validation into `DecodePayload` / `MaterializeArtifact`.

Each stage should return a typed result:

```rust
pub enum UpdateStageResult<T> {
    Success(T),
    RetryableFailure(UpdateError),
    TerminalFailure(UpdateError),
}
```

This replaces the current branch-heavy logic in `feat/profile.rs`.

## Transport Capability Layer

Transport selection should be moved into a dedicated module, for example:

- `src-tauri/src/subscription/transport.rs`

Suggested interfaces:

```rust
pub enum TransportKind {
    Direct,
    LocalProxy { host: String, port: u16 },
    SystemProxy { proxy_url: String },
}

pub struct TransportCandidate {
    pub kind: TransportKind,
    pub reason: String,
}

pub struct TransportPlan {
    pub ordered_candidates: Vec<TransportCandidate>,
}
```

Inputs should include:

- source fetch policy
- current runtime port configuration
- runtime reachability
- system proxy state
- TUN availability if relevant

The plan builder should fail early when a candidate is impossible, instead of waiting for a request error.

## Format Negotiation

The subsystem must distinguish between:

- Clash YAML
- base64 link subscription
- sing-box style payload
- HTML/login/error page
- unknown text payload

Suggested types:

```rust
pub enum SubscriptionFormatHint {
    Auto,
    ClashYaml,
    Base64Links,
    SingBox,
}

pub enum SubscriptionFormat {
    ClashYaml,
    Base64Links,
    SingBox,
    Html,
    UnknownText,
}
```

Detection should happen before persistence. Parsing should be delegated to format adapters instead of embedding YAML assumptions directly in `PrfItem::from_url`.

## Persistence Model

### Current issue

Current persistence stores both source definition and fetched state in `profiles.yaml`, while payload content is stored directly in profile files.

### Target

Split persistence into:

- `profiles.yaml` or successor config for profile/source definitions
- `subscriptions/state.yaml` for update history and activation pointers
- `subscriptions/artifacts/<source_id>/<version>/...` for fetched payloads and derived files

Current artifact directory contents:

- `raw.body`
- `normalized.yaml`
- `metadata.yaml`
- `diagnostics.yaml`

Publishing should atomically switch the active artifact pointer after validation succeeds.

## Runtime Activation Model

Fetching a subscription should not immediately overwrite the active profile content.

Target model:

1. Download candidate artifact.
2. Validate candidate subscription format.
3. Generate runtime config from candidate.
4. Validate runtime config.
5. If successful, atomically publish candidate as active.
6. Only then notify runtime/UI.

This makes rollback implicit:

- old active artifact remains active until new candidate is proven valid

No compensating snapshot restore should be required for the happy path architecture.

Current model:

1. Profile import/create/patch/delete writes `subscriptions/state.yaml.sources[*].source_config`.
2. `SubscriptionUpdateExecutor` reads source config, downloads, decodes, and materializes the artifact candidate.
3. If the updated source is current, the app writes a temporary runtime candidate file from artifact content and validates the generated runtime config with the Rust native validator.
4. `PublishArtifact` updates `subscriptions/state.yaml.sources[*].active_artifact_version`.
5. Runtime activation consumes `active_artifact_version` + `source_config`.
6. If activation fails, the code restores only the previous active artifact pointer.

There is no successful-update profile rewrite in this chain.

## Event Protocol

Current frontend notifications are stringly typed. Replace them with structured events.

Suggested event family:

```rust
pub enum SubscriptionEvent {
    UpdateQueued {
        source_id: String,
        attempt_id: String,
        trigger: UpdateTrigger,
    },
    StageChanged {
        source_id: String,
        attempt_id: String,
        stage: UpdateStage,
        transport: Option<TransportKindView>,
    },
    UpdateFailed {
        source_id: String,
        attempt_id: String,
        stage: UpdateStage,
        error: UpdateErrorView,
        active_artifact_unchanged: bool,
    },
    UpdateSucceeded {
        source_id: String,
        attempt_id: String,
        artifact_version: String,
        runtime_activated: bool,
    },
}
```

Frontend should render messages from structured payloads instead of interpreting opaque status strings.

## Canonical State Chain

### Read path

- update reads source config from `subscriptions/state.yaml`
- runtime reads active artifact content from `subscriptions/artifacts/<source_id>/<version>/normalized.yaml`
- UI status reads structured attempts/events from subscription state

### Write path

- profile commands write `source_config`
- update writes immutable artifacts, attempt history, and `active_artifact_version`
- update does not rewrite profile item payloads

### Remaining adapter

`profiles.yaml` can still drive the visible profile list/current selection until remote subscription source listing moves fully to subscription state, but it is not the subscription update source-of-truth.

## Proposed Module Layout

Suggested new backend modules:

```text
src-tauri/src/subscription/
  mod.rs
  model.rs
  fetch.rs
  format.rs
  transport.rs
  executor.rs
  persist.rs
  runtime_candidate.rs
  activate.rs          # pending
  diagnostics.rs
  events.rs
```

Suggested responsibilities:

- `model.rs`: source, artifact, attempt, errors
- `transport.rs`: runtime/system transport resolution
- `fetch.rs`: HTTP execution and response metadata capture
- `format.rs`: payload detection and parsing
- `executor.rs`: orchestrates update stages
- `persist.rs`: state/artifact persistence
- `runtime_candidate.rs`: transitional runtime candidate validation
- `activate.rs`: publish and runtime activation boundary (pending extraction)
- `diagnostics.rs`: structured logs and history records
- `events.rs`: frontend event payloads

## Frontend Changes

The frontend should stop treating subscription updates as a single promise with a string error.

Recommended additions:

- per-profile update status badge
- last successful update timestamp
- last failure stage and reason
- expanded diagnostics panel
- explicit distinction between:
  - fetch failed
  - parse failed
  - validation failed
  - activation failed
  - activation skipped because current active artifact remains valid

The current `IProfileItem` UI can remain initially, but update status should come from subscription state, not inferred from `updated` alone.

## Incremental Rollout Plan

### Phase 0: Stabilize Contracts

- Define `SubscriptionSource`, `SubscriptionArtifact`, `SubscriptionUpdateAttempt`. **Done.**
- Introduce typed `UpdateStage`. **Done.**
- Add structured event types alongside existing notice messages. **Mostly done; frontend still renders some existing notifications.**

### Phase 1: Extract Transport and Fetch

- Move transport decision out of `app::subscription`. **Done.**
- Create `TransportPlan`. **Done.**
- Centralize request execution and response capture. **Done in `SubscriptionUpdateExecutor`; fetch implementation still reuses existing HTTP helper.**
- Preserve existing persistence for now. **Superseded: update source/config/artifact state now lives in `subscriptions/state.yaml` and artifact directories.**

### Phase 2: Extract Format Detection

- Move payload detection and parsing out of `PrfItem::from_url`. **Done for update artifact path.**
- Add support for typed format results. **Done.**
- Preserve Clash YAML path as first adapter. **Done.**

### Phase 3: Introduce Candidate Artifact Persistence

- Write fetched payloads into artifact directories. **Done.**
- Stop directly replacing active profile files during fetch. **Done: successful updates publish artifacts and do not rewrite profile payloads.**
- Generate runtime config from candidate artifacts. **Done for current update path through a temporary runtime adapter profile file generated from artifact content.**

### Phase 4: Publish/Activate Boundary

- Activate only after validation succeeds. **Done for current subscription updates.**
- Replace snapshot rollback with publish gating. **Done for update path: rollback restores only the active artifact pointer.**
- Emit structured success/failure events. **Done for current update flow.**

### Phase 4.5: Consume Active Artifact

Implemented:

- Read `active_artifact_version` from `subscriptions/state.yaml`.
- Load `subscriptions/artifacts/<source_id>/<version>/normalized.yaml`.
- Generate the runtime candidate from the active artifact rather than from a rewritten profile file.
- Preserve rollback behavior at the artifact boundary: activation failure leaves the previous active artifact pointer in place.

### Phase 5: UI Migration

- Replace string notice handling with typed subscription event handling. **Partially done: subscription notices now share structured stage/transport labels, profile cards render live and persisted status from subscription state/events, and subscription event queries invalidate from typed events.**
- Add update history and stage-specific diagnostics UI. **Partially done: profile context menus expose an update-history dialog with the latest structured attempt timeline plus raw body / normalized YAML / diagnostics artifact previews.**
- Add UI tests for stage-specific fetch / parse / validation / activation failures. **Done: `SubscriptionUpdateHistoryDialog` has Vitest coverage for these structured failure stages.**

### Phase 6: Remove Legacy Coupling

- Reduce `PrfItem` to a UI/runtime adapter only. **Done for the update path: fetch reads `SubscriptionSourceConfig` from `subscriptions/state.yaml`; artifact publication owns active versions; runtime activation consumes the active artifact plus source config.**
- Remove retry and notification logic from `app::subscription`. **Done: `app::subscription` is a thin command entrypoint; update orchestration, stage events, final notifications, attempt persistence, and artifact rollback live in `subscription::orchestration`.**
- Make subscription pipeline the only update path. **Done for current remote updates: profile create/import/patch/delete commands write subscription state directly, update no longer syncs from `profiles.yaml`, and successful fetch no longer writes duplicate profile payloads.**

The single fact chain is now:

```text
profile command → subscriptions/state.yaml source_config
source_config → fetch/decode → immutable artifact
artifact publish → active_artifact_version
active_artifact_version + source_config → runtime candidate → runtime activation
```

`profiles.yaml` remains only the existing UI selection/list adapter until the frontend model is fully moved; it is not a subscription update source of truth.

## Testing Strategy

Add scenario-based tests for:

- direct success
- direct failure, local proxy success
- direct failure, system proxy success
- unreachable local proxy
- system proxy configured but unusable
- HTML payload returned
- base64 link payload returned
- YAML missing required top-level keys
- runtime validation failure after successful fetch
- activation failure with old artifact preserved
- source state migration/removal

Tests should cover pipeline stage outputs, persisted artifacts, and emitted structured events.

## Success Criteria

The redesign is complete when:

- subscription source config never mutates due to retry behavior
- fetch success and activation success are independently visible
- active runtime is never overwritten by an unvalidated artifact
- users can see the exact stage and reason of failure
- transport decisions are reproducible and diagnosable
- future subscription formats can be added without rewriting profile logic

## Recommended Next Implementation Slice

The next highest-leverage slice is removing the remaining UI dependency on `profiles.yaml` for remote subscription source listing/selection.

1. Expose subscription source list/edit commands backed by `subscriptions/state.yaml`.
2. Promote diagnostics previews into file/open-location deep links if needed.
