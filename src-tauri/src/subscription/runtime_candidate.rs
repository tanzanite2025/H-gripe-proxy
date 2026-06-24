use crate::{
    config::{
        Config, ConfigType, PrfItem,
        profiles::{IProfiles, resolve_profile_file_path},
    },
    core::{
        runtime_lifecycle,
        validate::{CoreConfigValidator, ValidationOutcome},
    },
    subscription::{
        model::SubscriptionSourceConfig,
        persist::{
            SubscriptionArtifactContentKind, read_subscription_artifact_content, read_subscription_source_state,
        },
    },
    utils::{dirs, help},
};
use anyhow::{Result, anyhow, bail};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use std::path::PathBuf;
use tokio::fs;

pub async fn validate_subscription_artifact_runtime_candidate(
    source_id: &String,
    artifact_version: &String,
    source_config: &SubscriptionSourceConfig,
) -> Result<ValidationOutcome> {
    let candidate_item =
        build_subscription_artifact_runtime_candidate_item(source_id, artifact_version, source_config).await?;

    validate_subscription_runtime_candidate(source_id, source_config, &candidate_item).await
}

async fn validate_subscription_runtime_candidate(
    source_id: &String,
    source_config: &SubscriptionSourceConfig,
    candidate_item: &PrfItem,
) -> Result<ValidationOutcome> {
    let candidate_profile = write_subscription_runtime_candidate_profile(candidate_item).await?;
    let validation = async {
        Config::profiles().await.edit_draft(|profiles| {
            apply_subscription_runtime_candidate_profile(
                profiles,
                source_id,
                source_config,
                candidate_profile.file.clone(),
            )
        })?;

        match Config::generate().await {
            Ok(()) => {
                let check_path = Config::generate_file(ConfigType::Check).await?;
                logging!(
                    info,
                    Type::Validate,
                    "[Subscription Update] generated runtime candidate for validation: {:?}",
                    check_path
                );
                let check_path = dirs::path_to_str(&check_path)?;
                CoreConfigValidator::validate_config_file_outcome(check_path, Some(false)).await
            }
            Err(err) => Ok(ValidationOutcome::invalid_from_message(format!(
                "failed to generate subscription runtime candidate: {err}"
            ))),
        }
    }
    .await;

    Config::profiles().await.discard();
    Config::runtime().await.discard();

    cleanup_subscription_runtime_candidate_profile(&candidate_profile).await;

    validation
}

pub async fn activate_subscription_active_artifact_runtime(
    source_id: &String,
    source_config: &SubscriptionSourceConfig,
    force: bool,
) -> Result<ValidationOutcome> {
    let artifact_version = active_subscription_artifact_version(source_id).await?;
    let candidate_item =
        build_subscription_artifact_runtime_candidate_item(source_id, &artifact_version, source_config).await?;
    let candidate_profile = write_subscription_runtime_candidate_profile(&candidate_item).await?;

    let result = async {
        Config::profiles().await.edit_draft(|profiles| {
            apply_subscription_runtime_candidate_profile(
                profiles,
                source_id,
                source_config,
                candidate_profile.file.clone(),
            )
        })?;

        runtime_lifecycle::update_runtime_config_with_force(force, "subscription-active-artifact").await
    }
    .await;

    Config::profiles().await.discard();
    cleanup_subscription_runtime_candidate_profile(&candidate_profile).await;

    result
}

struct RuntimeCandidateProfile {
    file: String,
    path: PathBuf,
}

async fn write_subscription_runtime_candidate_profile(candidate_item: &PrfItem) -> Result<RuntimeCandidateProfile> {
    let candidate_file = candidate_item
        .file
        .clone()
        .ok_or_else(|| anyhow!("subscription runtime candidate is missing profile file"))?;
    let candidate_data = candidate_item
        .file_data
        .clone()
        .ok_or_else(|| anyhow!("subscription runtime candidate is missing profile content"))?;
    let candidate_path = resolve_profile_file_path(candidate_file.as_str())?;

    if fs::try_exists(&candidate_path).await.unwrap_or(false) {
        bail!("subscription runtime candidate file already exists: {candidate_file}");
    }

    fs::write(&candidate_path, candidate_data.as_bytes()).await?;

    Ok(RuntimeCandidateProfile {
        file: candidate_file,
        path: candidate_path,
    })
}

async fn cleanup_subscription_runtime_candidate_profile(candidate_profile: &RuntimeCandidateProfile) {
    if let Err(err) = fs::remove_file(&candidate_profile.path).await {
        logging!(
            warn,
            Type::Config,
            "Warning: [Subscription Update] failed to remove runtime candidate file {:?}: {}",
            candidate_profile.path,
            err
        );
    }
}

async fn active_subscription_artifact_version(source_id: &String) -> Result<String> {
    let source_state = read_subscription_source_state(source_id.as_str())
        .await?
        .ok_or_else(|| anyhow!("subscription state is missing for source \"uid:{source_id}\""))?;

    source_state
        .active_artifact_version
        .ok_or_else(|| anyhow!("subscription source \"uid:{source_id}\" has no active artifact"))
}

async fn build_subscription_artifact_runtime_candidate_item(
    source_id: &String,
    artifact_version: &String,
    source_config: &SubscriptionSourceConfig,
) -> Result<PrfItem> {
    let artifact = read_subscription_artifact_content(
        source_id.as_str(),
        artifact_version.as_str(),
        SubscriptionArtifactContentKind::NormalizedYaml,
    )
    .await?
    .ok_or_else(|| {
        anyhow!(
            "active subscription artifact normalized.yaml is missing for uid:{source_id} version:{artifact_version}"
        )
    })?;

    Ok(runtime_candidate_item_from_artifact_content(
        source_id,
        source_config,
        artifact.version.as_str(),
        artifact.content,
    ))
}

fn runtime_candidate_item_from_artifact_content(
    source_id: &String,
    source_config: &SubscriptionSourceConfig,
    artifact_version: &str,
    normalized_yaml: String,
) -> PrfItem {
    PrfItem {
        uid: Some(source_id.clone()),
        itype: Some("remote".into()),
        file: Some(format!("{}-{artifact_version}.yaml", help::get_uid("S")).into()),
        updated: Some((source_config.updated_at / 1000) as usize),
        option: source_config.option.clone(),
        file_data: Some(normalized_yaml),
        ..PrfItem::default()
    }
}

fn apply_subscription_runtime_candidate_profile(
    profiles: &mut IProfiles,
    source_id: &String,
    source_config: &SubscriptionSourceConfig,
    candidate_file: String,
) -> Result<()> {
    let items = profiles.items.get_or_insert_with(Vec::new);
    let Some(item) = items.iter_mut().find(|item| item.uid.as_ref() == Some(source_id)) else {
        bail!("failed to find subscription runtime candidate source \"uid:{source_id}\"");
    };

    item.updated = Some((source_config.updated_at / 1000) as usize);
    item.option = source_config.option.clone();
    item.file = Some(candidate_file);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PrfOption;

    #[test]
    fn applies_runtime_candidate_profile_without_changing_current() {
        let source_id = String::from("source-a");
        let mut profiles = IProfiles {
            current: Some(source_id.clone()),
            items: Some(vec![PrfItem {
                uid: Some(source_id.clone()),
                itype: Some("remote".into()),
                file: Some("Rold.yaml".into()),
                option: Some(PrfOption {
                    update_interval: Some(12),
                    ..PrfOption::default()
                }),
                ..PrfItem::default()
            }]),
        };
        let source_config = SubscriptionSourceConfig {
            url: "https://example.com/sub.yaml".into(),
            option: Some(PrfOption {
                allow_auto_update: Some(false),
                ..PrfOption::default()
            }),
            updated_at: 123000,
        };

        apply_subscription_runtime_candidate_profile(
            &mut profiles,
            &source_id,
            &source_config,
            "Rcandidate.yaml".into(),
        )
        .expect("candidate profile");

        let item = profiles.get_item(source_id.as_str()).expect("source item");
        assert_eq!(profiles.current.as_deref(), Some("source-a"));
        assert_eq!(item.file.as_deref(), Some("Rcandidate.yaml"));
        assert_eq!(item.updated, Some(123));
        assert!(item.home.is_none());
        assert!(item.extra.is_none());
        let option = item.option.as_ref().expect("source option");
        assert_eq!(option.update_interval, None);
        assert_eq!(option.allow_auto_update, Some(false));
    }

    #[test]
    fn builds_runtime_candidate_item_from_artifact_content() {
        let source_id = String::from("source-a");
        let source_config = SubscriptionSourceConfig {
            url: "https://example.com/sub.yaml".into(),
            option: None,
            updated_at: 123000,
        };

        let candidate = runtime_candidate_item_from_artifact_content(
            &source_id,
            &source_config,
            "artifact-a",
            "proxies: []".into(),
        );

        assert_eq!(candidate.uid.as_deref(), Some("source-a"));
        assert_eq!(candidate.updated, Some(123));
        assert_eq!(candidate.itype.as_deref(), Some("remote"));
        assert_eq!(candidate.file_data.as_deref(), Some("proxies: []"));
        assert!(candidate.url.is_none());
        assert!(candidate.name.is_none());
        assert!(
            candidate
                .file
                .as_deref()
                .is_some_and(|file| file.ends_with("-artifact-a.yaml"))
        );
    }

    #[test]
    fn candidate_profile_requires_existing_source() {
        let source_id = String::from("missing");
        let mut profiles = IProfiles::default();

        assert!(
            apply_subscription_runtime_candidate_profile(
                &mut profiles,
                &source_id,
                &SubscriptionSourceConfig {
                    url: "https://example.com/sub.yaml".into(),
                    option: None,
                    updated_at: 0,
                },
                "Rcandidate.yaml".into(),
            )
            .is_err()
        );
    }
}
