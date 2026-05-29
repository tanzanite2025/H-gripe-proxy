import { invoke } from '@tauri-apps/api/core';

/**
 * IP 信誉度配置
 */
export interface IpReputationConfig {
  enabled: boolean;
  cacheTtl: number;
  routingRules: RiskRoutingRule[];
  useLocalDb: boolean;
}

/**
 * IP 信誉度信息
 */
export interface IpReputation {
  ip: string;
  ipType: 'Datacenter' | 'Residential' | 'Mobile' | 'Unknown';
  asn: string;
  asnOrg: string;
  fraudScore: number;
  riskLevel: 'Low' | 'Medium' | 'High' | 'VeryHigh';
  isProxy: boolean;
  isVpn: boolean;
  isTor: boolean;
  countryCode: string;
  city?: string;
  checkedAt: number;
}

/**
 * 风控等级路由规则
 */
export interface RiskRoutingRule {
  domainPatterns: string[];
  enabled: boolean;
  requiredIpType?: 'Datacenter' | 'Residential' | 'Mobile';
  maxFraudScore: number;
  fallbackPolicy: 'Block' | 'Warn' | 'Allow';
  description: string;
}

/**
 * 获取 IP 信誉度配置
 */
export async function ipReputationGetConfig(): Promise<IpReputationConfig> {
  return await invoke<IpReputationConfig>('ip_reputation_get_config');
}

/**
 * 更新 IP 信誉度配置
 */
export async function ipReputationUpdateConfig(
  config: IpReputationConfig
): Promise<void> {
  await invoke('ip_reputation_update_config', { config });
}

/**
 * 检测 IP 信誉度
 */
export async function ipReputationCheckIp(ip: string): Promise<IpReputation> {
  return await invoke<IpReputation>('ip_reputation_check_ip', { ip });
}

/**
 * 获取预定义路由规则
 */
export async function ipReputationGetPredefinedRules(): Promise<RiskRoutingRule[]> {
  return await invoke<RiskRoutingRule[]>('ip_reputation_get_predefined_rules');
}

/**
 * 为域名选择节点
 */
export async function ipReputationSelectNodeForDomain(
  domain: string,
  availableNodes: [string, string][]
): Promise<string> {
  return await invoke<string>('ip_reputation_select_node_for_domain', {
    domain,
    availableNodes,
  });
}

/**
 * 清除缓存
 */
export async function ipReputationClearCache(): Promise<void> {
  await invoke('ip_reputation_clear_cache');
}

/**
 * 获取缓存统计
 */
export async function ipReputationGetCacheStats(): Promise<[number, number]> {
  return await invoke<[number, number]>('ip_reputation_get_cache_stats');
}

/**
 * 获取 IP 类型的显示文本
 */
export function getIpTypeText(ipType: string): string {
  switch (ipType) {
    case 'Datacenter':
      return '机房 IP';
    case 'Residential':
      return '住宅 IP';
    case 'Mobile':
      return '移动 IP';
    default:
      return '未知';
  }
}

/**
 * 获取风险等级的显示文本
 */
export function getRiskLevelText(riskLevel: string): string {
  switch (riskLevel) {
    case 'Low':
      return '低风险';
    case 'Medium':
      return '中风险';
    case 'High':
      return '高风险';
    case 'VeryHigh':
      return '极高风险';
    default:
      return '未知';
  }
}

/**
 * 获取风险等级的颜色
 */
export function getRiskLevelColor(riskLevel: string): string {
  switch (riskLevel) {
    case 'Low':
      return 'text-green-600';
    case 'Medium':
      return 'text-yellow-600';
    case 'High':
      return 'text-orange-600';
    case 'VeryHigh':
      return 'text-red-600';
    default:
      return 'text-gray-600';
  }
}
