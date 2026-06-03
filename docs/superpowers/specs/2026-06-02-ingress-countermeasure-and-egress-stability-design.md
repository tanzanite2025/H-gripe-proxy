# Ingress Countermeasure And Egress Stability Design

## Goal

Upgrade the current security and traffic pipeline toward a strong-adversary posture.

The primary goal is stronger ingress defense:

- make inbound probing harder to classify
- move from simple allow/deny behavior to graded response
- introduce active deception and honeypot-backed fake surfaces

The secondary goal is egress stability as a supporting behavior:

- reduce observable egress drift while a suspicious or hostile flow is active
- keep ingress countermeasures from causing unstable upstream identity behavior

This design intentionally favors stronger adversarial resistance over minimum complexity, while still preserving a safe fallback path for normal traffic.

## Context

The current codebase already has useful building blocks, but they are not yet composed into a single ingress-defense system.

- [`src-tauri/src/enhance/obfuscation.rs`](C:/Users/P16V/Desktop/个人开发/clashverge-clean/src-tauri/src/enhance/obfuscation.rs) applies shallow global obfuscation through TLS fingerprint and UA overrides.
- [`src-tauri/src/enhance/sniffer.rs`](C:/Users/P16V/Desktop/个人开发/clashverge-clean/src-tauri/src/enhance/sniffer.rs) injects static sniffing rules into Mihomo config.
- [`src-tauri/src/feat/anti_probe.rs`](C:/Users/P16V/Desktop/个人开发/clashverge-clean/src-tauri/src/feat/anti_probe.rs) exposes anti-probe handshake checks, but only as a narrow verification path.
- [`src-tauri/src/security/honeypot/strategy.rs`](C:/Users/P16V/Desktop/个人开发/clashverge-clean/src-tauri/src/security/honeypot/strategy.rs) provides deception primitives, but they are not yet part of a graded ingress response model.
- [`src-tauri/src/enhance/stable_egress.rs`](C:/Users/P16V/Desktop/个人开发/clashverge-clean/src-tauri/src/enhance/stable_egress.rs) and [`src-tauri/src/core/stable_egress.rs`](C:/Users/P16V/Desktop/个人开发/clashverge-clean/src-tauri/src/core/stable_egress.rs) already provide stable-group injection and runtime backwrite support.
- [`src-tauri/src/core/session_affinity.rs`](C:/Users/P16V/Desktop/个人开发/clashverge-clean/src-tauri/src/core/session_affinity.rs) and the egress monitor stack provide the main tools for keeping egress identity stable enough to support ingress countermeasures.

The opportunity is not to add isolated toggles, but to make these pieces cooperate through one shared ingress threat model.

## Recommended Approach

Use an ingress-first architecture with graded countermeasures:

1. Introduce a lightweight threat classifier that assigns each inbound flow to `Normal`, `Suspicious`, or `Hostile`.
2. Route each class to a different response profile.
3. Keep egress coordination minimal and supportive, rather than turning this work into a full egress redesign.

This is preferred over a symmetric ingress/egress redesign because the user priority is strong ingress countermeasures. It also keeps blast radius smaller than a full risk-state machine spanning anti-probe, honeypot, blackhole breaker, IP reputation, and egress monitor all at once.

## Architecture

### 1. Threat Classifier

Add a new ingress-focused classifier module that evaluates a request or connection attempt using a small set of explainable signals.

Initial signals:

- anti-probe handshake failure or missing expected token
- suspicious sniffer observations
- abnormal request method or header combinations
- repeated access bursts from the same source
- honeypot or decoy interaction
- repeated attempts across fake surfaces or invalid routes

Classifier output:

- `Normal`
- `Suspicious`
- `Hostile`

The classifier should produce both a level and a short reason set. The reason set is important for tuning and debugging. This design explicitly avoids a fully opaque score-only system.

### 2. Response Profiles

#### Normal

Normal traffic stays on the real path with light obfuscation only.

- preserve compatibility
- keep latency overhead low
- avoid unnecessary fingerprint churn

#### Suspicious

Suspicious traffic enters a stronger mimic layer instead of being blocked immediately.

- stronger timing jitter
- stricter browser-persona shaping
- header ordering and Accept-family normalization
- controlled response delay
- limited fake capability leakage

The intention is to waste probe confidence without yet revealing that the request has been classified as hostile.

#### Hostile

Hostile traffic enters an active deception path.

- prefer decoy response over direct reject
- expose fake configuration or fake service surfaces
- use rate limiting, drag, or incomplete fake success
- prevent access to real upstream services

The key behavior is "show a believable but useless surface" rather than just "fail closed."

### 3. Persona-Based Obfuscation

Current obfuscation is mostly static. Replace it with a small set of stable personas rather than pure randomness.

Each persona defines:

- TLS fingerprint preference
- UA family
- header order profile
- header optionality pattern
- timing jitter envelope
- packet or response-size shaping level

Personas must remain stable enough to look believable over time. Full per-request randomness is explicitly not recommended because it tends to look synthetic.

### 4. Active Deception Integration

Integrate existing honeypot and decoy features into the main ingress response flow.

Deception modes:

- fake configuration endpoint behavior
- fake service endpoint behavior
- fake success behavior for selected probe types

These behaviors must be isolated from real upstream resources. A hostile flow should never be able to reach a real sensitive backend through a deception path fallback.

### 5. Egress Support Layer

Egress is not the primary system here, but it must support ingress defense.

Supporting behaviors:

- avoid frequent node re-selection during suspicious or hostile sessions
- prefer existing stable bindings for high-risk domains
- reduce observable IP drift while ingress countermeasures are active

This should be implemented as policy guidance layered on top of stable egress and session affinity, not as a new independent egress engine.

## Data Flow

1. Inbound request or connection arrives.
2. Threat classifier gathers contextual signals.
3. Classifier returns `Normal`, `Suspicious`, or `Hostile`, plus reason codes.
4. Response profile is selected.
5. Profile applies the corresponding obfuscation, deception, or isolation behavior.
6. If the session is `Suspicious` or `Hostile`, egress-support policy asks the stable-egress path to minimize drift.
7. Runtime logging records classification and selected path for operator visibility.

The short-lived classification context should remain scoped to the connection or near-term flow window. Long-lived persistence is not required for the first implementation beyond counters and recent event memory.

## Failure Handling

The system must degrade safely. Preservation of normal usability is more important than preserving the strongest countermeasure in every failure mode.

- If the threat classifier fails, fall back to `Normal` and log the failure.
- If stronger obfuscation logic fails, fall back to light obfuscation.
- If the deception path fails, fall back to limited reject or static fake response, not to the real backend.
- If egress-support logic fails, do not block the request; only stop applying the drift-reduction policy.

Guiding rule:

- degrade toward lower-strength defense
- never degrade from hostile deception into unrestricted real-service exposure

## Configuration

Add a focused ingress countermeasure config section rather than scattering more booleans across existing configs.

Suggested fields:

- `enabled`
- `classifier_thresholds`
- `persona_profiles`
- `deception_mode`
- `response_delay_ranges`
- `fake_surface_policies`
- `egress_stability_support`

Existing anti-probe, obfuscation, and honeypot config should remain valid. The new config should orchestrate them rather than replace every underlying option.

## Testing Strategy

### Unit Tests

Cover:

- classifier outputs for representative signal combinations
- profile routing for all three threat levels
- fallback behavior when classifier or deception logic fails

### Contract Tests

Cover:

- advanced config shape for the new ingress countermeasure settings
- serialization and deserialization of persona and deception settings
- compatibility with existing advanced config loading and saving

### Behavioral Verification

Cover:

- normal browser-like samples remain usable
- weak probes are classified as `Suspicious`
- clear malicious probes are classified as `Hostile`
- hostile flows cannot reach real upstream services
- suspicious or hostile sessions do not cause unnecessary egress churn

## Rollout Order

Recommended implementation order:

1. Introduce the threat classifier and reason codes.
2. Add `Normal / Suspicious / Hostile` routing.
3. Upgrade obfuscation into stable persona-based profiles.
4. Connect hostile routing to honeypot and decoy surfaces.
5. Add egress-support policies that reduce drift for suspicious and hostile flows.
6. Add observability and tuning support.

This order creates a usable minimum loop early without forcing a full-system rewrite.

## Risks

- Over-aggressive classification can misroute legitimate traffic into deception or heavy jitter.
- Excessive randomness can make traffic look more synthetic instead of more human.
- A deception fallback bug could accidentally expose the real upstream if isolation boundaries are weak.
- Egress drift suppression can accidentally make recovery slower if tuned too conservatively.

These risks are why the first version should prefer explainable thresholds, stable personas, and hard isolation between deception and real-service paths.

## Non-Goals

This design does not attempt to:

- redesign the entire egress monitor architecture
- build a general-purpose distributed IDS
- introduce a heavyweight policy engine
- optimize for the lowest possible latency under hostile mode

The purpose is stronger adversarial ingress handling with enough egress stability to keep the disguise coherent.
