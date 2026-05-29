use super::*;
use std::sync::Arc;

    #[tokio::test]
    async fn test_domain_binding_basic() {
        let manager = Arc::new(SessionAffinityManager::new());
        let nodes = vec!["node1".to_string(), "node2".to_string()];

        // 第一次选择
        let node1 = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        // 第二次选择应该返回相同节点
        let node2 = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        assert_eq!(node1, node2, "同一域名应该绑定到相同节点");
    }

    #[tokio::test]
    async fn test_domain_binding_different_domains() {
        let manager = Arc::new(SessionAffinityManager::new());
        let nodes = vec!["node1".to_string(), "node2".to_string()];

        // 不同域名可能选择不同节点
        let node1 = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        let node2 = manager
            .select_node_for_domain("api.anthropic.com", &nodes)
            .await
            .unwrap();

        // 两个域名都应该成功选择节点
        assert!(nodes.contains(&node1));
        assert!(nodes.contains(&node2));
    }

    #[tokio::test]
    async fn test_domain_wildcard_matching() {
        assert!(domain_matches("chat.openai.com", "*.openai.com"));
        assert!(domain_matches("api.openai.com", "*.openai.com"));
        assert!(domain_matches("openai.com", "*.openai.com"));
        assert!(!domain_matches("openai.org", "*.openai.com"));
        assert!(!domain_matches("fakeopenai.com", "*.openai.com"));
    }

    #[tokio::test]
    async fn test_binding_expiration() {
        let manager = Arc::new(SessionAffinityManager::new());
        
        // 创建一个短期绑定的配置
        let mut config = SessionAffinityConfig::default();
        config.domain_rules = vec![DomainBindingRule {
            domain_pattern: "test.example.com".to_string(),
            enabled: true,
            bound_node: None,
            ttl: 1, // 1秒后过期
            fallback_policy: FallbackPolicy::AutoSwitch,
            description: "测试规则".to_string(),
        }];
        
        manager.update_config(config).await.unwrap();

        let nodes = vec!["node1".to_string(), "node2".to_string()];

        // 第一次选择
        let node1 = manager
            .select_node_for_domain("test.example.com", &nodes)
            .await
            .unwrap();

        // 等待绑定过期
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 清理过期绑定
        manager.cleanup_expired_bindings().await.unwrap();

        // 再次选择（应该创建新绑定）
        let node2 = manager
            .select_node_for_domain("test.example.com", &nodes)
            .await
            .unwrap();

        // 两次选择都应该成功
        assert!(nodes.contains(&node1));
        assert!(nodes.contains(&node2));
    }

    #[tokio::test]
    async fn test_get_bindings() {
        let manager = Arc::new(SessionAffinityManager::new());
        let nodes = vec!["node1".to_string()];

        // 创建几个绑定
        manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();
        
        manager
            .select_node_for_domain("api.anthropic.com", &nodes)
            .await
            .unwrap();

        // 获取绑定信息
        let bindings = manager.get_all_bindings().await.unwrap();
        
        assert_eq!(bindings.len(), 2, "应该有 2 个绑定");
        assert!(bindings.iter().any(|b| b.key == "chat.openai.com"));
        assert!(bindings.iter().any(|b| b.key == "api.anthropic.com"));
    }

    #[tokio::test]
    async fn test_clear_binding() {
        let manager = Arc::new(SessionAffinityManager::new());
        let nodes = vec!["node1".to_string()];

        // 创建绑定
        manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        // 验证绑定存在
        let bindings = manager.get_all_bindings().await.unwrap();
        assert_eq!(bindings.len(), 1);

        // 清除绑定
        manager.clear_domain_binding("chat.openai.com").await.unwrap();

        // 验证绑定已清除
        let bindings = manager.get_all_bindings().await.unwrap();
        assert_eq!(bindings.len(), 0);
    }

    #[tokio::test]
    async fn test_predefined_rules() {
        let rules = get_predefined_rules();
        
        assert!(!rules.is_empty(), "应该有预定义规则");
        
        // 验证关键服务的规则存在
        assert!(rules.iter().any(|r| r.domain_pattern == "*.openai.com"));
        assert!(rules.iter().any(|r| r.domain_pattern == "*.steampowered.com"));
        assert!(rules.iter().any(|r| r.domain_pattern == "*.stripe.com"));
        
        // 验证 TTL 设置合理
        for rule in &rules {
            assert!(rule.ttl > 0, "TTL 应该大于 0");
            assert!(rule.ttl <= 2592000, "TTL 不应超过 30 天");
        }
    }

    #[tokio::test]
    async fn test_connection_binding() {
        let manager = Arc::new(SessionAffinityManager::new());
        let nodes = vec!["node1".to_string(), "node2".to_string()];

        // 第一次选择
        let node1 = manager
            .select_node_for_connection("192.168.1.100", 12345, &nodes)
            .await
            .unwrap();

        // 第二次选择应该返回相同节点
        let node2 = manager
            .select_node_for_connection("192.168.1.100", 12345, &nodes)
            .await
            .unwrap();

        assert_eq!(node1, node2, "同一连接应该绑定到相同节点");
    }

    #[tokio::test]
    async fn test_disabled_session_affinity() {
        let manager = Arc::new(SessionAffinityManager::new());
        
        // 禁用会话绑定
        let mut config = SessionAffinityConfig::default();
        config.enabled = false;
        manager.update_config(config).await.unwrap();

        let nodes = vec!["node1".to_string(), "node2".to_string()];

        // 应该直接返回第一个节点，不创建绑定
        let node = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        assert_eq!(node, "node1");

        // 验证没有创建绑定
        let bindings = manager.get_all_bindings().await.unwrap();
        assert_eq!(bindings.len(), 0);
    }

    #[tokio::test]
    async fn test_runtime_domain_rule_binding_overrides_existing_domain_binding() {
        let manager = Arc::new(SessionAffinityManager::new());
        let nodes = vec!["node1".to_string(), "node2".to_string()];

        let first = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();
        assert_eq!(first, "node1");

        manager
            .record_domain_rule_binding("*.openai.com", "node2".to_string())
            .await
            .unwrap();

        let second = manager
            .select_node_for_domain("chat.openai.com", &nodes)
            .await
            .unwrap();

        assert_eq!(second, "node2");

        let bindings = manager.get_all_bindings().await.unwrap();
        assert!(bindings.iter().any(|binding| {
            binding.binding_type == "domain-rule"
                && binding.key == "rule:*.openai.com"
                && binding.node_id == "node2"
        }));

        manager
            .clear_domain_binding("rule:*.openai.com")
            .await
            .unwrap();

        let cleared_bindings = manager.get_all_bindings().await.unwrap();
        assert!(!cleared_bindings.iter().any(|binding| {
            binding.binding_type == "domain-rule" && binding.key == "rule:*.openai.com"
        }));
    }
