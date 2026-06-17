use super::*;
use crate::{
    config::{Config, IProfiles, PrfItem, PrfOption, profiles::resolve_profile_file_path},
    core::CoreManager,
    utils::{dirs, help},
};
use anyhow::{Result, bail};
use chrono::Local;
use serde::Deserialize;
use serde_yaml_ng::{Mapping, Value};
use sha2::{Digest as _, Sha256};
use smartstring::alias::String;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedAppRuntimeProjectionArtifact {
    pub(super) artifact_id: String,
    pub(super) app_id: String,
    pub(super) storage_path: Option<String>,
    pub(super) activation_mode: AppRuntimeProjectionActivationMode,
    pub(super) mutates_runtime: bool,
    pub(super) checksum: String,
    pub(super) projection: PersistedAppRuntimeMihomoProjection,
    pub(super) validation: PersistedAppRuntimeProjectionValidation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedAppRuntimeMihomoProjection {
    #[serde(default)]
    pub(super) proxy_groups: Vec<MihomoProxyGroupProjection>,
    #[serde(default)]
    pub(super) rules: Vec<MihomoRuleProjection>,
    #[serde(default)]
    pub(super) dns: Option<MihomoDnsProjection>,
    pub(super) yaml_patch: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PersistedAppRuntimeProjectionValidation {
    pub(super) status: AppRuntimeDiagnosticStatus,
}

pub fn project_app_runtime_plan_to_mihomo(
    state: &AppRuntimeStateDocument,
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeMihomoProjection> {
    let plan = explain_app_runtime_plan(state, request);
    let mut facts = plan.facts.clone();
    facts.push("Mihomo projection is an execution artifact; Rust app runtime state remains the source of truth".into());

    if plan.status == AppRuntimePlanStatus::Rejected {
        return Ok(AppRuntimeMihomoProjection {
            status: plan.status,
            reason: plan.reason,
            app_id: plan.app_id,
            session_id: plan.session_id,
            mutates_runtime: false,
            proxy_groups: Vec::new(),
            rules: Vec::new(),
            dns: None,
            yaml_patch: String::new(),
            facts,
            warnings: plan.warnings,
        });
    }

    let mut warnings = plan.warnings.clone();
    let Some(app) = plan.app.as_ref() else {
        warnings.push("runtime plan is ready but missing app facts".into());
        return ready_projection_without_yaml(plan, facts, warnings);
    };
    let routing_intent = plan.routing_intent.unwrap_or(AppRoutingIntent::Direct);
    let mut proxy_groups = Vec::new();
    let target = mihomo_target_for_plan(&plan, routing_intent, &mut proxy_groups, &mut warnings);
    let rules = target
        .as_ref()
        .map(|target| mihomo_rules_for_app(app, target, &mut warnings))
        .unwrap_or_default();
    let dns = plan.dns_profile.as_ref().map(mihomo_dns_projection);
    let yaml_patch = mihomo_yaml_patch(&proxy_groups, &rules)?;
    let reason = if rules.is_empty() {
        format!("app `{}` produced no Mihomo-compatible rule projection", plan.app_id).into()
    } else {
        format!(
            "app `{}` projected {} Mihomo rule(s) and {} proxy group(s)",
            plan.app_id,
            rules.len(),
            proxy_groups.len()
        )
        .into()
    };

    Ok(AppRuntimeMihomoProjection {
        status: plan.status,
        reason,
        app_id: plan.app_id,
        session_id: plan.session_id,
        mutates_runtime: false,
        proxy_groups,
        rules,
        dns,
        yaml_patch,
        facts,
        warnings,
    })
}

pub fn build_app_runtime_projection_artifact(
    state: &AppRuntimeStateDocument,
    request: AppRuntimePlanRequest,
) -> Result<AppRuntimeProjectionArtifact> {
    let diagnostics = diagnose_app_runtime(state, request)?;
    let plan = diagnostics.plan.clone();
    let projection = diagnostics.mihomo_projection.clone();
    let validation = validate_app_runtime_projection_artifact(&plan, &projection, &diagnostics);
    let checksum = app_runtime_projection_checksum(&projection);
    let binding = plan.policy_binding.as_ref();
    let generated_at = Local::now().timestamp_millis();
    let artifact_id = format!("app-runtime-{}-{}", plan.app_id, &checksum[..12]);
    let mut facts = plan.facts.clone();
    facts.push("Projection artifact is generated from Rust AppRuntimeStateDocument and RuntimePlan".into());
    facts.push("Artifact activation is staged; this command does not reload or mutate Mihomo runtime".into());
    let mut warnings = projection.warnings.clone();
    warnings.extend(validation.warnings.iter().cloned());
    warnings.sort();
    warnings.dedup();

    Ok(AppRuntimeProjectionArtifact {
        artifact_id: artifact_id.into(),
        app_id: plan.app_id.clone(),
        session_id: plan.session_id.clone(),
        binding_id: binding.map(|item| item.binding_id.clone()),
        node_pool_id: binding.and_then(|item| item.node_pool_id.clone()),
        dns_profile_id: binding.and_then(|item| item.dns_profile_id.clone()),
        security_profile_id: binding.and_then(|item| item.security_profile_id.clone()),
        generated_at,
        storage_path: None,
        activation_mode: AppRuntimeProjectionActivationMode::Staged,
        mutates_runtime: false,
        checksum: checksum.into(),
        plan,
        projection,
        diagnostics,
        validation,
        facts,
        warnings,
    })
}

pub async fn persist_app_runtime_projection_artifact(artifact: &AppRuntimeProjectionArtifact) -> Result<String> {
    let path = app_runtime_projection_artifact_path(&artifact.artifact_id)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let storage_path: String = path.to_string_lossy().to_string().into();
    let mut persisted_artifact = artifact.clone();
    persisted_artifact.storage_path = Some(storage_path.clone());
    help::save_yaml(&path, &persisted_artifact, None).await?;

    Ok(storage_path)
}

pub async fn preflight_app_runtime_projection_activation(
    request: AppRuntimeProjectionActivationPreflightRequest,
) -> Result<AppRuntimeProjectionActivationPreflightReport> {
    let path = app_runtime_projection_artifact_path(&request.artifact_id)?;
    let storage_path: String = path.to_string_lossy().to_string().into();
    let raw_yaml = match tokio::fs::read_to_string(&path).await {
        Ok(raw_yaml) => raw_yaml,
        Err(err) => {
            return Ok(app_runtime_activation_preflight_missing_artifact_report(
                request,
                storage_path,
                err.to_string().into(),
            ));
        }
    };

    Ok(app_runtime_activation_preflight_report_from_yaml(
        &request,
        storage_path,
        raw_yaml.as_str(),
    ))
}

pub async fn activate_app_runtime_projection_artifact(
    request: AppRuntimeProjectionActivationPreflightRequest,
) -> Result<AppRuntimeStateDocument> {
    let artifact = read_persisted_app_runtime_projection_artifact(&request.artifact_id).await?;
    validate_app_runtime_projection_artifact_activation_request(&artifact, &request)?;
    update_state_document(|state| {
        let previous = state.active_projection.clone();
        state.active_projection = Some(app_runtime_active_projection_record_from_artifact(
            &artifact,
            "state_marker",
            previous.as_ref(),
            now_millis(),
        ));
        Ok(())
    })
    .await
}

pub async fn apply_app_runtime_projection_artifact_to_runtime(
    request: AppRuntimeProjectionRuntimeApplyRequest,
) -> Result<AppRuntimeStateDocument> {
    let artifact = read_persisted_app_runtime_projection_artifact(&request.artifact_id).await?;
    validate_app_runtime_projection_artifact_activation_request(
        &artifact,
        &AppRuntimeProjectionActivationPreflightRequest {
            artifact_id: request.artifact_id.clone(),
            expected_checksum: request.expected_checksum.clone(),
        },
    )?;
    validate_app_runtime_projection_runtime_apply_candidate(&artifact)?;

    let previous = read_app_runtime_state_document().await?.active_projection;
    let active_projection = previous
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("projection artifact must be marked active before runtime apply"))?;
    if active_projection.artifact_id != artifact.artifact_id || active_projection.checksum != artifact.checksum {
        bail!("projection artifact must match the current active projection marker before runtime apply");
    }
    if active_projection.mutates_runtime {
        bail!("active projection already mutates runtime; rollback it before applying another runtime candidate");
    }
    let runtime_apply_decision =
        validate_app_runtime_projection_runtime_apply_boundary_decision(&artifact, &request).await?;

    let candidate = write_app_runtime_projection_runtime_merge_candidate(&artifact).await?;
    let candidate_summary = app_runtime_projection_runtime_apply_candidate_summary(&artifact, &candidate);
    let result = async {
        let profiles = Config::profiles().await;
        profiles.edit_draft(|profiles| attach_app_runtime_runtime_merge_candidate(profiles, &candidate))?;

        let outcome = CoreManager::global()
            .update_config_without_restart_with_force(request.force)
            .await?;
        if !outcome.is_valid() {
            bail!("app runtime projection candidate failed runtime validation: {outcome}");
        }
        let validation_outcome: String = outcome.to_string().into();
        let applied_at = now_millis();
        let audit = app_runtime_projection_runtime_apply_audit_record(
            &artifact,
            &runtime_apply_decision,
            "runtime_profile_merge",
            previous.as_ref(),
            &candidate_summary,
            validation_outcome,
            applied_at,
        );

        update_state_document(|state| {
            ensure_active_projection_unchanged(state.active_projection.as_ref(), previous.as_ref())?;
            mark_runtime_apply_audits_superseded(&mut state.runtime_apply_audits, &audit, applied_at);
            state.active_projection = Some(app_runtime_active_projection_record_from_artifact_with_runtime(
                &artifact,
                "runtime_profile_merge",
                previous.as_ref(),
                applied_at,
                true,
            ));
            state.runtime_apply_audits.push(audit);
            Ok(())
        })
        .await
    }
    .await;

    Config::profiles().await.discard();
    cleanup_app_runtime_projection_runtime_merge_candidate(&candidate).await;

    if result.is_err() {
        let _ = CoreManager::global()
            .update_config_without_restart_with_force(true)
            .await;
    }

    result
}

pub(super) async fn validate_app_runtime_projection_runtime_apply_boundary_decision(
    artifact: &PersistedAppRuntimeProjectionArtifact,
    request: &AppRuntimeProjectionRuntimeApplyRequest,
) -> Result<AppRuntimeRuntimeApplyBoundaryDecisionRecord> {
    let decision_id = request
        .runtime_apply_decision_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime apply requires an accepted runtime-apply boundary decision id"))?;
    let decision = super::handoff::read_persisted_app_runtime_runtime_apply_boundary_decision(decision_id).await?;
    if decision.decision_id != *decision_id {
        bail!("runtime-apply boundary decision id does not match persisted record");
    }
    if decision.app_id != artifact.app_id {
        bail!("runtime-apply boundary decision app id does not match projection artifact");
    }
    if decision.artifact_id != artifact.artifact_id {
        bail!("runtime-apply boundary decision artifact id does not match projection artifact");
    }
    if decision.checksum != artifact.checksum {
        bail!("runtime-apply boundary decision checksum does not match projection artifact");
    }
    if request
        .expected_runtime_apply_decision_checksum
        .as_ref()
        .is_some_and(|expected| expected != &decision.checksum)
    {
        bail!("runtime-apply boundary decision checksum does not match expected checksum");
    }
    if decision.decision != AppRuntimeRuntimeApplyBoundaryDecision::AllowRuntimeCandidate {
        bail!("runtime apply requires an allowRuntimeCandidate boundary decision");
    }
    if !decision.decision_accepted || !decision.runtime_apply_candidate_allowed || !decision.runtime_apply_allowed {
        bail!("runtime apply requires an accepted boundary decision with runtime candidate access");
    }
    if decision.rollback_recommended {
        bail!("runtime apply is blocked because the boundary decision recommends rollback");
    }
    if decision.phase8_allowed
        || decision.promotion_allowed
        || decision.auto_rollout
        || decision.auto_rollback
        || decision.mutates_runtime
        || decision.reload_mihomo
    {
        bail!("runtime-apply boundary decision has unsafe runtime mutation flags");
    }
    Ok(decision)
}

pub async fn list_app_runtime_projection_runtime_apply_audits(
    artifact_id: Option<String>,
) -> Result<Vec<AppRuntimeProjectionRuntimeApplyAuditRecord>> {
    let mut audits: Vec<_> = read_app_runtime_state_document()
        .await?
        .runtime_apply_audits
        .into_iter()
        .filter(|audit| {
            artifact_id
                .as_ref()
                .is_none_or(|artifact_id| &audit.artifact_id == artifact_id)
        })
        .collect();
    audits.sort_by(|left, right| {
        right
            .applied_at
            .cmp(&left.applied_at)
            .then_with(|| right.audit_id.cmp(&left.audit_id))
    });
    Ok(audits)
}

pub async fn verify_app_runtime_projection_runtime_apply(
    request: AppRuntimeProjectionRuntimeVerificationRequest,
) -> Result<AppRuntimeProjectionRuntimeVerificationReport> {
    let state = read_app_runtime_state_document().await?;
    let artifact_id = request.artifact_id.or_else(|| {
        state
            .active_projection
            .as_ref()
            .map(|active| active.artifact_id.clone())
    });
    let report = app_runtime_projection_runtime_verification_report(&state, artifact_id.as_ref()).await?;
    if let Some(audit_id) = report.audit_id.as_ref() {
        let status = report.status;
        let reason = report.reason.clone();
        let observed_at = report.observed_at;
        update_state_document(|state| {
            if let Some(audit) = state
                .runtime_apply_audits
                .iter_mut()
                .find(|audit| &audit.audit_id == audit_id)
            {
                audit.latest_verification_status = Some(status);
                audit.latest_verification_reason = Some(reason);
                audit.latest_verification_at = Some(observed_at);
            }
            Ok(())
        })
        .await?;
    }
    Ok(report)
}

pub async fn rollback_app_runtime_projection_activation() -> Result<AppRuntimeStateDocument> {
    let current = read_app_runtime_state_document()
        .await?
        .active_projection
        .ok_or_else(|| anyhow::anyhow!("no active projection marker is available for rollback"))?;

    let restored = match current.rollback.previous_artifact_id.as_ref() {
        Some(previous_artifact_id) => {
            let artifact = read_persisted_app_runtime_projection_artifact(previous_artifact_id).await?;
            if current.rollback.previous_checksum.as_ref() != Some(&artifact.checksum) {
                bail!(
                    "rollback checksum for `{}` does not match persisted artifact",
                    artifact.artifact_id
                );
            }
            let storage_path = app_runtime_projection_artifact_storage_path(&artifact);
            if current.rollback.previous_storage_path.as_ref() != Some(&storage_path) {
                bail!(
                    "rollback storage path for `{}` does not match persisted artifact",
                    artifact.artifact_id
                );
            }
            validate_app_runtime_projection_artifact_activation_request(
                &artifact,
                &AppRuntimeProjectionActivationPreflightRequest {
                    artifact_id: artifact.artifact_id.clone(),
                    expected_checksum: Some(artifact.checksum.clone()),
                },
            )?;
            Some(app_runtime_active_projection_record_from_artifact(
                &artifact,
                "state_marker_rollback",
                Some(&current),
                now_millis(),
            ))
        }
        None => None,
    };

    if current.mutates_runtime {
        let outcome = CoreManager::global()
            .update_config_without_restart_with_force(true)
            .await?;
        if !outcome.is_valid() {
            bail!("failed to restore runtime while rolling back active projection: {outcome}");
        }
    }

    update_state_document(|state| {
        ensure_active_projection_unchanged(state.active_projection.as_ref(), Some(&current))?;
        if current.mutates_runtime {
            mark_runtime_apply_audits_rolled_back(&mut state.runtime_apply_audits, &current, now_millis());
        }
        state.active_projection = restored;
        Ok(())
    })
    .await
}

pub(super) fn app_runtime_projection_checksum(projection: &AppRuntimeMihomoProjection) -> String {
    let mut hasher = Sha256::new();
    hasher.update(projection.app_id.as_bytes());
    if let Some(session_id) = projection.session_id.as_ref() {
        hasher.update(session_id.as_bytes());
    }
    hasher.update(projection.yaml_patch.as_bytes());
    for rule in &projection.rules {
        hasher.update(rule.rule.as_bytes());
    }
    for group in &projection.proxy_groups {
        hasher.update(group.name.as_bytes());
        hasher.update(group.group_type.as_bytes());
        for proxy in &group.proxies {
            hasher.update(proxy.as_bytes());
        }
    }
    format!("{:x}", hasher.finalize()).into()
}

pub(super) fn app_runtime_projection_artifact_path(artifact_id: &str) -> Result<std::path::PathBuf> {
    let artifact_segment = safe_app_runtime_artifact_segment(artifact_id);
    Ok(dirs::app_runtime_projection_artifacts_dir()?
        .join(artifact_segment.as_str())
        .join("artifact.yaml"))
}

pub(super) fn safe_app_runtime_artifact_segment(value: &str) -> String {
    let segment: std::string::String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let segment = segment.trim_matches('-');

    if segment.is_empty() {
        "artifact".into()
    } else {
        segment.into()
    }
}

pub(super) async fn read_persisted_app_runtime_projection_artifact(
    artifact_id: &str,
) -> Result<PersistedAppRuntimeProjectionArtifact> {
    let path = app_runtime_projection_artifact_path(artifact_id)?;
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        bail!("projection artifact `{artifact_id}` was not found");
    }
    help::read_yaml(&path).await
}

pub(super) fn validate_app_runtime_projection_artifact_activation_request(
    artifact: &PersistedAppRuntimeProjectionArtifact,
    request: &AppRuntimeProjectionActivationPreflightRequest,
) -> Result<()> {
    if artifact.artifact_id != request.artifact_id {
        bail!(
            "persisted artifact id `{}` does not match requested `{}`",
            artifact.artifact_id,
            request.artifact_id
        );
    }
    if let Some(expected_checksum) = request.expected_checksum.as_ref()
        && artifact.checksum != *expected_checksum
    {
        bail!(
            "persisted artifact checksum `{}` does not match expected `{}`",
            artifact.checksum,
            expected_checksum
        );
    }
    if artifact.activation_mode != AppRuntimeProjectionActivationMode::Staged {
        bail!("only staged projection artifacts can be marked active");
    }
    if artifact.mutates_runtime {
        bail!("projection artifact mutates runtime and cannot be marked active by the state marker gate");
    }
    if artifact.validation.status == AppRuntimeDiagnosticStatus::Blocked {
        bail!("projection artifact validation is blocked");
    }
    Ok(())
}

pub(super) fn app_runtime_projection_artifact_storage_path(artifact: &PersistedAppRuntimeProjectionArtifact) -> String {
    artifact.storage_path.clone().unwrap_or_else(|| {
        app_runtime_projection_artifact_path(&artifact.artifact_id)
            .map(|path| path.to_string_lossy().to_string().into())
            .unwrap_or_default()
    })
}

pub(super) fn validate_app_runtime_projection_runtime_apply_candidate(
    artifact: &PersistedAppRuntimeProjectionArtifact,
) -> Result<()> {
    if artifact.projection.proxy_groups.is_empty() && artifact.projection.rules.is_empty() {
        bail!("projection artifact has no proxy groups or rules to apply");
    }
    if artifact.projection.dns.is_some() {
        bail!("runtime apply guard does not mutate DNS runtime yet");
    }
    app_runtime_projection_runtime_merge_mapping(&artifact.projection.yaml_patch)?;
    Ok(())
}

pub(super) fn app_runtime_active_projection_record_from_artifact(
    artifact: &PersistedAppRuntimeProjectionArtifact,
    activation_kind: &str,
    previous: Option<&AppRuntimeActiveProjectionRecord>,
    activated_at: i64,
) -> AppRuntimeActiveProjectionRecord {
    app_runtime_active_projection_record_from_artifact_with_runtime(
        artifact,
        activation_kind,
        previous,
        activated_at,
        false,
    )
}

pub(super) fn app_runtime_active_projection_record_from_artifact_with_runtime(
    artifact: &PersistedAppRuntimeProjectionArtifact,
    activation_kind: &str,
    previous: Option<&AppRuntimeActiveProjectionRecord>,
    activated_at: i64,
    mutates_runtime: bool,
) -> AppRuntimeActiveProjectionRecord {
    AppRuntimeActiveProjectionRecord {
        artifact_id: artifact.artifact_id.clone(),
        app_id: artifact.app_id.clone(),
        checksum: artifact.checksum.clone(),
        storage_path: app_runtime_projection_artifact_storage_path(artifact),
        activated_at,
        activation_kind: activation_kind.into(),
        mutates_runtime,
        rollback: AppRuntimeProjectionRollbackMetadata {
            previous_artifact_id: previous.map(|item| item.artifact_id.clone()),
            previous_checksum: previous.map(|item| item.checksum.clone()),
            previous_storage_path: previous.map(|item| item.storage_path.clone()),
            captured_at: activated_at,
            rollback_strategy: if mutates_runtime {
                "restore_runtime_from_profile_and_previous_marker".into()
            } else {
                "restore_previous_active_projection_marker".into()
            },
        },
    }
}

pub(super) fn ensure_active_projection_unchanged(
    current: Option<&AppRuntimeActiveProjectionRecord>,
    expected: Option<&AppRuntimeActiveProjectionRecord>,
) -> Result<()> {
    match (current, expected) {
        (None, None) => Ok(()),
        (Some(current), Some(expected))
            if current.artifact_id == expected.artifact_id
                && current.checksum == expected.checksum
                && current.activated_at == expected.activated_at =>
        {
            Ok(())
        }
        _ => bail!("active projection marker changed before the operation could complete"),
    }
}

pub(super) struct AppRuntimeRuntimeMergeCandidate {
    uid: String,
    file: String,
    path: PathBuf,
}

pub(super) async fn write_app_runtime_projection_runtime_merge_candidate(
    artifact: &PersistedAppRuntimeProjectionArtifact,
) -> Result<AppRuntimeRuntimeMergeCandidate> {
    let profiles = Config::profiles().await;
    let profiles_arc = profiles.latest_arc();
    let current_profile_uid = profiles_arc
        .get_current()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no current profile is selected for app runtime activation"))?;
    let current_item = profiles_arc
        .get_item(&current_profile_uid)
        .map_err(|err| anyhow::anyhow!("failed to get current profile `{current_profile_uid}`: {err}"))?;
    let current_merge_uid = current_item.current_merge().cloned();
    let current_merge_item = current_merge_uid
        .as_ref()
        .and_then(|uid| profiles_arc.get_item(uid).ok().cloned());
    let current_merge_yaml = match current_merge_item {
        Some(item) => Some(item.read_file().await?),
        None => None,
    };
    drop(profiles_arc);
    drop(profiles);

    let merge_yaml =
        app_runtime_projection_runtime_merge_yaml(current_merge_yaml.as_deref(), &artifact.projection.yaml_patch)?;
    let uid: String = format!(
        "m-app-runtime-{}-{}",
        safe_app_runtime_artifact_segment(&artifact.artifact_id),
        help::get_uid("")
    )
    .into();
    let file: String = format!("{uid}.yaml").into();
    let path = resolve_profile_file_path(file.as_str())?;
    if fs::try_exists(&path).await.unwrap_or(false) {
        bail!("app runtime merge candidate file already exists: {file}");
    }
    fs::write(&path, merge_yaml.as_bytes()).await?;

    Ok(AppRuntimeRuntimeMergeCandidate { uid, file, path })
}

pub(super) fn app_runtime_projection_runtime_merge_yaml(
    current_merge_yaml: Option<&str>,
    projection_yaml_patch: &str,
) -> Result<String> {
    let mut merge = current_merge_yaml
        .filter(|yaml| !yaml.trim().is_empty())
        .map(app_runtime_projection_runtime_merge_mapping)
        .transpose()?
        .unwrap_or_default();
    let patch = app_runtime_projection_runtime_merge_mapping(projection_yaml_patch)?;
    merge_app_runtime_projection_runtime_patch(&mut merge, patch);
    Ok(serde_yaml_ng::to_string(&merge)?.into())
}

pub(super) fn app_runtime_projection_runtime_merge_mapping(yaml: &str) -> Result<Mapping> {
    if yaml.trim().is_empty() {
        return Ok(Mapping::new());
    }
    let value = serde_yaml_ng::from_str::<Value>(yaml)?;
    value
        .as_mapping()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("app runtime projection YAML patch must be a mapping"))
}

pub(super) fn merge_app_runtime_projection_runtime_patch(merge: &mut Mapping, patch: Mapping) {
    for (key, value) in patch {
        if matches!(key.as_str(), Some("rules" | "proxy-groups"))
            && let Some(existing) = merge.get_mut(&key).and_then(Value::as_sequence_mut)
            && let Some(incoming) = value.as_sequence()
        {
            existing.extend(incoming.iter().cloned());
            continue;
        }
        merge.insert(key, value);
    }
}

pub(super) fn attach_app_runtime_runtime_merge_candidate(
    profiles: &mut IProfiles,
    candidate: &AppRuntimeRuntimeMergeCandidate,
) -> Result<()> {
    let current_profile_uid = profiles
        .get_current()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no current profile is selected for app runtime activation"))?;
    let items = profiles.items.get_or_insert_with(Vec::new);
    items.retain(|item| item.uid.as_ref() != Some(&candidate.uid));
    items.push(PrfItem {
        uid: Some(candidate.uid.clone()),
        itype: Some("merge".into()),
        file: Some(candidate.file.clone()),
        updated: Some((now_millis() / 1000) as usize),
        ..PrfItem::default()
    });
    let Some(current_item) = items
        .iter_mut()
        .find(|item| item.uid.as_ref() == Some(&current_profile_uid))
    else {
        bail!("failed to find current profile `{current_profile_uid}` for app runtime activation");
    };
    let option = current_item.option.get_or_insert_with(PrfOption::default);
    option.merge = Some(candidate.uid.clone());
    Ok(())
}

pub(super) async fn cleanup_app_runtime_projection_runtime_merge_candidate(
    candidate: &AppRuntimeRuntimeMergeCandidate,
) {
    let _ = fs::remove_file(&candidate.path).await;
}

pub(super) fn app_runtime_projection_runtime_apply_candidate_summary(
    artifact: &PersistedAppRuntimeProjectionArtifact,
    candidate: &AppRuntimeRuntimeMergeCandidate,
) -> AppRuntimeProjectionRuntimeApplyCandidateSummary {
    AppRuntimeProjectionRuntimeApplyCandidateSummary {
        profile_item_uid: candidate.uid.clone(),
        profile_item_file: candidate.file.clone(),
        proxy_group_count: artifact.projection.proxy_groups.len(),
        rule_count: artifact.projection.rules.len(),
        dns_profile_projected: artifact.projection.dns.is_some(),
    }
}

pub(super) fn app_runtime_projection_runtime_apply_audit_record(
    artifact: &PersistedAppRuntimeProjectionArtifact,
    runtime_apply_decision: &AppRuntimeRuntimeApplyBoundaryDecisionRecord,
    activation_kind: &str,
    previous: Option<&AppRuntimeActiveProjectionRecord>,
    candidate_summary: &AppRuntimeProjectionRuntimeApplyCandidateSummary,
    validation_outcome: String,
    applied_at: i64,
) -> AppRuntimeProjectionRuntimeApplyAuditRecord {
    AppRuntimeProjectionRuntimeApplyAuditRecord {
        audit_id: format!(
            "runtime-apply-{}-{}",
            safe_app_runtime_artifact_segment(&artifact.artifact_id),
            applied_at
        )
        .into(),
        artifact_id: artifact.artifact_id.clone(),
        app_id: artifact.app_id.clone(),
        checksum: artifact.checksum.clone(),
        runtime_apply_decision_id: Some(runtime_apply_decision.decision_id.clone()),
        runtime_apply_decision_boundary_manifest_id: Some(runtime_apply_decision.boundary_manifest_id.clone()),
        activation_kind: activation_kind.into(),
        applied_at,
        validation_outcome,
        candidate_summary: candidate_summary.clone(),
        previous_marker: previous.map(app_runtime_projection_runtime_apply_marker_snapshot),
        rollback_strategy: "restore_runtime_from_profile_and_previous_marker".into(),
        status: AppRuntimeProjectionRuntimeApplyAuditStatus::Active,
        status_updated_at: applied_at,
        latest_verification_status: None,
        latest_verification_reason: None,
        latest_verification_at: None,
    }
}

pub(super) fn app_runtime_projection_runtime_apply_marker_snapshot(
    marker: &AppRuntimeActiveProjectionRecord,
) -> AppRuntimeProjectionRuntimeApplyMarkerSnapshot {
    AppRuntimeProjectionRuntimeApplyMarkerSnapshot {
        artifact_id: marker.artifact_id.clone(),
        checksum: marker.checksum.clone(),
        storage_path: marker.storage_path.clone(),
        activation_kind: marker.activation_kind.clone(),
        mutates_runtime: marker.mutates_runtime,
        activated_at: marker.activated_at,
    }
}

pub(super) fn mark_runtime_apply_audits_superseded(
    audits: &mut [AppRuntimeProjectionRuntimeApplyAuditRecord],
    next_audit: &AppRuntimeProjectionRuntimeApplyAuditRecord,
    updated_at: i64,
) {
    for audit in audits
        .iter_mut()
        .filter(|audit| audit.status == AppRuntimeProjectionRuntimeApplyAuditStatus::Active)
    {
        if audit.audit_id != next_audit.audit_id {
            audit.status = AppRuntimeProjectionRuntimeApplyAuditStatus::Superseded;
            audit.status_updated_at = updated_at;
        }
    }
}

pub(super) fn mark_runtime_apply_audits_rolled_back(
    audits: &mut [AppRuntimeProjectionRuntimeApplyAuditRecord],
    marker: &AppRuntimeActiveProjectionRecord,
    updated_at: i64,
) {
    for audit in audits.iter_mut().filter(|audit| {
        audit.status == AppRuntimeProjectionRuntimeApplyAuditStatus::Active
            && audit.artifact_id == marker.artifact_id
            && audit.checksum == marker.checksum
    }) {
        audit.status = AppRuntimeProjectionRuntimeApplyAuditStatus::RolledBack;
        audit.status_updated_at = updated_at;
    }
}

pub(super) async fn app_runtime_projection_runtime_verification_report(
    state: &AppRuntimeStateDocument,
    artifact_id: Option<&String>,
) -> Result<AppRuntimeProjectionRuntimeVerificationReport> {
    let observed_at = now_millis();
    let mut checks = Vec::new();
    let mut facts = vec!["runtime apply verification is read-only and does not reload Mihomo".into()];
    let mut warnings = Vec::new();
    let active = state.active_projection.as_ref();
    let audit = latest_runtime_apply_audit(state, artifact_id);
    let artifact = match artifact_id {
        Some(artifact_id) => Some(read_persisted_app_runtime_projection_artifact(artifact_id).await?),
        None => None,
    };

    checks.push(runtime_verification_active_marker_check(active, artifact_id));
    checks.push(runtime_verification_audit_check(audit));

    let runtime_config = Config::runtime().await.latest_arc().config.clone();
    checks.push(runtime_verification_runtime_config_check(runtime_config.as_ref()));

    if let Some(artifact) = artifact.as_ref() {
        facts.push(
            format!(
                "artifact projects {} proxy group(s) and {} rule(s)",
                artifact.projection.proxy_groups.len(),
                artifact.projection.rules.len()
            )
            .into(),
        );
        if let Some(runtime_config) = runtime_config.as_ref() {
            checks.push(runtime_verification_proxy_groups_check(
                runtime_config,
                &artifact.projection.proxy_groups,
            ));
            checks.push(runtime_verification_rules_check(
                runtime_config,
                &artifact.projection.rules,
            ));
        }
    }

    let summary = diagnostics_summary(&checks);
    let status = diagnostics_status(&summary);
    let reason = runtime_verification_reason(status, &summary);
    warnings.extend(runtime_verification_warnings(&checks));

    Ok(AppRuntimeProjectionRuntimeVerificationReport {
        status,
        reason,
        artifact_id: artifact_id.cloned(),
        checksum: active.map(|active| active.checksum.clone()),
        audit_id: audit.map(|audit| audit.audit_id.clone()),
        observed_at,
        checks,
        summary,
        facts,
        warnings,
    })
}

pub(super) fn latest_runtime_apply_audit<'a>(
    state: &'a AppRuntimeStateDocument,
    artifact_id: Option<&String>,
) -> Option<&'a AppRuntimeProjectionRuntimeApplyAuditRecord> {
    state
        .runtime_apply_audits
        .iter()
        .filter(|audit| artifact_id.is_none_or(|artifact_id| &audit.artifact_id == artifact_id))
        .max_by(|left, right| {
            left.applied_at
                .cmp(&right.applied_at)
                .then_with(|| left.audit_id.cmp(&right.audit_id))
        })
}

pub(super) fn runtime_verification_active_marker_check(
    active: Option<&AppRuntimeActiveProjectionRecord>,
    artifact_id: Option<&String>,
) -> AppRuntimeDiagnosticCheck {
    match active {
        Some(active)
            if artifact_id.is_none_or(|artifact_id| &active.artifact_id == artifact_id) && active.mutates_runtime =>
        {
            diagnostic_check(
                "runtime_apply_active_marker",
                AppRuntimeDiagnosticCategory::RuntimeBoundary,
                AppRuntimeDiagnosticCheckStatus::Passed,
                "active projection marker records a runtime mutation".into(),
                vec![format!("activationKind={}", active.activation_kind).into()],
            )
        }
        Some(active) if artifact_id.is_some_and(|artifact_id| &active.artifact_id != artifact_id) => diagnostic_check(
            "runtime_apply_active_marker",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "active projection marker does not match requested artifact".into(),
            vec![format!("activeArtifactId={}", active.artifact_id).into()],
        ),
        Some(_) => diagnostic_check(
            "runtime_apply_active_marker",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Warning,
            "active projection marker has not mutated runtime".into(),
            Vec::new(),
        ),
        None => diagnostic_check(
            "runtime_apply_active_marker",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "no active projection marker is available for runtime verification".into(),
            Vec::new(),
        ),
    }
}

pub(super) fn runtime_verification_audit_check(
    audit: Option<&AppRuntimeProjectionRuntimeApplyAuditRecord>,
) -> AppRuntimeDiagnosticCheck {
    match audit {
        Some(audit) if audit.status == AppRuntimeProjectionRuntimeApplyAuditStatus::Active => diagnostic_check(
            "runtime_apply_audit_active",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Passed,
            "latest runtime apply audit is active".into(),
            vec![format!("auditId={}", audit.audit_id).into()],
        ),
        Some(audit) => diagnostic_check(
            "runtime_apply_audit_active",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Warning,
            "latest runtime apply audit is not active".into(),
            vec![format!("status={:?}", audit.status).into()],
        ),
        None => diagnostic_check(
            "runtime_apply_audit_active",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "no runtime apply audit exists for this artifact".into(),
            Vec::new(),
        ),
    }
}

pub(super) fn runtime_verification_runtime_config_check(runtime_config: Option<&Mapping>) -> AppRuntimeDiagnosticCheck {
    if runtime_config.is_some() {
        diagnostic_check(
            "runtime_apply_runtime_config",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Passed,
            "runtime config is available for read-only verification".into(),
            Vec::new(),
        )
    } else {
        diagnostic_check(
            "runtime_apply_runtime_config",
            AppRuntimeDiagnosticCategory::RuntimeBoundary,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "runtime config is not available for read-only verification".into(),
            Vec::new(),
        )
    }
}

pub(super) fn runtime_verification_proxy_groups_check(
    runtime_config: &Mapping,
    proxy_groups: &[MihomoProxyGroupProjection],
) -> AppRuntimeDiagnosticCheck {
    if proxy_groups.is_empty() {
        return diagnostic_check(
            "runtime_apply_proxy_groups_observed",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Skipped,
            "projection contains no proxy groups to verify".into(),
            Vec::new(),
        );
    }
    let observed = runtime_config
        .get("proxy-groups")
        .and_then(Value::as_sequence)
        .map(|groups| {
            proxy_groups
                .iter()
                .filter(|group| runtime_proxy_group_observed(groups, &group.name))
                .count()
        })
        .unwrap_or(0);
    if observed == proxy_groups.len() {
        diagnostic_check(
            "runtime_apply_proxy_groups_observed",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Passed,
            "all projected proxy groups are observable in runtime config".into(),
            vec![format!("observed={observed}/{}", proxy_groups.len()).into()],
        )
    } else {
        diagnostic_check(
            "runtime_apply_proxy_groups_observed",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "some projected proxy groups are missing from runtime config".into(),
            vec![format!("observed={observed}/{}", proxy_groups.len()).into()],
        )
    }
}

pub(super) fn runtime_verification_rules_check(
    runtime_config: &Mapping,
    rules: &[MihomoRuleProjection],
) -> AppRuntimeDiagnosticCheck {
    if rules.is_empty() {
        return diagnostic_check(
            "runtime_apply_rules_observed",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Skipped,
            "projection contains no rules to verify".into(),
            Vec::new(),
        );
    }
    let observed = runtime_config
        .get("rules")
        .and_then(Value::as_sequence)
        .map(|runtime_rules| {
            rules
                .iter()
                .filter(|rule| runtime_rule_observed(runtime_rules, &rule.rule))
                .count()
        })
        .unwrap_or(0);
    if observed == rules.len() {
        diagnostic_check(
            "runtime_apply_rules_observed",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Passed,
            "all projected rules are observable in runtime config".into(),
            vec![format!("observed={observed}/{}", rules.len()).into()],
        )
    } else {
        diagnostic_check(
            "runtime_apply_rules_observed",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "some projected rules are missing from runtime config".into(),
            vec![format!("observed={observed}/{}", rules.len()).into()],
        )
    }
}

pub(super) fn runtime_proxy_group_observed(groups: &[Value], name: &str) -> bool {
    groups.iter().any(|group| {
        group
            .as_mapping()
            .and_then(|mapping| mapping.get("name"))
            .and_then(Value::as_str)
            == Some(name)
    })
}

pub(super) fn runtime_rule_observed(rules: &[Value], expected: &str) -> bool {
    rules.iter().any(|rule| rule.as_str() == Some(expected))
}

pub(super) fn runtime_verification_reason(
    status: AppRuntimeDiagnosticStatus,
    summary: &AppRuntimeDiagnosticsSummary,
) -> String {
    match status {
        AppRuntimeDiagnosticStatus::Healthy => "runtime apply evidence is observable".into(),
        AppRuntimeDiagnosticStatus::Degraded => format!(
            "runtime apply evidence is partially observable: {} warning(s), {} passed",
            summary.warnings, summary.passed
        )
        .into(),
        AppRuntimeDiagnosticStatus::Blocked => format!(
            "runtime apply evidence is incomplete: {} failed check(s)",
            summary.failed
        )
        .into(),
    }
}

pub(super) fn runtime_verification_warnings(checks: &[AppRuntimeDiagnosticCheck]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| {
            matches!(
                check.status,
                AppRuntimeDiagnosticCheckStatus::Warning | AppRuntimeDiagnosticCheckStatus::Failed
            )
        })
        .map(|check| check.message.clone())
        .collect()
}

pub(super) fn app_runtime_activation_preflight_missing_artifact_report(
    request: AppRuntimeProjectionActivationPreflightRequest,
    storage_path: String,
    error: String,
) -> AppRuntimeProjectionActivationPreflightReport {
    let checks = vec![diagnostic_check(
        "activation_artifact_exists",
        AppRuntimeDiagnosticCategory::Projection,
        AppRuntimeDiagnosticCheckStatus::Failed,
        format!("projection artifact `{}` was not found", request.artifact_id).into(),
        vec![storage_path.clone(), error],
    )];
    app_runtime_activation_preflight_report(request.artifact_id, None, None, Some(storage_path), None, None, checks)
}

pub(super) fn app_runtime_activation_preflight_report_from_yaml(
    request: &AppRuntimeProjectionActivationPreflightRequest,
    storage_path: String,
    raw_yaml: &str,
) -> AppRuntimeProjectionActivationPreflightReport {
    let mut checks = vec![diagnostic_check(
        "activation_artifact_exists",
        AppRuntimeDiagnosticCategory::Projection,
        AppRuntimeDiagnosticCheckStatus::Passed,
        format!("projection artifact `{}` is persisted", request.artifact_id).into(),
        vec![storage_path.clone()],
    )];
    let parsed = match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw_yaml) {
        Ok(value) => value,
        Err(err) => {
            checks.push(diagnostic_check(
                "activation_artifact_parse",
                AppRuntimeDiagnosticCategory::Projection,
                AppRuntimeDiagnosticCheckStatus::Failed,
                "projection artifact YAML could not be parsed".into(),
                vec![err.to_string().into()],
            ));
            return app_runtime_activation_preflight_report(
                request.artifact_id.clone(),
                None,
                None,
                Some(storage_path),
                None,
                None,
                checks,
            );
        }
    };

    checks.push(diagnostic_check(
        "activation_artifact_parse",
        AppRuntimeDiagnosticCategory::Projection,
        AppRuntimeDiagnosticCheckStatus::Passed,
        "projection artifact YAML is parseable".into(),
        Vec::new(),
    ));

    let Some(mapping) = parsed.as_mapping() else {
        checks.push(diagnostic_check(
            "activation_artifact_shape",
            AppRuntimeDiagnosticCategory::Projection,
            AppRuntimeDiagnosticCheckStatus::Failed,
            "projection artifact YAML is not an object".into(),
            Vec::new(),
        ));
        return app_runtime_activation_preflight_report(
            request.artifact_id.clone(),
            None,
            None,
            Some(storage_path),
            None,
            None,
            checks,
        );
    };

    let artifact_id = yaml_string_field(mapping, "artifactId").unwrap_or_else(|| request.artifact_id.clone());
    let app_id = yaml_string_field(mapping, "appId");
    let checksum = yaml_string_field(mapping, "checksum");
    let activation_mode = yaml_string_field(mapping, "activationMode");
    let mutates_runtime = yaml_bool_field(mapping, "mutatesRuntime");
    let validation_status =
        yaml_mapping_field(mapping, "validation").and_then(|validation| yaml_string_field(validation, "status"));

    checks.push(diagnostic_check(
        "activation_artifact_id_match",
        AppRuntimeDiagnosticCategory::Projection,
        if artifact_id == request.artifact_id {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else {
            AppRuntimeDiagnosticCheckStatus::Failed
        },
        if artifact_id == request.artifact_id {
            "persisted artifact id matches the requested artifact".into()
        } else {
            "persisted artifact id does not match the requested artifact".into()
        },
        vec![
            format!("requested={}", request.artifact_id).into(),
            format!("persisted={artifact_id}").into(),
        ],
    ));

    checks.push(diagnostic_check(
        "activation_checksum_match",
        AppRuntimeDiagnosticCategory::Projection,
        checksum_preflight_status(checksum.as_ref(), request.expected_checksum.as_ref()),
        checksum_preflight_message(checksum.as_ref(), request.expected_checksum.as_ref()),
        checksum_preflight_details(checksum.as_ref(), request.expected_checksum.as_ref()),
    ));

    checks.push(diagnostic_check(
        "activation_validation_gate",
        AppRuntimeDiagnosticCategory::Projection,
        validation_status_preflight_status(validation_status.as_deref()),
        validation_status_preflight_message(validation_status.as_deref()),
        validation_status
            .as_ref()
            .map(|status| vec![format!("validation.status={status}").into()])
            .unwrap_or_default(),
    ));

    let runtime_boundary_passed = activation_mode.as_deref() == Some("staged") && mutates_runtime == Some(false);
    checks.push(diagnostic_check(
        "activation_runtime_boundary",
        AppRuntimeDiagnosticCategory::RuntimeBoundary,
        if runtime_boundary_passed {
            AppRuntimeDiagnosticCheckStatus::Passed
        } else {
            AppRuntimeDiagnosticCheckStatus::Failed
        },
        "activation preflight requires staged artifact and mutatesRuntime=false".into(),
        vec![
            format!("activationMode={}", activation_mode.as_deref().unwrap_or("missing")).into(),
            format!(
                "mutatesRuntime={}",
                mutates_runtime
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "missing".into())
            )
            .into(),
        ],
    ));

    checks.push(diagnostic_check(
        "activation_executor_guard",
        AppRuntimeDiagnosticCategory::RuntimeBoundary,
        AppRuntimeDiagnosticCheckStatus::Failed,
        "controlled activation executor is not enabled in this preflight batch".into(),
        vec![
            "No Mihomo reload/restart was performed".into(),
            "Next activation PR must add runtime apply and rollback metadata before this guard can pass".into(),
        ],
    ));

    app_runtime_activation_preflight_report(
        artifact_id,
        app_id,
        checksum,
        Some(storage_path),
        activation_mode.and_then(|mode| {
            if mode == "staged" {
                Some(AppRuntimeProjectionActivationMode::Staged)
            } else {
                None
            }
        }),
        mutates_runtime,
        checks,
    )
}

pub(super) fn app_runtime_activation_preflight_report(
    artifact_id: String,
    app_id: Option<String>,
    checksum: Option<String>,
    storage_path: Option<String>,
    activation_mode: Option<AppRuntimeProjectionActivationMode>,
    mutates_runtime: Option<bool>,
    checks: Vec<AppRuntimeDiagnosticCheck>,
) -> AppRuntimeProjectionActivationPreflightReport {
    let summary = diagnostics_summary(&checks);
    let status = diagnostics_status(&summary);
    let reason = diagnostics_reason(status, &summary);
    let warnings = projection_artifact_validation_warnings(&checks);
    let facts = vec![
        "Activation preflight reads a persisted Rust projection artifact".into(),
        "This command never reloads, restarts, or mutates Mihomo runtime".into(),
    ];

    AppRuntimeProjectionActivationPreflightReport {
        status,
        reason,
        artifact_id,
        app_id,
        checksum,
        storage_path,
        activation_mode,
        mutates_runtime,
        checks,
        summary,
        facts,
        warnings,
    }
}

pub(super) fn yaml_value_for_key<'a>(
    mapping: &'a serde_yaml_ng::Mapping,
    key: &str,
) -> Option<&'a serde_yaml_ng::Value> {
    mapping.get(&serde_yaml_ng::Value::String(std::string::String::from(key)))
}

pub(super) fn yaml_string_field(mapping: &serde_yaml_ng::Mapping, key: &str) -> Option<String> {
    yaml_value_for_key(mapping, key)
        .and_then(serde_yaml_ng::Value::as_str)
        .map(Into::into)
}

pub(super) fn yaml_bool_field(mapping: &serde_yaml_ng::Mapping, key: &str) -> Option<bool> {
    yaml_value_for_key(mapping, key).and_then(serde_yaml_ng::Value::as_bool)
}

pub(super) fn yaml_mapping_field<'a>(
    mapping: &'a serde_yaml_ng::Mapping,
    key: &str,
) -> Option<&'a serde_yaml_ng::Mapping> {
    yaml_value_for_key(mapping, key).and_then(serde_yaml_ng::Value::as_mapping)
}

pub(super) fn checksum_preflight_status(
    checksum: Option<&String>,
    expected_checksum: Option<&String>,
) -> AppRuntimeDiagnosticCheckStatus {
    match (checksum, expected_checksum) {
        (Some(checksum), Some(expected_checksum)) if checksum == expected_checksum => {
            AppRuntimeDiagnosticCheckStatus::Passed
        }
        (Some(_), Some(_)) | (None, Some(_)) => AppRuntimeDiagnosticCheckStatus::Failed,
        (_, None) => AppRuntimeDiagnosticCheckStatus::Warning,
    }
}

pub(super) fn checksum_preflight_message(checksum: Option<&String>, expected_checksum: Option<&String>) -> String {
    match (checksum, expected_checksum) {
        (Some(checksum), Some(expected_checksum)) if checksum == expected_checksum => {
            "persisted artifact checksum matches the selected artifact".into()
        }
        (Some(_), Some(_)) => "persisted artifact checksum differs from the selected artifact".into(),
        (None, Some(_)) => "persisted artifact is missing checksum".into(),
        (_, None) => "selected artifact checksum was not provided for comparison".into(),
    }
}

pub(super) fn checksum_preflight_details(checksum: Option<&String>, expected_checksum: Option<&String>) -> Vec<String> {
    vec![
        format!("persisted={}", checksum.map(String::as_str).unwrap_or("missing")).into(),
        format!(
            "expected={}",
            expected_checksum.map(String::as_str).unwrap_or("missing")
        )
        .into(),
    ]
}

pub(super) fn validation_status_preflight_status(status: Option<&str>) -> AppRuntimeDiagnosticCheckStatus {
    match status {
        Some("healthy") => AppRuntimeDiagnosticCheckStatus::Passed,
        Some("degraded") => AppRuntimeDiagnosticCheckStatus::Warning,
        Some("blocked") | None => AppRuntimeDiagnosticCheckStatus::Failed,
        Some(_) => AppRuntimeDiagnosticCheckStatus::Failed,
    }
}

pub(super) fn validation_status_preflight_message(status: Option<&str>) -> String {
    match status {
        Some("healthy") => "artifact validation gate is healthy".into(),
        Some("degraded") => "artifact validation gate is degraded".into(),
        Some("blocked") => "artifact validation gate is blocked".into(),
        Some(status) => format!("artifact validation gate has unknown status `{status}`").into(),
        None => "artifact validation gate status is missing".into(),
    }
}

pub(super) fn yaml_patch_validation_status(
    plan: &AppRuntimePlan,
    projection: &AppRuntimeMihomoProjection,
) -> AppRuntimeDiagnosticCheckStatus {
    if projection.yaml_patch.trim().is_empty() {
        return if plan.status == AppRuntimePlanStatus::Ready {
            AppRuntimeDiagnosticCheckStatus::Warning
        } else {
            AppRuntimeDiagnosticCheckStatus::Skipped
        };
    }

    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&projection.yaml_patch) {
        Ok(_) => AppRuntimeDiagnosticCheckStatus::Passed,
        Err(_) => AppRuntimeDiagnosticCheckStatus::Failed,
    }
}

pub(super) fn yaml_patch_validation_message(plan: &AppRuntimePlan, projection: &AppRuntimeMihomoProjection) -> String {
    if projection.yaml_patch.trim().is_empty() {
        return if plan.status == AppRuntimePlanStatus::Ready {
            "ready plan produced an empty YAML patch".into()
        } else {
            "YAML patch parse skipped for rejected plan".into()
        };
    }

    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&projection.yaml_patch) {
        Ok(_) => "projection YAML patch parses successfully".into(),
        Err(error) => format!("projection YAML patch failed to parse: {error}").into(),
    }
}

pub(super) fn yaml_patch_validation_details(projection: &AppRuntimeMihomoProjection) -> Vec<String> {
    if projection.yaml_patch.trim().is_empty() {
        return Vec::new();
    }

    vec![format!("checksum={}", app_runtime_projection_checksum(projection)).into()]
}

pub(super) fn projection_artifact_validation_warnings(checks: &[AppRuntimeDiagnosticCheck]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| {
            check.status == AppRuntimeDiagnosticCheckStatus::Warning
                || check.status == AppRuntimeDiagnosticCheckStatus::Failed
        })
        .map(|check| check.message.clone())
        .collect()
}

pub(super) fn diagnostic_severity(status: AppRuntimeDiagnosticCheckStatus) -> AppRuntimeDiagnosticSeverity {
    match status {
        AppRuntimeDiagnosticCheckStatus::Passed | AppRuntimeDiagnosticCheckStatus::Skipped => {
            AppRuntimeDiagnosticSeverity::Info
        }
        AppRuntimeDiagnosticCheckStatus::Warning => AppRuntimeDiagnosticSeverity::Warning,
        AppRuntimeDiagnosticCheckStatus::Failed => AppRuntimeDiagnosticSeverity::Error,
    }
}

pub(super) fn ready_projection_without_yaml(
    plan: AppRuntimePlan,
    facts: Vec<String>,
    warnings: Vec<String>,
) -> Result<AppRuntimeMihomoProjection> {
    Ok(AppRuntimeMihomoProjection {
        status: plan.status,
        reason: "runtime plan could not be projected to Mihomo YAML".into(),
        app_id: plan.app_id,
        session_id: plan.session_id,
        mutates_runtime: false,
        proxy_groups: Vec::new(),
        rules: Vec::new(),
        dns: None,
        yaml_patch: String::new(),
        facts,
        warnings,
    })
}

pub(super) fn mihomo_target_for_plan(
    plan: &AppRuntimePlan,
    routing_intent: AppRoutingIntent,
    proxy_groups: &mut Vec<MihomoProxyGroupProjection>,
    warnings: &mut Vec<String>,
) -> Option<String> {
    match routing_intent {
        AppRoutingIntent::Direct => Some("DIRECT".into()),
        AppRoutingIntent::Reject => Some("REJECT".into()),
        AppRoutingIntent::Proxy | AppRoutingIntent::Auto | AppRoutingIntent::Fallback => {
            let Some(node_pool) = plan.node_pool.as_ref() else {
                warnings.push("Mihomo projection requires a node pool for proxy-like routing intents".into());
                return None;
            };
            let proxies = sorted_candidate_node_names(&node_pool.candidates);
            if proxies.is_empty() {
                warnings.push(format!("node pool `{}` has no Mihomo proxy candidates", node_pool.pool_id).into());
                return None;
            }
            let group = mihomo_proxy_group(&plan.app_id, routing_intent, proxies);
            let target = group.name.clone();
            proxy_groups.push(group);
            Some(target)
        }
    }
}

pub(super) fn mihomo_proxy_group(
    app_id: &str,
    routing_intent: AppRoutingIntent,
    proxies: Vec<String>,
) -> MihomoProxyGroupProjection {
    let (group_type, url, interval) = match routing_intent {
        AppRoutingIntent::Auto => (
            "url-test",
            Some("https://www.gstatic.com/generate_204".into()),
            Some(300),
        ),
        AppRoutingIntent::Fallback => (
            "fallback",
            Some("https://www.gstatic.com/generate_204".into()),
            Some(300),
        ),
        _ => ("select", None, None),
    };

    MihomoProxyGroupProjection {
        name: format!("app-{app_id}").into(),
        group_type: group_type.into(),
        proxies,
        url,
        interval,
    }
}

pub(super) fn sorted_candidate_node_names(candidates: &[NodePoolCandidate]) -> Vec<String> {
    let mut ordered = candidates.to_vec();
    ordered.sort_by(|left, right| {
        left.priority
            .unwrap_or(u32::MAX)
            .cmp(&right.priority.unwrap_or(u32::MAX))
            .then_with(|| left.node_name.cmp(&right.node_name))
    });

    let mut seen = BTreeSet::new();
    ordered
        .into_iter()
        .filter_map(|candidate| {
            let node_name = candidate.node_name.trim();
            if node_name.is_empty() || !seen.insert(node_name.to_owned()) {
                None
            } else {
                Some(node_name.into())
            }
        })
        .collect()
}

pub(super) fn mihomo_rules_for_app(
    app: &AppRegistryEntry,
    target: &str,
    warnings: &mut Vec<String>,
) -> Vec<MihomoRuleProjection> {
    let mut rules = Vec::new();
    for matcher in &app.process_matchers {
        let Some(mihomo_matcher) = mihomo_matcher_kind(matcher.kind) else {
            warnings.push(
                format!(
                    "process matcher `{:?}` cannot be projected to a Mihomo rule",
                    matcher.kind
                )
                .into(),
            );
            continue;
        };
        let value = matcher.pattern.trim();
        if value.is_empty() {
            warnings.push(format!("process matcher `{mihomo_matcher}` has an empty pattern").into());
            continue;
        }
        if value.contains(',') {
            warnings.push(format!("process matcher `{mihomo_matcher}` contains ',' and cannot be projected").into());
            continue;
        }

        rules.push(MihomoRuleProjection {
            matcher: mihomo_matcher.into(),
            value: value.into(),
            target: target.into(),
            rule: format!("{mihomo_matcher},{value},{target}").into(),
        });
    }

    if rules.is_empty() {
        warnings.push(format!("app `{}` has no Mihomo-compatible process matchers", app.app_id).into());
    }

    rules
}

pub(super) fn mihomo_matcher_kind(kind: AppProcessMatcherKind) -> Option<&'static str> {
    match kind {
        AppProcessMatcherKind::ProcessName => Some("PROCESS-NAME"),
        AppProcessMatcherKind::ProcessPath => Some("PROCESS-PATH"),
        AppProcessMatcherKind::ProcessNameRegex
        | AppProcessMatcherKind::ProcessPathRegex
        | AppProcessMatcherKind::BundleId => None,
    }
}

pub(super) fn mihomo_dns_projection(profile: &DnsProfilePlanView) -> MihomoDnsProjection {
    MihomoDnsProjection {
        profile_id: profile.profile_id.clone(),
        name: profile.name.clone(),
        nameservers: profile
            .resolver_plan
            .nameservers
            .iter()
            .map(|nameserver| nameserver.server.as_str().into())
            .collect(),
        runtime_supported_nameservers: profile
            .resolver_plan
            .nameservers
            .iter()
            .filter(|nameserver| nameserver.runtime_supported)
            .count(),
    }
}

pub(super) fn mihomo_yaml_patch(
    proxy_groups: &[MihomoProxyGroupProjection],
    rules: &[MihomoRuleProjection],
) -> Result<String> {
    if proxy_groups.is_empty() && rules.is_empty() {
        return Ok(String::new());
    }

    Ok(serde_yaml_ng::to_string(&MihomoYamlPatch {
        proxy_groups: proxy_groups.to_vec(),
        rules: rules.iter().map(|rule| rule.rule.clone()).collect(),
    })?
    .into())
}
