use super::*;

pub(crate) async fn persist_default_runtime_execution_guard_state(
    preflight: &DnsDefaultRuntimeOptInExecutorPreflightReport,
) -> (
    DnsDefaultRuntimeExecutionPersistence,
    DnsDefaultRuntimeExecutionSupersededState,
) {
    let superseded_state = default_runtime_execution_superseded_state(preflight);
    if preflight.status != DnsDefaultRuntimeExecutorPreflightStatus::Ready {
        return (
            DnsDefaultRuntimeExecutionPersistence {
                requested: false,
                prepared: false,
                audit_record_path: None,
                rollback_marker_path: None,
                superseded_state_path: None,
                audit_persisted: false,
                rollback_marker_persisted: false,
                superseded_state_persisted: false,
                errors: Vec::new(),
            },
            superseded_state,
        );
    }

    let event_segment = safe_dns_runtime_guard_segment(&preflight.audit_record.event_id);
    let guard_dir = match dirs::app_runtime_dir() {
        Ok(path) => path
            .join("dns-default-runtime")
            .join("execution-guards")
            .join(event_segment),
        Err(error) => {
            return (
                DnsDefaultRuntimeExecutionPersistence {
                    requested: true,
                    prepared: false,
                    audit_record_path: None,
                    rollback_marker_path: None,
                    superseded_state_path: None,
                    audit_persisted: false,
                    rollback_marker_persisted: false,
                    superseded_state_persisted: false,
                    errors: vec![format!("failed to resolve execution guard storage path: {error}")],
                },
                superseded_state,
            );
        }
    };

    let audit_record_path = guard_dir.join("audit.yaml");
    let rollback_marker_path = guard_dir.join("rollback-marker.yaml");
    let superseded_state_path = guard_dir.join("superseded-state.yaml");
    let mut errors = Vec::new();
    let mut audit_persisted = false;
    let mut rollback_marker_persisted = false;
    let mut superseded_state_persisted = false;

    if let Err(error) = fs::create_dir_all(&guard_dir).await {
        errors.push(format!("failed to create execution guard directory: {error}"));
    } else {
        audit_persisted =
            persist_default_runtime_guard_yaml(&audit_record_path, &preflight.audit_record, &mut errors).await;
        rollback_marker_persisted =
            persist_default_runtime_guard_yaml(&rollback_marker_path, &preflight.rollback_marker, &mut errors).await;
        superseded_state_persisted =
            persist_default_runtime_guard_yaml(&superseded_state_path, &superseded_state, &mut errors).await;
    }

    let prepared = audit_persisted && rollback_marker_persisted && superseded_state_persisted;
    (
        DnsDefaultRuntimeExecutionPersistence {
            requested: true,
            prepared,
            audit_record_path: Some(audit_record_path.to_string_lossy().to_string()),
            rollback_marker_path: Some(rollback_marker_path.to_string_lossy().to_string()),
            superseded_state_path: Some(superseded_state_path.to_string_lossy().to_string()),
            audit_persisted,
            rollback_marker_persisted,
            superseded_state_persisted,
            errors,
        },
        superseded_state,
    )
}

pub(crate) async fn persist_default_runtime_guard_yaml<T: Serialize + Sync>(
    path: &std::path::Path,
    value: &T,
    errors: &mut Vec<String>,
) -> bool {
    let yaml = match serde_yaml_ng::to_string(value) {
        Ok(yaml) => yaml,
        Err(error) => {
            errors.push(format!("failed to serialize {}: {error}", path.display()));
            return false;
        }
    };
    match fs::write(path, yaml.as_bytes()).await {
        Ok(()) => true,
        Err(error) => {
            errors.push(format!("failed to persist {}: {error}", path.display()));
            false
        }
    }
}

pub(crate) async fn verify_default_runtime_execution_guard_metadata(
    guard: &DnsDefaultRuntimeOptInExecutionGuardReport,
    errors: &mut Vec<String>,
) -> bool {
    let audit_record = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExecutorAuditRecord>(
        guard.persistence.audit_record_path.as_deref(),
        "audit record",
        errors,
    )
    .await;
    let rollback_marker = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExecutorRollbackMarker>(
        guard.persistence.rollback_marker_path.as_deref(),
        "rollback marker",
        errors,
    )
    .await;
    let superseded_state = read_default_runtime_guard_yaml::<DnsDefaultRuntimeExecutionSupersededState>(
        guard.persistence.superseded_state_path.as_deref(),
        "superseded state",
        errors,
    )
    .await;

    let mut verified = errors.is_empty();
    if let Some(audit_record) = audit_record {
        if audit_record.event_id != guard.preflight.audit_record.event_id {
            errors.push("persisted audit record does not match executor preflight event".into());
            verified = false;
        }
    } else {
        verified = false;
    }
    if let Some(rollback_marker) = rollback_marker {
        if !rollback_marker.prepared || !rollback_marker.restores_runtime {
            errors.push("persisted rollback marker is not prepared".into());
            verified = false;
        }
        if rollback_marker.candidate_runtime != guard.preflight.mutation_diff.candidate_runtime {
            errors.push("persisted rollback marker does not match candidate runtime".into());
            verified = false;
        }
    } else {
        verified = false;
    }
    if let Some(superseded_state) = superseded_state {
        if superseded_state.candidate_runtime != guard.preflight.mutation_diff.candidate_runtime {
            errors.push("persisted superseded state does not match candidate runtime".into());
            verified = false;
        }
    } else {
        verified = false;
    }

    verified && errors.is_empty()
}

pub(crate) async fn read_default_runtime_guard_yaml<T: DeserializeOwned>(
    path: Option<&str>,
    label: &str,
    errors: &mut Vec<String>,
) -> Option<T> {
    let path = match path {
        Some(path) if !path.trim().is_empty() => path,
        _ => {
            errors.push(format!("persisted {label} path is missing"));
            return None;
        }
    };
    let raw_yaml = match fs::read_to_string(path).await {
        Ok(raw_yaml) => raw_yaml,
        Err(error) => {
            errors.push(format!("failed to read persisted {label}: {error}"));
            return None;
        }
    };
    match serde_yaml_ng::from_str::<T>(&raw_yaml) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("failed to parse persisted {label}: {error}"));
            None
        }
    }
}

pub(crate) async fn read_default_runtime_active_state(
    errors: &mut Vec<String>,
) -> Option<DnsDefaultRuntimeActiveState> {
    let path = match default_runtime_active_state_path() {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!(
                "failed to resolve default DNS runtime active state path: {error}"
            ));
            return None;
        }
    };
    let raw_yaml = match fs::read_to_string(&path).await {
        Ok(raw_yaml) => raw_yaml,
        Err(error) => {
            errors.push(format!("failed to read default DNS runtime active state: {error}"));
            return None;
        }
    };
    match serde_yaml_ng::from_str::<DnsDefaultRuntimeActiveState>(&raw_yaml) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("failed to parse default DNS runtime active state: {error}"));
            None
        }
    }
}

pub(crate) async fn read_default_runtime_execution_record_from_active(
    active_state: Option<&DnsDefaultRuntimeActiveState>,
    errors: &mut Vec<String>,
) -> Option<DnsDefaultRuntimeExecutionRecord> {
    let event_id = match active_state {
        Some(active_state) if !active_state.execution_event_id.trim().is_empty() => {
            active_state.execution_event_id.as_str()
        }
        _ => {
            errors.push("default DNS runtime active state has no execution event id".into());
            return None;
        }
    };
    let path = match default_runtime_execution_record_path(event_id) {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!("failed to resolve limited execution audit path: {error}"));
            return None;
        }
    };
    let raw_yaml = match fs::read_to_string(&path).await {
        Ok(raw_yaml) => raw_yaml,
        Err(error) => {
            errors.push(format!("failed to read limited execution audit record: {error}"));
            return None;
        }
    };
    match serde_yaml_ng::from_str::<DnsDefaultRuntimeExecutionRecord>(&raw_yaml) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("failed to parse limited execution audit record: {error}"));
            None
        }
    }
}

pub(crate) async fn persist_default_runtime_execution_record(
    record: &DnsDefaultRuntimeExecutionRecord,
    storage_path: &mut Option<String>,
    errors: &mut Vec<String>,
) -> bool {
    let path = match default_runtime_execution_record_path(&record.event_id) {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!("failed to resolve execution record path: {error}"));
            return false;
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            errors.push(format!("failed to create execution record directory: {error}"));
            return false;
        }
    }
    *storage_path = Some(path.to_string_lossy().to_string());
    persist_default_runtime_guard_yaml(&path, record, errors).await
}

pub(crate) async fn persist_default_runtime_active_state(
    state: &DnsDefaultRuntimeActiveState,
    storage_path: &mut Option<String>,
    errors: &mut Vec<String>,
) -> bool {
    let path = match default_runtime_active_state_path() {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!("failed to resolve active state path: {error}"));
            return false;
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent).await {
            errors.push(format!("failed to create active state directory: {error}"));
            return false;
        }
    }
    *storage_path = Some(path.to_string_lossy().to_string());
    persist_default_runtime_guard_yaml(&path, state, errors).await
}

pub(crate) fn default_runtime_state_dir() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join("dns-default-runtime"))
}

pub(crate) fn default_runtime_active_state_path() -> Result<std::path::PathBuf> {
    Ok(default_runtime_state_dir()?.join("active-runtime.yaml"))
}

pub(crate) fn default_runtime_execution_record_path(event_id: &str) -> Result<std::path::PathBuf> {
    Ok(default_runtime_state_dir()?
        .join("executions")
        .join(safe_dns_runtime_guard_segment(event_id))
        .join("execution.yaml"))
}

pub(crate) async fn runtime_dns_shadow_yaml(yaml: Option<String>, purpose: &str) -> Result<String> {
    match yaml {
        Some(yaml) => Ok(yaml),
        None => {
            let runtime = Config::runtime().await;
            let runtime = runtime.latest_arc();
            let runtime_config = runtime
                .config
                .as_ref()
                .ok_or_else(|| anyhow!("runtime config is not available for DNS {purpose}"))?;
            serde_yaml_ng::to_string(&Value::Mapping(runtime_config.clone()))
                .with_context(|| format!("failed to serialize runtime config for DNS {purpose}"))
        }
    }
}

pub(crate) fn normalize_shadow_domain(domain: Option<String>) -> String {
    domain
        .as_deref()
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
        .unwrap_or(DEFAULT_DNS_HEALTH_CHECK_DOMAIN)
        .to_string()
}

pub(crate) fn default_runtime_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn safe_dns_runtime_guard_segment(value: &str) -> String {
    let segment: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    if segment.is_empty() {
        "dns-default-runtime-execution-guard".into()
    } else {
        segment
    }
}
