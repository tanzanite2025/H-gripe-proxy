# Ingress Countermeasure And Egress Stability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an ingress-first threat classification and deception pipeline that upgrades anti-probe, obfuscation, and honeypot behavior, while adding lightweight egress-stability support for suspicious and hostile flows.

**Architecture:** Add a new ingress countermeasure subsystem in the Tauri backend that classifies flows into `Normal`, `Suspicious`, and `Hostile`, routes them into persona-based obfuscation or deception behavior, and exposes the resulting configuration and runtime status through `AdvancedConfig`, coordinator, and the existing advanced security UI. Keep egress changes minimal by layering policy hints on top of stable egress, session affinity, and egress monitor instead of redesigning those systems.

**Tech Stack:** Rust (`serde`, `tokio`, existing Tauri backend modules), TypeScript/React, existing advanced config and contract-test patterns, Node `--test`, `pnpm typecheck`, focused Rust tests.

---

## File Structure

### Backend runtime and config

- Create: `src-tauri/src/security/ingress_countermeasure/mod.rs`
  Central runtime service for threat classification, persona selection, deception routing, and recent signal memory.
- Create: `src-tauri/src/security/ingress_countermeasure/config.rs`
  Rust config types for the new ingress countermeasure section.
- Create: `src-tauri/src/security/ingress_countermeasure/classifier.rs`
  Threat-level scoring and reason-code generation.
- Create: `src-tauri/src/security/ingress_countermeasure/persona.rs`
  Stable persona definitions and persona-selection logic.
- Create: `src-tauri/src/security/ingress_countermeasure/deception.rs`
  Hostile-path routing decisions and fake-surface policy mapping.
- Create: `src-tauri/src/security/ingress_countermeasure/tests.rs`
  Focused Rust tests for classifier, fallback, and persona selection.

### Backend integration points

- Modify: `src-tauri/src/config/advanced.rs`
  Add config model fields, defaults, and serialization support.
- Modify: `src-tauri/src/core/coordinator.rs`
  Hydrate and expose the new ingress countermeasure subsystem.
- Modify: `src-tauri/src/feat/coordinator.rs`
  Project runtime status for the new subsystem if needed.
- Modify: `src-tauri/src/enhance/obfuscation.rs`
  Apply persona-driven obfuscation instead of only static level-based overrides.
- Modify: `src-tauri/src/enhance/sniffer.rs`
  Allow classifier-friendly signal capture and dynamic sniff behavior hooks.
- Modify: `src-tauri/src/feat/anti_probe.rs`
  Record anti-probe outcomes into ingress risk signals.
- Modify: `src-tauri/src/core/security_runtime.rs`
  Connect honeypot and decoy activity into ingress countermeasure state.
- Modify: `src-tauri/src/core/stable_egress.rs`
  Add egress-support policy hooks for suspicious or hostile sessions.
- Modify: `src-tauri/src/core/egress_monitor/mod.rs`
  Surface enough signal for drift-minimization policy without changing probe fundamentals.
- Modify: `src-tauri/src/lib.rs`
  Register new Tauri commands if runtime status or config endpoints are added.

### Frontend types and UI

- Modify: `src/services/coordinator.ts`
  Add TypeScript interfaces and coordinator serialization support for ingress countermeasure config.
- Modify: `src/components/advanced/security-config-panel.tsx`
  Add advanced security controls for classifier thresholds, personas, deception mode, and egress support.
- Create: `src/components/security/ingress-countermeasure-panel.tsx`
  Optional focused panel if the existing security panel becomes too crowded.

### Tests

- Modify: `tests/ip-reputation-contract.test.mjs`
  Only if advanced-config contract coverage is reused here.
- Create: `tests/ingress-countermeasure-contract.test.mjs`
  Contract coverage for `AdvancedConfig` ingress countermeasure shape.
- Create: `tests/tauri-security-countermeasure-contract.test.mjs`
  Contract coverage for backend-facing config names and required fields.

## Phase 1: Add Config Model And Contract Surface

### Task 1: Define the Rust config model

**Files:**
- Create: `src-tauri/src/security/ingress_countermeasure/config.rs`
- Modify: `src-tauri/src/security/mod.rs`
- Modify: `src-tauri/src/config/advanced.rs`
- Test: `src-tauri/src/security/ingress_countermeasure/tests.rs`

- [ ] **Step 1: Write the failing Rust test for default config shape**

Add a test module that asserts:

```rust
#[test]
fn ingress_countermeasure_config_defaults_are_safe() {
    let cfg = IngressCountermeasureConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.deception_mode, DeceptionMode::DecoyPreferred);
    assert!(cfg.persona_profiles.len() >= 2);
    assert!(cfg.egress_stability_support.enabled);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
cargo test ingress_countermeasure_config_defaults_are_safe --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because `IngressCountermeasureConfig` does not exist yet.

- [ ] **Step 3: Implement the config model**

Create config types similar to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressCountermeasureConfig {
    pub enabled: bool,
    pub classifier_thresholds: ClassifierThresholds,
    pub persona_profiles: Vec<PersonaProfile>,
    pub deception_mode: DeceptionMode,
    pub response_delay_ranges: ResponseDelayRanges,
    pub fake_surface_policies: Vec<FakeSurfacePolicy>,
    pub egress_stability_support: EgressStabilitySupportConfig,
}
```

Include `Default` impls and keep names consistent with existing `AdvancedConfig` style.

- [ ] **Step 4: Wire config into `AdvancedConfig`**

Add a field like:

```rust
#[serde(default)]
pub ingress_countermeasure: IngressCountermeasureConfig,
```

in `src-tauri/src/config/advanced.rs`, plus `Default` and `recommended()` support.

- [ ] **Step 5: Re-run the Rust test**

Run:

```powershell
cargo test ingress_countermeasure_config_defaults_are_safe --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/security/ingress_countermeasure/config.rs src-tauri/src/security/mod.rs src-tauri/src/config/advanced.rs src-tauri/src/security/ingress_countermeasure/tests.rs
git commit -m "feat: add ingress countermeasure config model"
```

### Task 2: Add frontend config types and contract coverage

**Files:**
- Modify: `src/services/coordinator.ts`
- Create: `tests/ingress-countermeasure-contract.test.mjs`

- [ ] **Step 1: Write the failing contract test**

Create a test that asserts `coordinator.ts` contains:

```javascript
assert.match(service, /export interface IngressCountermeasureConfig/)
assert.match(service, /classifierThresholds/)
assert.match(service, /personaProfiles/)
assert.match(service, /deceptionMode/)
assert.match(service, /egressStabilitySupport/)
```

- [ ] **Step 2: Run the contract test and watch it fail**

Run:

```powershell
node --test tests\ingress-countermeasure-contract.test.mjs
```

Expected: FAIL because the interfaces do not exist yet.

- [ ] **Step 3: Add TypeScript interfaces in `coordinator.ts`**

Define:

```ts
export interface IngressCountermeasureConfig {
  enabled: boolean
  classifierThresholds: ClassifierThresholds
  personaProfiles: PersonaProfile[]
  deceptionMode: DeceptionMode
  responseDelayRanges: ResponseDelayRanges
  fakeSurfacePolicies: FakeSurfacePolicy[]
  egressStabilitySupport: EgressStabilitySupportConfig
}
```

Then add `ingress_countermeasure: IngressCountermeasureConfig` to `AdvancedConfig`.

- [ ] **Step 4: Add normalization and serialization support**

Extend `normalizeAdvancedConfig` and `serializeAdvancedConfig` if the new field needs camel/snake conversion. Keep the style consistent with the recent `ip_reputation` fix.

- [ ] **Step 5: Re-run the contract test**

Run:

```powershell
node --test tests\ingress-countermeasure-contract.test.mjs
```

Expected: PASS.

- [ ] **Step 6: Run typecheck**

Run:

```powershell
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 7: Commit**

```powershell
git add src/services/coordinator.ts tests/ingress-countermeasure-contract.test.mjs
git commit -m "feat: add ingress countermeasure frontend config types"
```

## Phase 2: Build The Threat Classifier Runtime

### Task 3: Introduce threat levels and reason codes

**Files:**
- Create: `src-tauri/src/security/ingress_countermeasure/mod.rs`
- Create: `src-tauri/src/security/ingress_countermeasure/classifier.rs`
- Test: `src-tauri/src/security/ingress_countermeasure/tests.rs`

- [ ] **Step 1: Write the failing Rust classifier test**

Add a test like:

```rust
#[test]
fn classifier_marks_hostile_when_honeypot_and_probe_failures_stack() {
    let classifier = IngressThreatClassifier::new(ClassifierThresholds::default());
    let result = classifier.classify(IngressSignalSnapshot {
        anti_probe_failed: true,
        honeypot_triggered: true,
        suspicious_header_count: 2,
        repeated_burst_count: 3,
    });
    assert_eq!(result.level, ThreatLevel::Hostile);
    assert!(result.reasons.len() >= 2);
}
```

- [ ] **Step 2: Run the Rust test and confirm failure**

Run:

```powershell
cargo test classifier_marks_hostile_when_honeypot_and_probe_failures_stack --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because the classifier types do not exist yet.

- [ ] **Step 3: Implement `ThreatLevel`, `ThreatReason`, `IngressSignalSnapshot`, and `ClassificationResult`**

Keep the model explicit and serializable:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThreatLevel {
    Normal,
    Suspicious,
    Hostile,
}
```

The classifier should accept a plain snapshot and return deterministic level plus reasons.

- [ ] **Step 4: Implement threshold-based classification**

Use explainable scoring:

- honeypot trigger should heavily bias toward `Hostile`
- anti-probe failure plus repeated burst behavior should at least produce `Suspicious`
- thresholds must come from config, not hardcoded magic spread across call sites

- [ ] **Step 5: Re-run the Rust test**

Run:

```powershell
cargo test classifier_marks_hostile_when_honeypot_and_probe_failures_stack --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/security/ingress_countermeasure/mod.rs src-tauri/src/security/ingress_countermeasure/classifier.rs src-tauri/src/security/ingress_countermeasure/tests.rs
git commit -m "feat: add ingress threat classifier"
```

### Task 4: Add recent-signal memory and coordinator wiring

**Files:**
- Modify: `src-tauri/src/core/coordinator.rs`
- Modify: `src-tauri/src/security/ingress_countermeasure/mod.rs`
- Test: `src-tauri/src/security/ingress_countermeasure/tests.rs`

- [ ] **Step 1: Write the failing runtime-memory test**

Add a test that records signals and asserts recent state is retained:

```rust
#[tokio::test]
async fn runtime_records_recent_signals_by_source() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());
    runtime.record_signal("1.2.3.4", ThreatReason::AntiProbeFailed).await;
    let snapshot = runtime.snapshot_for_source("1.2.3.4").await;
    assert!(snapshot.anti_probe_failed);
}
```

- [ ] **Step 2: Run the test to verify failure**

Run:

```powershell
cargo test runtime_records_recent_signals_by_source --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because the runtime memory APIs do not exist yet.

- [ ] **Step 3: Implement source-scoped recent signal memory**

Use a simple in-memory structure keyed by source or flow key. Start with bounded retention and avoid building a heavyweight store.

- [ ] **Step 4: Add coordinator ownership**

Extend `CoreCoordinator` with an `ingress_countermeasure` service and hydrate it inside `apply_sub_configs`.

- [ ] **Step 5: Re-run the Rust test**

Run:

```powershell
cargo test runtime_records_recent_signals_by_source --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/core/coordinator.rs src-tauri/src/security/ingress_countermeasure/mod.rs src-tauri/src/security/ingress_countermeasure/tests.rs
git commit -m "feat: wire ingress countermeasure runtime into coordinator"
```

## Phase 3: Add Persona-Based Obfuscation And Deception Routing

### Task 5: Implement persona profiles

**Files:**
- Create: `src-tauri/src/security/ingress_countermeasure/persona.rs`
- Modify: `src-tauri/src/enhance/obfuscation.rs`
- Test: `src-tauri/src/security/ingress_countermeasure/tests.rs`

- [ ] **Step 1: Write the failing persona-selection test**

Add a test like:

```rust
#[test]
fn suspicious_flow_uses_non_normal_persona() {
    let personas = default_persona_profiles();
    let persona = select_persona(ThreatLevel::Suspicious, &personas).unwrap();
    assert_ne!(persona.name, "normal-browser");
}
```

- [ ] **Step 2: Run the test and verify failure**

Run:

```powershell
cargo test suspicious_flow_uses_non_normal_persona --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because persona support does not exist yet.

- [ ] **Step 3: Implement persona definitions**

Each persona should include:

- `name`
- `tls_fingerprint`
- `ua_family`
- `header_order_profile`
- `timing_jitter_profile`
- `size_shaping_level`
- `eligible_levels`

- [ ] **Step 4: Apply persona influence in `obfuscation.rs`**

Keep the existing config as baseline, but allow classifier-driven persona overrides to supply stronger `global-client-fingerprint` and `global-ua` behavior.

- [ ] **Step 5: Re-run the Rust test**

Run:

```powershell
cargo test suspicious_flow_uses_non_normal_persona --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/security/ingress_countermeasure/persona.rs src-tauri/src/enhance/obfuscation.rs src-tauri/src/security/ingress_countermeasure/tests.rs
git commit -m "feat: add persona-based obfuscation profiles"
```

### Task 6: Implement deception routing for hostile flows

**Files:**
- Create: `src-tauri/src/security/ingress_countermeasure/deception.rs`
- Modify: `src-tauri/src/core/security_runtime.rs`
- Modify: `src-tauri/src/feat/anti_probe.rs`
- Test: `src-tauri/src/security/ingress_countermeasure/tests.rs`

- [ ] **Step 1: Write the failing deception-routing test**

Add a test like:

```rust
#[test]
fn hostile_flow_prefers_decoy_route() {
    let runtime = IngressCountermeasureRuntime::new(IngressCountermeasureConfig::default());
    let plan = runtime.route_for_level(ThreatLevel::Hostile);
    assert_eq!(plan.mode, ResponseMode::Deception);
}
```

- [ ] **Step 2: Run the test to verify failure**

Run:

```powershell
cargo test hostile_flow_prefers_decoy_route --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because response routing does not exist yet.

- [ ] **Step 3: Implement response-mode routing**

Define explicit response modes:

- `Real`
- `Mimic`
- `Deception`
- `LimitedReject`

Hostile flows should choose `Deception` first and only fall back to `LimitedReject` if deception is unavailable.

- [ ] **Step 4: Connect honeypot and anti-probe outcomes into runtime**

When honeypot triggers or anti-probe verification fails, record threat reasons into the runtime service rather than leaving them as isolated checks.

- [ ] **Step 5: Re-run the Rust test**

Run:

```powershell
cargo test hostile_flow_prefers_decoy_route --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/src/security/ingress_countermeasure/deception.rs src-tauri/src/core/security_runtime.rs src-tauri/src/feat/anti_probe.rs src-tauri/src/security/ingress_countermeasure/tests.rs
git commit -m "feat: add hostile deception routing"
```

## Phase 4: Add Egress Support Policies

### Task 7: Add suspicious/hostile drift-minimization policy

**Files:**
- Modify: `src-tauri/src/core/stable_egress.rs`
- Modify: `src-tauri/src/core/egress_monitor/mod.rs`
- Test: `src-tauri/src/security/ingress_countermeasure/tests.rs`

- [ ] **Step 1: Write the failing egress-support test**

Add a focused test that asserts suspicious or hostile flows request reduced drift behavior:

```rust
#[test]
fn hostile_flow_requests_stable_egress_support() {
    let cfg = IngressCountermeasureConfig::default();
    assert!(cfg.egress_stability_support.enabled);
}
```

Then add a runtime-focused assertion that policy output marks drift minimization on hostile flows.

- [ ] **Step 2: Run the test to verify failure**

Run:

```powershell
cargo test hostile_flow_requests_stable_egress_support --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because policy output is not wired yet.

- [ ] **Step 3: Implement policy hooks**

Add a lightweight helper that, for suspicious or hostile flows:

- biases toward existing stable binding
- suppresses unnecessary node reselection
- avoids exporting a fresh identity if a suitable stable one already exists

Do not redesign the rebind strategy itself in this task.

- [ ] **Step 4: Re-run the Rust test**

Run:

```powershell
cargo test hostile_flow_requests_stable_egress_support --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/src/core/stable_egress.rs src-tauri/src/core/egress_monitor/mod.rs src-tauri/src/security/ingress_countermeasure/tests.rs
git commit -m "feat: add ingress-aware egress stability support"
```

## Phase 5: Frontend Controls And Runtime Visibility

### Task 8: Add advanced security UI controls

**Files:**
- Modify: `src/components/advanced/security-config-panel.tsx`
- Optionally Create: `src/components/security/ingress-countermeasure-panel.tsx`
- Modify: `src/services/coordinator.ts`
- Test: `tests/tauri-security-countermeasure-contract.test.mjs`

- [ ] **Step 1: Write the failing UI contract test**

Create a contract test that asserts the security panel or dedicated panel references:

```javascript
assert.match(panel, /ingress countermeasure/i)
assert.match(panel, /classifier/i)
assert.match(panel, /persona/i)
assert.match(panel, /deception/i)
```

- [ ] **Step 2: Run the test and verify failure**

Run:

```powershell
node --test tests\tauri-security-countermeasure-contract.test.mjs
```

Expected: FAIL because the UI controls do not exist yet.

- [ ] **Step 3: Add UI for the new config**

Expose at minimum:

- master enable switch
- threshold controls
- persona selection or profile list
- deception mode selector
- egress support toggle

Reuse existing advanced security styling and interaction patterns.

- [ ] **Step 4: Re-run the contract test**

Run:

```powershell
node --test tests\tauri-security-countermeasure-contract.test.mjs
```

Expected: PASS.

- [ ] **Step 5: Run typecheck**

Run:

```powershell
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 6: Commit**

```powershell
git add src/components/advanced/security-config-panel.tsx src/components/security/ingress-countermeasure-panel.tsx src/services/coordinator.ts tests/tauri-security-countermeasure-contract.test.mjs
git commit -m "feat: add ingress countermeasure security controls"
```

## Phase 6: Final Verification

### Task 9: Run focused backend and contract verification

**Files:**
- Test: `src-tauri/src/security/ingress_countermeasure/tests.rs`
- Test: `tests/ingress-countermeasure-contract.test.mjs`
- Test: `tests/tauri-security-countermeasure-contract.test.mjs`

- [ ] **Step 1: Run focused Rust tests**

Run:

```powershell
cargo test ingress_countermeasure --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 2: Run focused contract tests**

Run:

```powershell
node --test tests\ingress-countermeasure-contract.test.mjs tests\tauri-security-countermeasure-contract.test.mjs
```

Expected: PASS.

- [ ] **Step 3: Run TypeScript verification**

Run:

```powershell
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 4: Run any targeted formatting or lint commands that became necessary**

Run only the smallest required checks for touched files, consistent with repo practice.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri src tests
git commit -m "feat: complete ingress countermeasure and egress support rollout"
```

## Spec Coverage Check

- Threat classifier: covered by Tasks 3 and 4.
- `Normal / Suspicious / Hostile` routing: covered by Task 6.
- Persona-based obfuscation: covered by Task 5.
- Active deception and honeypot integration: covered by Task 6.
- Egress support layer: covered by Task 7.
- Config model and UI controls: covered by Tasks 1, 2, and 8.
- Failure handling and safe fallback: covered by Tasks 3, 5, and 6 through explicit routing and fallback tests.

## Placeholder Scan

Checked for `TODO`, `TBD`, and vague “implement later” language in task steps. None remain in the executable steps.

## Type Consistency Check

- Rust config root name is consistently `IngressCountermeasureConfig`.
- Threat-level enum is consistently `ThreatLevel`.
- Response routing uses `ResponseMode`.
- Frontend config field is consistently `ingress_countermeasure`.
- Egress helper stays as support policy rather than a separate subsystem.
