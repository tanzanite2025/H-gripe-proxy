use crate::multipath::{
    MultipathConfig, MultipathManager, NodePool, PathNode, PoolType, SessionBinding, SlicingStrategy, NodeStats,
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

fn multipath_manager() -> Arc<MultipathManager> {
    crate::feat::get_coordinator().multipath_manager()
}

fn persist_multipath_config(config: &MultipathConfig) -> Result<()> {
    let path = crate::config::AdvancedConfig::default_path()?;
    let mut advanced_config = crate::config::AdvancedConfig::load(&path)?;
    advanced_config.multipath = config.clone();
    advanced_config.save(&path)?;

    let coordinator = crate::feat::get_coordinator();
    coordinator.apply_advanced_config(&advanced_config)?;
    Ok(())
}

pub fn apply_multipath_config(config: MultipathConfig) -> Result<()> {
    let manager = multipath_manager();
    manager.update_config(config.clone());
    persist_multipath_config(&config)?;
    log::info!("[Multipath] config updated");
    Ok(())
}

/// 获取多路径配置
pub fn multipath_get_config() -> MultipathConfig {
    let manager = multipath_manager();
    manager.get_config()
}

/// 获取会话绑定规则
pub fn multipath_get_bindings() -> Vec<SessionBinding> {
    let manager = multipath_manager();
    manager.get_bindings()
}

/// 添加会话绑定规则
pub fn multipath_add_binding(binding: SessionBinding) -> Result<()> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.bindings.push(binding);
    apply_multipath_config(config)?;
    log::info!("已添加会话绑定规则");
    Ok(())
}

/// 删除会话绑定规则
pub fn multipath_remove_binding(domain_pattern: &str) -> Result<()> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.bindings.retain(|b| b.domain_pattern != domain_pattern);
    apply_multipath_config(config)?;
    log::info!("已删除会话绑定规则: {}", domain_pattern);
    Ok(())
}

/// 获取预定义的会话绑定规则
pub fn multipath_get_predefined_bindings() -> Vec<SessionBinding> {
    SessionBinding::all_predefined()
}

/// 添加节点池
pub fn multipath_add_pool(pool: NodePool) -> Result<()> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.node_pools.push(pool);
    apply_multipath_config(config)?;
    log::info!("已添加节点池");
    Ok(())
}

/// 删除节点池
pub fn multipath_remove_pool(pool_name: &str) -> Result<()> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.node_pools.retain(|p| p.name != pool_name);
    apply_multipath_config(config)?;
    log::info!("已删除节点池: {}", pool_name);
    Ok(())
}

/// 更新节点池
pub fn multipath_update_pool(pool: NodePool) -> Result<()> {
    let manager = multipath_manager();
    let mut config = manager.get_config();

    if let Some(existing) = config.node_pools.iter_mut().find(|p| p.name == pool.name) {
        *existing = pool;
        apply_multipath_config(config)?;
        log::info!("已更新节点池");
        Ok(())
    } else {
        Err(anyhow::anyhow!("节点池不存在"))
    }
}

/// 添加节点到池
pub fn multipath_add_node(pool_name: &str, node: PathNode) -> Result<()> {
    let manager = multipath_manager();
    let mut config = manager.get_config();

    if let Some(pool) = config.node_pools.iter_mut().find(|p| p.name == pool_name) {
        pool.nodes.push(node);
        apply_multipath_config(config)?;
        log::info!("已添加节点到池: {}", pool_name);
        Ok(())
    } else {
        Err(anyhow::anyhow!("节点池不存在"))
    }
}

/// 从池中删除节点
pub fn multipath_remove_node(pool_name: &str, node_name: &str) -> Result<()> {
    let manager = multipath_manager();
    let mut config = manager.get_config();

    if let Some(pool) = config.node_pools.iter_mut().find(|p| p.name == pool_name) {
        pool.nodes.retain(|n| n.name != node_name);
        apply_multipath_config(config)?;
        log::info!("已从池 {} 删除节点: {}", pool_name, node_name);
        Ok(())
    } else {
        Err(anyhow::anyhow!("节点池不存在"))
    }
}

/// 批量导入节点
pub fn multipath_import_nodes(pool_name: &str, nodes_yaml: &str) -> Result<usize> {
    let manager = multipath_manager();
    let mut config = manager.get_config();

    let nodes: Vec<PathNode> = serde_yaml_ng::from_str(nodes_yaml)?;
    if let Some(pool) = config.node_pools.iter_mut().find(|p| p.name == pool_name) {
        let count = nodes.len();
        pool.nodes.extend(nodes);
        apply_multipath_config(config)?;
        log::info!("已导入 {} 个节点到池: {}", count, pool_name);
        Ok(count)
    } else {
        Err(anyhow::anyhow!("节点池不存在"))
    }
}

/// 导出节点配置
pub fn multipath_export_nodes(pool_name: &str) -> Result<std::string::String> {
    let manager = multipath_manager();
    let config = manager.get_config();

    if let Some(pool) = config.node_pools.iter().find(|p| p.name == pool_name) {
        Ok(serde_yaml_ng::to_string(&pool.nodes)?)
    } else {
        Err(anyhow::anyhow!("节点池不存在"))
    }
}

/// 获取推荐配置
pub fn multipath_get_recommended_config() -> MultipathConfig {
    MultipathConfig {
        enabled: true,
        strategy: SlicingStrategy::Weighted,
        node_pools: vec![
            NodePool {
                name: "流媒体专用".to_string(),
                pool_type: PoolType::Streaming,
                nodes: Vec::new(),
                enabled: true,
            },
            NodePool {
                name: "游戏专用".to_string(),
                pool_type: PoolType::Gaming,
                nodes: Vec::new(),
                enabled: true,
            },
            NodePool {
                name: "下载专用".to_string(),
                pool_type: PoolType::Download,
                nodes: Vec::new(),
                enabled: true,
            },
            NodePool {
                name: "通用池".to_string(),
                pool_type: PoolType::General,
                nodes: Vec::new(),
                enabled: true,
            },
        ],
        min_fragment_size: 1024,
        max_fragment_size: 65536,
        reassembly_timeout: 5000,
        session_persistence: true,
        bindings: SessionBinding::all_predefined(),
    }
}

/// 获取节点统计
pub fn multipath_get_node_stats() -> HashMap<String, NodeStats> {
    let manager = multipath_manager();
    manager.get_node_stats()
}

/// Select a node for a multipath session.
pub fn multipath_select_node(domain: &str, session_id: u64) -> Option<String> {
    let manager = multipath_manager();
    manager.select_node(domain, session_id)
}

pub fn multipath_record_connection_end(node_name: &str, bytes: u64) {
    let manager = multipath_manager();
    manager.record_connection_end(node_name, bytes);
}

pub fn multipath_record_latency(node_name: &str, latency_us: u64) {
    let manager = multipath_manager();
    manager.record_latency(node_name, latency_us);
}

pub fn multipath_record_error(node_name: &str) {
    let manager = multipath_manager();
    manager.record_error(node_name);
}

pub fn multipath_get_active_session_count() -> usize {
    let manager = multipath_manager();
    manager.active_session_count()
}

/// 清理过期会话
pub fn multipath_cleanup_sessions() {
    let manager = multipath_manager();
    manager.cleanup_expired_sessions();
    log::info!("[Multipath] expired sessions cleaned");
}
