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

Status after PR #60:

| Area | Status | Notes |
| --- | --- | --- |
| Typed source view | Done | `subscription::source` projects legacy remote profiles into read-only `SubscriptionSource` records. |
| Transport planning | Done | `subscription::transport::TransportPlan` explains direct / local Mihomo / system proxy candidates. |
| Fetch / decode / artifact materialization | Done | Clash YAML payloads are fetched, format-detected, normalized, diagnosed, and written under artifact directories. |
| Artifact readers / cleanup | Done | Diagnostics, metadata, content, summaries, event timeline, and retention cleanup are exposed. |
| Typed executor/state machine | Done | `SubscriptionUpdateExecutor` owns source → transport → fetch retry → decode → materialize → legacy item generation. |
| Runtime candidate validation | Done | Current subscription updates validate a generated runtime candidate before committing the legacy profile write. |
| Artifact publish pointer | Done | `PublishArtifact` cuts `active_artifact_version` after validation and rolls it back if legacy activation later fails. |
| Active artifact consumption | Done | Current subscription updates validate candidate artifacts and activate runtime from `active_artifact_version` / `normalized.yaml`; legacy files remain compatibility views. |
| Runtime activation replacement | In progress | The success path now feeds `CoreManager::update_config_without_restart_with_force(...)` through an active-artifact-backed runtime source; the CoreManager boundary itself remains. |
| Legacy storage removal | Not started | `PrfItem` / `profiles.yaml` remain compatibility and source-definition storage. |

## Architecture Principles

1. Separate source-of-truth configuration from downloaded artifacts.
2. Make update execution a typed state machine.
3. Treat activation as a publish step, not a side effect of fetch.
4. Move transport selection into a dedicated capability layer.
5. Emit structured events and structured diagnostics.
6. Preserve backward compatibility during migration.

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

Current transitional model after PR #60:

1. `SubscriptionUpdateExecutor` downloads, decodes, and materializes the artifact candidate.
2. If the updated source is current, the app writes a temporary candidate profile file and validates the generated runtime config with the Rust native validator.
3. The legacy profile write is committed only after candidate validation succeeds.
4. `PublishArtifact` updates `subscriptions/state.yaml.sources[*].active_artifact_version`.
5. Existing legacy runtime activation still runs through `CoreManager::update_config_without_restart_with_force(...)`.
6. If legacy activation fails, the code restores both the legacy profile snapshot and the previous active artifact pointer.

This is intentionally not the final model: runtime generation still needs to consume the active artifact directly.

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

## Backward Compatibility

Migration must preserve existing users and files.

### Read path

During transition:

- continue reading legacy `IProfileItem`
- derive `SubscriptionSource` from remote `type: remote` entries
- derive initial artifact pointer from existing profile files

### Write path

Phase 1 may continue writing legacy structures while also writing new state files.

### Final phase

Once stable:

- `PrfItem` becomes a compatibility view model
- source and artifact persistence become canonical

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
- Add structured event types alongside existing notice messages. **Mostly done; frontend still renders compatibility notifications.**

### Phase 1: Extract Transport and Fetch

- Move transport decision out of `app::subscription`. **Done.**
- Create `TransportPlan`. **Done.**
- Centralize request execution and response capture. **Done in `SubscriptionUpdateExecutor`; fetch implementation still reuses existing HTTP helper.**
- Preserve existing persistence for now. **Done during transition.**

### Phase 2: Extract Format Detection

- Move payload detection and parsing out of `PrfItem::from_url`. **Done for update artifact path.**
- Add support for typed format results. **Done.**
- Preserve Clash YAML path as first adapter. **Done.**

### Phase 3: Introduce Candidate Artifact Persistence

- Write fetched payloads into artifact directories. **Done.**
- Stop directly replacing active profile files during fetch. **Partially done: validation happens before write, but successful updates still write legacy profile files.**
- Generate runtime config from candidate artifacts. **Partially done through temporary legacy-compatible candidate profile.**

### Phase 4: Publish/Activate Boundary

- Activate only after validation succeeds. **Done for current subscription updates.**
- Replace snapshot rollback with publish gating. **Partially done: publish is gated and rollback restores active artifact, but legacy profile snapshot rollback remains.**
- Emit structured success/failure events. **Partially done.**

### Phase 4.5: Consume Active Artifact

Implemented:

- Read `active_artifact_version` from `subscriptions/state.yaml`.
- Load `subscriptions/artifacts/<source_id>/<version>/normalized.yaml`.
- Generate the runtime candidate from the active artifact rather than from the just-written legacy profile file.
- Keep `profiles.yaml` / profile files in sync as compatibility views.
- Preserve rollback behavior: activation failure leaves the previous active artifact pointer in place.

### Phase 5: UI Migration

- Replace string notice handling with typed subscription event handling. **Partially done: subscription notices now share structured stage/transport labels, profile cards render live and persisted status from subscription state/events, and subscription event queries invalidate from typed events.**
- Add update history and stage-specific diagnostics UI. **Partially done: profile context menus expose an update-history dialog with the latest structured attempt timeline plus raw body / normalized YAML / diagnostics artifact previews.**
- Add UI tests for stage-specific fetch / parse / validation / activation failures. **Done: `SubscriptionUpdateHistoryDialog` has Vitest coverage for these structured failure stages.**

### Phase 6: Remove Legacy Coupling

- Reduce `PrfItem` to a UI projection or compatibility wrapper. **Partially done: runtime activation now consumes an explicit subscription runtime projection plus the active artifact, while legacy `PrfItem` materialization is kept as a compatibility projection.**
- Remove retry and notification logic from `app::subscription`. **Done for current update flow: `app::subscription` is now a thin command entrypoint; top-level update orchestration, final notifications, attempt persistence, stage notifications, and compatibility rollback helpers live in `subscription::orchestration`.**
- Make subscription pipeline the only update path. **Partially done: subscription updates use the pipeline artifact as the activation input, and legacy compatibility writes are committed after artifact publish with active-artifact rollback on failure.**

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
- legacy profile migration

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

The next highest-leverage slice is Phase 6: remove legacy coupling once the team is comfortable with the Phase 5 UI.

1. Extend subscription source configuration beyond the legacy `profiles.yaml` projection.
2. Promote diagnostics previews into file/open-location deep links if needed.
