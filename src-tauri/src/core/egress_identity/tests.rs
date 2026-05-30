use super::*;
use crate::core::ip_reputation::IpType;

#[test]
fn test_validate_recommended_config() {
    let config = EgressIdentityConfig::recommended();
    assert!(config.validate().is_ok());
}

#[test]
fn test_preview_match_uses_default_profile_with_matching_metadata() {
    let mut config = EgressIdentityConfig::recommended();
    config.enabled = true;

    let manager = EgressIdentityManager::new_with_config(config);
    let resolved = manager
        .preview_match(EgressSelectionContext {
            available_nodes: vec!["node-a".to_string()],
            available_node_metadata: vec![EgressNodeMetadata {
                name: "node-a".to_string(),
                pool_name: Some("通用池".to_string()),
                ip_type: Some(IpType::Residential),
                fraud_score: Some(15),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(resolved.profile_id, "stable-default");
    assert_eq!(resolved.selected_node, "node-a");
}

#[test]
fn test_preview_match_uses_domain_only_app_rule() {
    let mut config = EgressIdentityConfig::recommended();
    config.enabled = true;

    let manager = EgressIdentityManager::new_with_config(config);
    let resolved = manager
        .preview_match(EgressSelectionContext {
            domain: Some("chat.openai.com".to_string()),
            available_nodes: vec!["node-resi".to_string(), "node-dc".to_string()],
            available_node_metadata: vec![
                EgressNodeMetadata {
                    name: "node-resi".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(15),
                    ..Default::default()
                },
                EgressNodeMetadata {
                    name: "node-dc".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Datacenter),
                    fraud_score: Some(85),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(resolved.profile_id, "ai-strict");
    assert_eq!(resolved.selected_node, "node-resi");
    assert!(matches!(resolved.dns_mode, DnsMode::Remote));
}

#[test]
fn test_assign_records_active_assignment() {
    let mut config = EgressIdentityConfig::recommended();
    config.enabled = true;

    let manager = EgressIdentityManager::new_with_config(config);
    let resolved = manager
        .assign(EgressSelectionContext {
            shortcut_id: Some("chatgpt".to_string()),
            available_nodes: vec!["node-resi".to_string(), "node-dc".to_string()],
            available_node_metadata: vec![
                EgressNodeMetadata {
                    name: "node-resi".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(15),
                    ..Default::default()
                },
                EgressNodeMetadata {
                    name: "node-dc".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Datacenter),
                    fraud_score: Some(85),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(resolved.profile_id, "ai-strict");
    assert_eq!(resolved.selected_node, "node-resi");
    assert_eq!(manager.get_active_assignments().len(), 1);
}

#[test]
fn test_preferred_pool_and_score_constraints_filter_candidates() {
    let config = EgressIdentityConfig {
        enabled: true,
        default_profile: Some("pool-pref".to_string()),
        profiles: vec![EgressIdentityProfile {
            id: "pool-pref".to_string(),
            name: "池偏好画像".to_string(),
            enabled: true,
            preferred_nodes: Vec::new(),
            preferred_pools: vec!["ResidentialPool".to_string()],
            required_ip_type: Some(IpType::Residential),
            max_fraud_score: Some(30),
            dns_policy: DnsPolicy::default(),
            tls_fingerprint: None,
            session_policy: IdentitySessionPolicy::default(),
            failover_policy: EgressFailoverPolicy::Block,
            allowed_nodes: Vec::new(),
            strict_node_scope: false,
            description: String::new(),
        }],
        app_rules: Vec::new(),
        shortcut_rules: Vec::new(),
    };

    let manager = EgressIdentityManager::new_with_config(config);
    let resolved = manager
        .preview_match(EgressSelectionContext {
            available_nodes: vec!["node-a".to_string(), "node-b".to_string()],
            available_node_metadata: vec![
                EgressNodeMetadata {
                    name: "node-a".to_string(),
                    pool_name: Some("GeneralPool".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(20),
                    ..Default::default()
                },
                EgressNodeMetadata {
                    name: "node-b".to_string(),
                    pool_name: Some("ResidentialPool".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(10),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(resolved.selected_node, "node-b");
}

#[test]
fn test_domain_override_beats_existing_exact_domain_assignment() {
    let mut config = EgressIdentityConfig::recommended();
    config.enabled = true;

    let manager = EgressIdentityManager::new_with_config(config);
    let domain_ctx = EgressSelectionContext {
        domain: Some("chat.openai.com".to_string()),
        available_nodes: vec!["node-resi-a".to_string(), "node-resi-b".to_string()],
        available_node_metadata: vec![
            EgressNodeMetadata {
                name: "node-resi-a".to_string(),
                pool_name: Some("通用池".to_string()),
                ip_type: Some(IpType::Residential),
                fraud_score: Some(15),
                ..Default::default()
            },
            EgressNodeMetadata {
                name: "node-resi-b".to_string(),
                pool_name: Some("通用池".to_string()),
                ip_type: Some(IpType::Residential),
                fraud_score: Some(20),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let first = manager.assign(domain_ctx.clone()).unwrap();
    assert_eq!(first.selected_node, "node-resi-a");

    manager
        .record_domain_override(
            "*.openai.com",
            EgressSelectionContext {
                domain: Some("openai.com".to_string()),
                available_nodes: domain_ctx.available_nodes.clone(),
                available_node_metadata: domain_ctx.available_node_metadata.clone(),
                ..Default::default()
            },
            "node-resi-b".to_string(),
        )
        .unwrap();

    let updated = manager.assign(domain_ctx).unwrap();
    assert_eq!(updated.profile_id, "ai-strict");
    assert_eq!(updated.selected_node, "node-resi-b");
    assert_eq!(updated.matched_by, "runtime_domain_override:*.openai.com");

    let assignments = manager.get_active_assignments();
    assert!(assignments.iter().any(|assignment| {
        assignment.assignment_key.as_deref() == Some("domain-pattern:*.openai.com")
            && assignment.selected_node == "node-resi-b"
    }));

    manager.clear_assignment("domain-pattern:*.openai.com");

    let reassigned = manager
        .assign(EgressSelectionContext {
            domain: Some("chat.openai.com".to_string()),
            available_nodes: vec!["node-resi-a".to_string(), "node-resi-b".to_string()],
            available_node_metadata: vec![
                EgressNodeMetadata {
                    name: "node-resi-a".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(15),
                    ..Default::default()
                },
                EgressNodeMetadata {
                    name: "node-resi-b".to_string(),
                    pool_name: Some("通用池".to_string()),
                    ip_type: Some(IpType::Residential),
                    fraud_score: Some(20),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(reassigned.selected_node, "node-resi-a");
}
