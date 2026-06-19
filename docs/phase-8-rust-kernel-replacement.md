# Phase 8: Rust kernel replacement plan

This is the execution plan for replacing the Go/Mihomo data-plane kernel with Rust-owned runtime components. It is intentionally separate from `go-to-rust-migration-roadmap.md` because this phase touches high-risk runtime behavior: TUN, protocol stacks, adapter runtime, and real packet forwarding.

## Decision summary

Phase 8 may start, but not as a direct kernel swap.

The safe path is:

```text
inventory current Mihomo kernel seams
  -> introduce Rust kernel runtime capability boundaries
  -> shadow Rust components without forwarding traffic
  -> opt-in isolated execution
  -> observed verification + rollback drill
  -> expanded opt-in
  -> default cutover only after hold windows pass
```

Default behavior must remain Mihomo-backed until a specific phase explicitly changes it.

## Current Mihomo kernel dependency map

### Process lifecycle

Current owner: `src-tauri/src/core/manager/*`.

- `CoreManager::start_core_by_sidecar` starts the Mihomo sidecar via `tauri_plugin_shell`.
- Runtime config is generated with `Config::generate_file(ConfigType::Run)`.
- The sidecar receives `-d`, `-f`, and external-controller IPC arguments.
- `CoreManager::start_core_by_service` delegates privileged Windows/TUN mode to the service path.
- `CoreManager::restart_core` remains the central lifecycle entry point.

### Controller transport

Current owner: `crates/tauri-plugin-mihomo` and `src-tauri/src/core/handle.rs`.

- `Handle::sync_mihomo_controller_state` keeps the plugin pointed at HTTP or local-socket controller state.
- `tauri-plugin-mihomo::Mihomo` wraps the Mihomo controller API.
- Most runtime reads/writes still flow through the Mihomo controller API, even when app-facing commands are Rust-gated.

### Rust-owned control plane already available

Current owner: `src-tauri/src/core/app_runtime/*`, `src-tauri/src/cmd/runtime.rs`, and related modules.

- App runtime state document.
- Runtime plan.
- Mihomo projection artifact.
- Staged activation.
- Runtime apply boundary decision.
- Apply preflight, audit, observed verification, closeout, and hold.
- DNS readiness, shadow evidence, limited opt-in execution, and reverify history.

These pieces are control-plane inputs to a future Rust data plane; they are not a Rust forwarding engine yet.

## Non-goals for the first PR series

Do not start Phase 8 by doing any of the following:

- Replacing TUN or transparent proxy directly.
- Replacing outbound/inbound protocol implementations directly.
- Removing Mihomo sidecar startup.
- Changing default DNS runtime.
- Changing default packet forwarding.
- Running Rust and Mihomo as competing live forwarding engines.
- Adding a UI toggle that bypasses Rust gate/audit/rollback records.

## Required runtime safety model

Any Phase 8 mutation must carry these fields in design and PR descriptions:

```text
mutatesRuntime=true
kernelArea=<lifecycle|controller|dns|tun|inbound|outbound|adapter|forwarding>
defaultEnabled=false
mihomoFallback=true
rollback=<exact command or state transition>
observedVerification=<metrics/events/logs used to judge success>
holdWindow=<duration or reason for not holding>
```

A mutation is not allowed if it cannot be rolled back to Mihomo without restarting the full app or losing user connectivity state.

## Execution batches

### R0: Kernel seam inventory and capability manifest

Status: complete. `get_runtime_kernel_replacement_readiness` reports the current Mihomo-backed kernel seam without mutating runtime.

Add a Rust-owned read-only report that describes current kernel ownership and replacement readiness. It must not start, stop, or mutate the kernel.

Current command shape:

```rust
RustKernelReplacementReadiness {
    mutatesRuntime: false,
    activeKernel: "mihomo-sidecar" | "mihomo-service" | "not-running",
    controllerTransport: "http" | "local-socket" | "auto",
    rustOwnedControlPlane: Vec<String>,
    mihomoOwnedDataPlane: Vec<String>,
    blockedReplacementAreas: Vec<KernelReplacementBlocker>,
    nextSafeBatch: "rust-shadow-components",
}
```

UI can expose this later in diagnostics, but the first batch can stay Rust command + tests + docs.

### R1: Kernel runtime capability trait

Status: complete. The first implementation is `MihomoKernelRuntime`, an adapter over existing `CoreManager` / `tauri-plugin-mihomo` behavior.

Introduce an internal Rust abstraction without changing behavior:

```rust
trait KernelRuntime {
    fn runtime_id(&self) -> KernelRuntimeId;
    async fn status(&self) -> KernelRuntimeStatus;
    async fn apply_projection_preflight(&self, artifact_id: &str) -> Result<PreflightReport>;
}
```

The only implementation in R1 is `MihomoKernelRuntime`. It delegates to existing `CoreManager` / `tauri-plugin-mihomo` paths and exposes read-only readiness/preflight reports. This creates a seam for later Rust-native implementations while preserving current behavior.

### R2: Rust shadow components, no live forwarding

Status: in progress. `get_runtime_kernel_shadow_components` exposes the read-only component manifest. `get_runtime_kernel_dns_shadow_evidence`, `get_runtime_kernel_rule_shadow_evidence`, and `get_runtime_kernel_adapter_capability_report` now produce DNS, rule, and adapter inventory evidence without live execution.

Current shadow components:

- `dns-shadow-resolver`: compare Rust resolver answers against Mihomo/system output before opt-in execution. First evidence command: `get_runtime_kernel_dns_shadow_evidence`.
- `rule-shadow-classifier`: compare app runtime rule projection with Mihomo rule inventory without routing traffic. First evidence command: `get_runtime_kernel_rule_shadow_evidence`.
- `adapter-capability-shadow`: parse adapter capability before implementing Rust protocol stacks. First evidence command: `get_runtime_kernel_adapter_capability_report`.
- `connection-observer-shadow`: model connection/session shape before Rust forwarding takeover.

All R2 components must keep `mutatesRuntime=false`, `liveExecutionAllowed=false`, and Mihomo as the only live forwarding owner.

Next safe R2 slice: `connection-session-shadow-model`.

### R3: Isolated opt-in execution

Allow a Rust component to handle a bounded, non-default path with explicit opt-in:

- test-only listener or loopback-only DNS path
- no default system proxy/TUN takeover
- automatic rollback to Mihomo on failed verification

### R4: Expanded opt-in with hold window

Expand only after R3 produces evidence:

- repeated verification history
- rollback drill success
- leak checks
- platform matrix coverage
- hold window closeout

### R5: Default cutover candidate

Default cutover is allowed only after all high-risk areas have independent evidence and rollback. It must be a dedicated PR and cannot be combined with cleanup.

## Area-by-area replacement order

Recommended order from lowest to highest risk:

1. Controller lifecycle abstraction (`MihomoKernelRuntime` wrapper only).
2. Read-only telemetry and capability reports.
3. DNS shadow resolver.
4. Rule/adapter shadow classification.
5. Loopback-only or test-only listener.
6. Limited opt-in DNS execution.
7. Limited opt-in inbound/outbound path.
8. TUN / transparent proxy proof of concept.
9. Expanded opt-in forwarding.
10. Default cutover.

## Hard blockers before TUN/protocol replacement

Do not replace TUN, adapter runtime, or forwarding until these exist:

- Rust-owned kernel runtime abstraction with Mihomo fallback.
- Read-only readiness/capability report.
- Runtime apply audit linked to the selected kernel runtime.
- Observed verification for connectivity, DNS leak, proxy leak, and rollback.
- Platform-specific rollback for Windows service mode, macOS, and Linux.
- A user-visible status showing which kernel owns forwarding.

## PR discipline

Each Phase 8 PR must state:

- `mutatesRuntime=true/false`
- kernel area touched
- default enabled or not
- Mihomo fallback path
- verification evidence
- rollback path
- local checks

If a PR touches `tun`, `transparent proxy`, `adapter`, `protocol`, or `forwarding`, it must be small, isolated, and reviewed as a data-plane PR.
