/**
 * 重绑定策略
 *
 * 定义 RebindStrategy trait，后续智能故障转移只需新增策略实现。
 */

use std::future::Future;
use std::pin::Pin;

use crate::core::{handle, runtime_snapshot::RuntimeSnapshotService};
use tauri::Emitter;

/// 重绑定上下文：IP 变化时传给策略的画像信息
#[derive(Debug, Clone)]
pub struct RebindContext {
    /// 变化前的出口 IP
    pub previous_ip: String,
    /// 变化后的出口 IP
    pub current_ip: String,
    /// 变化前的国家代码
    pub previous_country: Option<String>,
    /// 变化后的国家代码
    pub current_country: Option<String>,
}

/// 重绑定策略 trait
pub trait RebindStrategy: Send + Sync {
    /// 在检测到 IP 变化后执行重绑定
    /// 返回 true 表示至少有一个组成功切换
    fn rebind(&self, ctx: RebindContext) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
}

// ── Round-Robin 策略 ──────────────────────────────────────────────────

/// 简单轮转策略：切换到 VERGE-STABLE-* 组中的下一个节点
pub struct RoundRobinRebind;

impl RebindStrategy for RoundRobinRebind {
    fn rebind(&self, ctx: RebindContext) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let current_ip = ctx.current_ip.clone();
        Box::pin(async move {
            log::info!("[RebindStrategy::RoundRobin] 尝试自动重绑定，新 IP: {}", current_ip);

            // 获取当前所有 VERGE-STABLE-* 组及其选中节点
            let snapshot_service = RuntimeSnapshotService::global();
            let proxies = match snapshot_service.refresh_proxies_result().await {
                Ok(snapshot) => match snapshot.proxies {
                    Some(proxies) => proxies,
                    None => {
                        log::warn!("[RebindStrategy::RoundRobin] core is not running");
                        return false;
                    }
                },
                Err(e) => {
                    log::warn!("[RebindStrategy::RoundRobin] 获取代理组失败: {:?}", e);
                    return false;
                }
            };

            let stable_groups: Vec<_> = proxies
                .proxies
                .iter()
                .filter(|(name, _)| name.starts_with("VERGE-STABLE-"))
                .collect();

            if stable_groups.is_empty() {
                log::debug!("[RebindStrategy::RoundRobin] 无 VERGE-STABLE-* 组，跳过重绑定");
                return false;
            }

            let mut any_switched = false;

            for (group_name, group_data) in &stable_groups {
                let current_node = group_data.now.as_deref().unwrap_or("");
                let Some(all_nodes) = group_data.all.as_ref() else {
                    continue;
                };

                if all_nodes.len() <= 1 {
                    continue;
                }

                // 找到当前节点在列表中的位置，尝试切换到下一个节点
                let current_idx = all_nodes
                    .iter()
                    .position(|n| n.as_str() == current_node)
                    .unwrap_or(0);

                let next_idx = (current_idx + 1) % all_nodes.len();
                let next_node = &all_nodes[next_idx];

                if next_node.as_str() == current_node {
                    continue;
                }

                match handle::Handle::mihomo()
                    .await
                    .select_node_for_group(group_name, next_node)
                    .await
                {
                    Ok(_) => {
                        log::info!(
                            "[RebindStrategy::RoundRobin] 自动重绑定: {} 从 {} 切换到 {}",
                            group_name,
                            current_node,
                            next_node
                        );
                        any_switched = true;
                    }
                    Err(e) => {
                        log::warn!(
                            "[RebindStrategy::RoundRobin] 重绑定失败: {} -> {}: {:?}",
                            group_name,
                            next_node,
                            e
                        );
                    }
                }
            }

            if any_switched {
                // 同步运行态回写
                backwrite_after_rebind().await;
                let _ = handle::Handle::app_handle().emit("verge://refresh-proxy-config", ());
            }

            any_switched
        })
    }
}

/// 重绑定后回写 egress_identity 和 session_affinity
async fn backwrite_after_rebind() {
    if let Some(runtime_config) = crate::config::Config::runtime().await.latest_arc().config.clone() {
        let coordinator = crate::feat::get_coordinator();
        let session_affinity = crate::feat::get_session_affinity_manager();
        let ip_reputation = crate::feat::get_ip_reputation_manager();
        if let Err(e) = crate::core::stable_egress::sync_runtime_stable_egress_selection(
            &coordinator,
            &session_affinity,
            &ip_reputation,
            &runtime_config,
        ).await {
            log::warn!("[RebindStrategy] 重绑定后回写失败: {}", e);
        }
    }
}

// ── 同画像优先策略 ────────────────────────────────────────────────────

/// 智能重绑定：优先选择与变化前 IP 同国家/同类型的节点
///
/// 评分规则：
/// - 同国家 +40 分
/// - 同 IP 类型（residential/datacenter/etc）+30 分
/// - fraud_score 越低越好：100 - fraud_score 分
/// - 非代理/VPN +20 分
/// - 当前节点（排除）0 分
pub struct SmartRebind;

impl RebindStrategy for SmartRebind {
    fn rebind(&self, ctx: RebindContext) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let ctx = ctx.clone();
        Box::pin(async move {
            log::info!(
                "[RebindStrategy::Smart] 尝试智能重绑定: {} ({:?}) -> {} ({:?})",
                ctx.previous_ip,
                ctx.previous_country,
                ctx.current_ip,
                ctx.current_country,
            );

            let target_country = ctx.previous_country.as_deref().unwrap_or("");

            // 获取代理组
            let snapshot_service = RuntimeSnapshotService::global();
            let proxies = match snapshot_service.refresh_proxies_result().await {
                Ok(snapshot) => match snapshot.proxies {
                    Some(proxies) => proxies,
                    None => {
                        log::warn!("[RebindStrategy::Smart] core is not running");
                        return false;
                    }
                },
                Err(e) => {
                    log::warn!("[RebindStrategy::Smart] 获取代理组失败: {:?}", e);
                    return false;
                }
            };

            let ip_reputation_manager = crate::feat::get_ip_reputation_manager();

            let stable_groups: Vec<_> = proxies
                .proxies
                .iter()
                .filter(|(name, _)| name.starts_with("VERGE-STABLE-"))
                .collect();

            if stable_groups.is_empty() {
                log::debug!("[RebindStrategy::Smart] 无 VERGE-STABLE-* 组，跳过重绑定");
                return false;
            }

            // 构建所有候选节点列表，用于批量获取元数据
            let mut all_candidate_nodes: Vec<String> = Vec::new();
            for (_, group_data) in &stable_groups {
                if let Some(nodes) = &group_data.all {
                    for node in nodes {
                        if !all_candidate_nodes.iter().any(|n| n == node) {
                            all_candidate_nodes.push(node.clone());
                        }
                    }
                }
            }

            // 使用 enrich_egress_selection_context 批量获取节点元数据
            let coordinator = crate::feat::get_coordinator();
            let ctx = crate::core::egress_identity::EgressSelectionContext {
                available_nodes: all_candidate_nodes.clone(),
                ..Default::default()
            };
            let enriched_ctx = crate::core::stable_egress::enrich_egress_selection_context(
                ctx,
                &coordinator.multipath_manager(),
                &ip_reputation_manager,
            ).await;

            // 建立元数据索引
            let metadata_index: std::collections::HashMap<String, _> = enriched_ctx
                .available_node_metadata
                .into_iter()
                .map(|m| (m.name.clone(), m))
                .collect();

            // 建立延迟索引：节点名 -> 最近延迟 ms
            let delay_index: std::collections::HashMap<String, u16> = proxies
                .proxies
                .iter()
                .filter_map(|(name, data)| {
                    let delay = data.history.last().map(|h| h.delay).unwrap_or(0);
                    if delay > 0 && delay < 10000 {
                        Some((name.clone(), delay))
                    } else {
                        None
                    }
                })
                .collect();

            let mut any_switched = false;

            for (group_name, group_data) in &stable_groups {
                let current_node = group_data.now.as_deref().unwrap_or("");
                let Some(all_nodes) = group_data.all.as_ref() else {
                    continue;
                };

                if all_nodes.len() <= 1 {
                    continue;
                }

                // 为每个候选节点评分
                let mut best_node: Option<String> = None;
                let mut best_score: i32 = -1;

                for node_name in all_nodes {
                    // 跳过当前节点（正是它导致了 IP 变化）
                    if node_name.as_str() == current_node {
                        continue;
                    }

                    let metadata = metadata_index.get(node_name.as_str());
                    let delay = delay_index.get(node_name.as_str()).copied();
                    let score = score_node(
                        node_name,
                        target_country,
                        metadata,
                        delay,
                    ).await;

                    if score > best_score {
                        best_score = score;
                        best_node = Some(node_name.clone());
                    }
                }

                let Some(best_node) = best_node else {
                    log::debug!(
                        "[RebindStrategy::Smart] 组 {} 无可用候选节点",
                        group_name
                    );
                    continue;
                };

                match handle::Handle::mihomo()
                    .await
                    .select_node_for_group(group_name, &best_node)
                    .await
                {
                    Ok(_) => {
                        log::info!(
                            "[RebindStrategy::Smart] 智能重绑定: {} 从 {} 切换到 {} (评分: {})",
                            group_name,
                            current_node,
                            best_node,
                            best_score
                        );
                        any_switched = true;
                    }
                    Err(e) => {
                        log::warn!(
                            "[RebindStrategy::Smart] 重绑定失败: {} -> {}: {:?}",
                            group_name,
                            best_node,
                            e
                        );
                    }
                }
            }

            if any_switched {
                backwrite_after_rebind().await;
                let _ = handle::Handle::app_handle().emit("verge://refresh-proxy-config", ());
            }

            any_switched
        })
    }
}

/// 为节点评分：同画像优先 + 延迟加分
async fn score_node(
    node_name: &str,
    target_country: &str,
    metadata: Option<&crate::core::egress_identity::EgressNodeMetadata>,
    delay_ms: Option<u16>,
) -> i32 {
    let mut score: i32 = 0;

    // 延迟评分：延迟越低分越高
    if let Some(delay) = delay_ms {
        match delay {
            d if d < 100 => score += 25,
            d if d < 300 => score += 15,
            d if d < 500 => score += 5,
            _ => {} // >500ms 无加分
        }
    }

    if let Some(metadata) = metadata {
        // 同 IP 类型 +30（residential 最优）
        if let Some(ref ip_type) = metadata.ip_type {
            match ip_type {
                crate::core::ip_reputation::IpType::Residential => score += 30,
                crate::core::ip_reputation::IpType::Datacenter => score += 15,
                crate::core::ip_reputation::IpType::Mobile => score += 25,
                _ => score += 5,
            }
        }

        // fraud_score 越低越好
        if let Some(fraud_score) = metadata.fraud_score {
            score += (100 - fraud_score as i32).max(0);
        }

        // 如果有 server IP，额外查询国家代码
        if let Some(ref server) = metadata.server {
            let ip_reputation_manager = crate::feat::get_ip_reputation_manager();
            if let Some(server_ip) = crate::core::stable_egress::resolve_server_ip(server).await {
                if let Ok(reputation) = ip_reputation_manager.inspect_ip_metadata(&server_ip).await {
                    // 同国家 +40
                    if !target_country.is_empty()
                        && reputation.country_code.eq_ignore_ascii_case(target_country)
                    {
                        score += 40;
                    }
                    // 非代理/VPN +20
                    if !reputation.is_proxy && !reputation.is_vpn {
                        score += 20;
                    }
                }
            }
        }

        // 无信誉度信息时给基础分
        if metadata.ip_type.is_none() && metadata.fraud_score.is_none() {
            score += 50;
        }
    } else {
        // 无元数据，给基础分
        log::debug!("[RebindStrategy::Smart] 节点 {} 无元数据，使用基础分", node_name);
        score += 50;
    }

    score
}
