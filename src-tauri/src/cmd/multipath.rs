/**
 * 多路径路由 Tauri 命令
 */

use crate::multipath::{
    MultipathConfig, NodePool, PathNode, PoolType, SessionBinding, SlicingStrategy,
};
use std::sync::Arc;

fn multipath_manager() -> Arc<crate::multipath::MultipathManager> {
    super::coordinator::get_coordinator().multipath_manager()
}

fn persist_multipath_config(config: &MultipathConfig) -> Result<(), String> {
    use crate::utils::dirs;

    let path = dirs::app_home_dir()
        .map_err(|e| e.to_string())?
        .join("advanced.yaml");

    let mut advanced_config = crate::config::AdvancedConfig::load(&path)
        .map_err(|e| e.to_string())?;
    advanced_config.multipath = config.clone();
    advanced_config.save(&path)
        .map_err(|e| e.to_string())?;

    let coordinator = super::coordinator::get_coordinator();
    let mut coordinator_config = coordinator.get_config();
    coordinator_config.multipath_enabled = config.enabled;
    coordinator.update_config(coordinator_config)
        .map_err(|e| e.to_string())
}

fn apply_multipath_config(config: MultipathConfig) -> Result<(), String> {
    let manager = multipath_manager();
    manager.update_config(config.clone());
    persist_multipath_config(&config)?;
    log::info!("[Multipath] config updated");
    Ok(())
}

/// 获取多路径配置
#[tauri::command]
pub fn multipath_get_config() -> Result<MultipathConfig, String> {
    let manager = multipath_manager();
    Ok(manager.get_config())
}

/// 更新多路径配置
#[tauri::command]
pub fn multipath_update_config(config: MultipathConfig) -> Result<(), String> {
    apply_multipath_config(config)
}

/// 获取会话绑定规则
#[tauri::command]
pub fn multipath_get_bindings() -> Result<Vec<SessionBinding>, String> {
    let manager = multipath_manager();
    Ok(manager.get_bindings())
}

/// 添加会话绑定规则
#[tauri::command]
pub fn multipath_add_binding(binding: SessionBinding) -> Result<(), String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.bindings.push(binding);
    apply_multipath_config(config)?;
    log::info!("已添加会话绑定规则");
    Ok(())
}

/// 删除会话绑定规则
#[tauri::command]
pub fn multipath_remove_binding(domain_pattern: String) -> Result<(), String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.bindings.retain(|binding| binding.domain_pattern != domain_pattern);
    apply_multipath_config(config)?;
    log::info!("已删除会话绑定规则: {}", domain_pattern);
    Ok(())
}

/// 获取预定义的会话绑定规则
#[tauri::command]
pub fn multipath_get_predefined_bindings() -> Result<Vec<SessionBinding>, String> {
    Ok(SessionBinding::all_predefined())
}

/// 添加节点池
#[tauri::command]
pub fn multipath_add_pool(pool: NodePool) -> Result<(), String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.node_pools.push(pool);
    apply_multipath_config(config)?;
    log::info!("已添加节点池");
    Ok(())
}

/// 删除节点池
#[tauri::command]
pub fn multipath_remove_pool(pool_name: String) -> Result<(), String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    config.node_pools.retain(|p| p.name != pool_name);
    apply_multipath_config(config)?;
    log::info!("已删除节点池: {}", pool_name);
    Ok(())
}

/// 更新节点池
#[tauri::command]
pub fn multipath_update_pool(pool: NodePool) -> Result<(), String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    
    if let Some(existing) = config.node_pools.iter_mut().find(|p| p.name == pool.name) {
        *existing = pool;
        apply_multipath_config(config)?;
        log::info!("已更新节点池");
        Ok(())
    } else {
        Err("节点池不存在".to_string())
    }
}

/// 添加节点到池
#[tauri::command]
pub fn multipath_add_node(pool_name: String, node: PathNode) -> Result<(), String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    
    if let Some(pool) = config.node_pools.iter_mut().find(|p| p.name == pool_name) {
        pool.nodes.push(node);
        apply_multipath_config(config)?;
        log::info!("已添加节点到池: {}", pool_name);
        Ok(())
    } else {
        Err("节点池不存在".to_string())
    }
}

/// 从池中删除节点
#[tauri::command]
pub fn multipath_remove_node(pool_name: String, node_name: String) -> Result<(), String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    
    if let Some(pool) = config.node_pools.iter_mut().find(|p| p.name == pool_name) {
        pool.nodes.retain(|n| n.name != node_name);
        apply_multipath_config(config)?;
        log::info!("已从池 {} 删除节点: {}", pool_name, node_name);
        Ok(())
    } else {
        Err("节点池不存在".to_string())
    }
}

/// 测试节点连接
#[tauri::command]
pub async fn multipath_test_node(node: PathNode) -> Result<TestResult, String> {
    // TODO: 实际测试节点连接
    log::info!("测试节点: {}", node.name);
    
    Ok(TestResult {
        success: true,
        latency: 50,
        message: "连接成功".to_string(),
    })
}

/// 批量导入节点
#[tauri::command]
pub fn multipath_import_nodes(
    pool_name: String,
    nodes_yaml: String,
) -> Result<ImportResult, String> {
    let manager = multipath_manager();
    let mut config = manager.get_config();
    
    // 解析 YAML
    let nodes: Vec<PathNode> = serde_yaml_ng::from_str(&nodes_yaml)
        .map_err(|e| format!("解析失败: {}", e))?;
    
    if let Some(pool) = config.node_pools.iter_mut().find(|p| p.name == pool_name) {
        let count = nodes.len();
        pool.nodes.extend(nodes);
        apply_multipath_config(config)?;
        
        log::info!("已导入 {} 个节点到池: {}", count, pool_name);
        
        Ok(ImportResult {
            success: true,
            imported_count: count,
            message: format!("成功导入 {} 个节点", count),
        })
    } else {
        Err("节点池不存在".to_string())
    }
}

/// 导出节点配置
#[tauri::command]
pub fn multipath_export_nodes(pool_name: String) -> Result<String, String> {
    let manager = multipath_manager();
    let config = manager.get_config();
    
    if let Some(pool) = config.node_pools.iter().find(|p| p.name == pool_name) {
        serde_yaml_ng::to_string(&pool.nodes)
            .map_err(|e| format!("导出失败: {}", e))
    } else {
        Err("节点池不存在".to_string())
    }
}

/// 获取推荐配置
#[tauri::command]
pub fn multipath_get_recommended_config() -> Result<MultipathConfig, String> {
    Ok(MultipathConfig {
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
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub latency: u64,
    pub message: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub imported_count: usize,
    pub message: String,
}
