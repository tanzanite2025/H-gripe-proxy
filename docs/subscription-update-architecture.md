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
- `src-tauri/src/feat/profile.rs`
- `src-tauri/src/core/notification.rs`
- `src/pages/_layout/utils/notification-handlers.ts`

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

Subscription update should become an explicit pipeline:

1. `ResolveSource`
2. `ResolveTransportPlan`
3. `FetchPayload`
4. `DecodePayload`
5. `ValidateSubscriptionPayload`
6. `MaterializeArtifact`
7. `GenerateRuntimeConfigCandidate`
8. `ValidateRuntimeCandidate`
9. `PublishArtifact`
10. `ActivateRuntime`
11. `EmitFinalResult`

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

Suggested artifact directory contents:

- `raw.body`
- `normalized.yaml`
- `metadata.json`
- `diagnostics.json`

Publishing should atomically switch the active artifact pointer after validation succeeds.

## Runtime Activation Model

Fetching a subscription should not immediately overwrite the active profile content.

Instead:

1. Download candidate artifact.
2. Validate candidate subscription format.
3. Generate runtime config from candidate.
4. Validate runtime config.
5. If successful, atomically publish candidate as active.
6. Only then notify runtime/UI.

This makes rollback implicit:

- old active artifact remains active until new candidate is proven valid

No compensating snapshot restore should be required for the happy path architecture.

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
  pipeline.rs
  persist.rs
  activate.rs
  diagnostics.rs
  events.rs
```

Suggested responsibilities:

- `model.rs`: source, artifact, attempt, errors
- `transport.rs`: runtime/system transport resolution
- `fetch.rs`: HTTP execution and response metadata capture
- `format.rs`: payload detection and parsing
- `pipeline.rs`: orchestrates update stages
- `persist.rs`: state/artifact persistence
- `activate.rs`: publish and runtime activation
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

- Define `SubscriptionSource`, `SubscriptionArtifact`, `SubscriptionUpdateAttempt`.
- Introduce typed `UpdateError` and `UpdateStage`.
- Add structured event types alongside existing notice messages.

### Phase 1: Extract Transport and Fetch

- Move transport decision out of `feat/profile.rs`.
- Create `TransportPlan`.
- Centralize request execution and response capture.
- Preserve existing persistence for now.

### Phase 2: Extract Format Detection

- Move payload detection and parsing out of `PrfItem::from_url`.
- Add support for typed format results.
- Preserve Clash YAML path as first adapter.

### Phase 3: Introduce Candidate Artifact Persistence

- Write fetched payloads into artifact directories.
- Stop directly replacing active profile files during fetch.
- Generate runtime config from candidate artifacts.

### Phase 4: Publish/Activate Boundary

- Activate only after validation succeeds.
- Replace snapshot rollback with publish gating.
- Emit structured success/failure events.

### Phase 5: UI Migration

- Replace string notice handling with typed subscription event handling.
- Add update history and stage-specific diagnostics UI.

### Phase 6: Remove Legacy Coupling

- Reduce `PrfItem` to a UI projection or compatibility wrapper.
- Remove retry and notification logic from `feat/profile.rs`.
- Make subscription pipeline the only update path.

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

## Recommended First Implementation Slice

If we want the highest leverage first step, do this sequence:

1. add typed update result and stage enums
2. extract transport planning
3. extract fetch + response metadata capture
4. emit structured update events

That slice alone will already remove most ambiguity and make the next migration steps much safer.
