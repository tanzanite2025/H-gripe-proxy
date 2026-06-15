use crate::{
    config::{
        Config, ConfigType, PrfItem, PrfOption,
        profiles::{IProfiles, resolve_profile_file_path},
    },
    core::validate::{CoreConfigValidator, ValidationOutcome},
    utils::dirs,
};
use anyhow::{Result, anyhow, bail};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;
use tokio::fs;

pub async fn validate_subscription_runtime_candidate(
    source_id: &String,
    candidate_item: &PrfItem,
) -> Result<ValidationOutcome> {
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
    let validation = async {
        Config::profiles().await.edit_draft(|profiles| {
            apply_subscription_runtime_candidate_profile(
                profiles,
                source_id,
                candidate_item,
                candidate_file.clone(),
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

    if let Err(err) = fs::remove_file(&candidate_path).await {
        logging!(
            warn,
            Type::Config,
            "Warning: [Subscription Update] failed to remove runtime candidate file {:?}: {}",
            candidate_path,
            err
        );
    }

    validation
}

fn apply_subscription_runtime_candidate_profile(
    profiles: &mut IProfiles,
    source_id: &String,
    candidate_item: &PrfItem,
    candidate_file: String,
) -> Result<()> {
    let items = profiles.items.get_or_insert_with(Vec::new);
    let Some(item) = items
        .iter_mut()
        .find(|item| item.uid.as_ref() == Some(source_id))
    else {
        bail!("failed to find subscription runtime candidate source \"uid:{source_id}\"");
    };

    item.extra = candidate_item.extra.clone();
    item.updated = candidate_item.updated;
    item.home = candidate_item.home.clone();
    item.option = PrfOption::merge(item.option.as_ref(), candidate_item.option.as_ref());
    item.file = Some(candidate_file);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PrfExtra, PrfOption};

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
        let candidate = PrfItem {
            extra: Some(PrfExtra {
                upload: 1,
                download: 2,
                total: 3,
                expire: 4,
            }),
            updated: Some(123),
            home: Some("https://example.com".into()),
            option: Some(PrfOption {
                allow_auto_update: Some(false),
                ..PrfOption::default()
            }),
            ..PrfItem::default()
        };

        apply_subscription_runtime_candidate_profile(
            &mut profiles,
            &source_id,
            &candidate,
            "Rcandidate.yaml".into(),
        )
        .expect("candidate profile");

        let item = profiles.get_item(source_id.as_str()).expect("source item");
        assert_eq!(profiles.current.as_deref(), Some("source-a"));
        assert_eq!(item.file.as_deref(), Some("Rcandidate.yaml"));
        assert_eq!(item.updated, Some(123));
        assert_eq!(item.home.as_deref(), Some("https://example.com"));
        assert_eq!(item.extra.as_ref().map(|extra| extra.total), Some(3));
        let option = item.option.as_ref().expect("merged option");
        assert_eq!(option.update_interval, Some(12));
        assert_eq!(option.allow_auto_update, Some(false));
    }

    #[test]
    fn candidate_profile_requires_existing_source() {
        let source_id = String::from("missing");
        let mut profiles = IProfiles::default();

        assert!(
            apply_subscription_runtime_candidate_profile(
                &mut profiles,
                &source_id,
                &PrfItem::default(),
                "Rcandidate.yaml".into(),
            )
            .is_err()
        );
    }
}
