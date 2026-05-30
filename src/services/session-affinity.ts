import { invoke } from '@tauri-apps/api/core';

/**
 * 会话绑定配置
 */
export interface SessionAffinityConfig {
  enabled: boolean;
  domainRules: DomainBindingRule[];
  processRules: ProcessBindingRule[];
  connectionBinding: ConnectionBindingConfig;
}

/**
 * 域名绑定规则
 */
export interface DomainBindingRule {
  domainPattern: string;
  enabled: boolean;
  boundNode?: string;
  ttl: number;
  fallbackPolicy: 'Manual' | 'AutoRetry' | 'AutoSwitch';
  description: string;
}

/**
 * 进程绑定规则
 */
export interface ProcessBindingRule {
  processName: string;
  enabled: boolean;
  boundNode?: string;
  ttl: number;
  fallbackPolicy: 'Manual' | 'AutoRetry' | 'AutoSwitch';
  description: string;
}

/**
 * 连接级绑定配置
 */
export interface ConnectionBindingConfig {
  enabled: boolean;
  trackBy: 'SourceIpPort' | 'SessionId';
  timeout: number;
}

/**
 * 绑定信息
 */
export interface BindingInfo {
  bindingType: string;
  key: string;
  nodeId: string;
  boundAt: number;
  expiresAt?: number;
  remainingSeconds?: number;
}

/**
 * 获取所有绑定信息
 */
export async function sessionAffinityGetBindings(): Promise<BindingInfo[]> {
  return await invoke<BindingInfo[]>('session_affinity_get_bindings');
}

/**
 * 清除域名绑定
 */
export async function sessionAffinityClearBinding(domain: string): Promise<void> {
  await invoke('session_affinity_clear_binding', { domain });
}

/**
 * 获取预定义规则
 */
export async function sessionAffinityGetPredefinedRules(): Promise<DomainBindingRule[]> {
  return await invoke<DomainBindingRule[]>('session_affinity_get_predefined_rules');
}

/**
 * 清理过期绑定
 */
export async function sessionAffinityCleanupExpired(): Promise<void> {
  await invoke('session_affinity_cleanup_expired');
}


/**
 * 为域名选择节点
 */
export async function sessionAffinitySelectNodeForDomain(
  domain: string,
  availableNodes: string[]
): Promise<string> {
  return await invoke<string>('session_affinity_select_node_for_domain', {
    domain,
    availableNodes,
  });
}

/**
 * 为进程选择节点
 */
export async function sessionAffinitySelectNodeForProcess(
  sourcePort: number,
  availableNodes: string[]
): Promise<string> {
  return await invoke<string>('session_affinity_select_node_for_process', {
    sourcePort,
    availableNodes,
  });
}

/**
 * 为连接选择节点
 */
export async function sessionAffinitySelectNodeForConnection(
  sourceIp: string,
  sourcePort: number,
  availableNodes: string[]
): Promise<string> {
  return await invoke<string>('session_affinity_select_node_for_connection', {
    sourceIp,
    sourcePort,
    availableNodes,
  });
}
