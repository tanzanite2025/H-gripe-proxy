/**
 * 本地安全服务
 * 
 * 提供本地安全监控和防火墙管理功能
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * 本地安全配置
 */
export interface LocalSecurityConfig {
  /** 绑定地址（强制 127.0.0.1） */
  bindAddress: string;
  /** 端口随机化 */
  portRandomization: boolean;
  /** 端口范围 */
  portRange: [number, number];
  /** 端口冲突自动切换 */
  autoSwitchOnConflict: boolean;
  /** 防火墙自动配置 */
  autoFirewall: boolean;
  /** 进程隐蔽 */
  processStealth: boolean;
  /** 泄漏监控 */
  leakMonitoring: boolean;
  /** 监控间隔（秒） */
  monitorInterval: number;
}

/**
 * 泄漏监控状态
 */
export interface LeakMonitorStatus {
  /** 本地绑定安全 */
  localBindingSecure: boolean;
  /** 防火墙规则生效 */
  firewallRulesActive: boolean;
  /** 进程隐蔽 */
  processHidden: boolean;
  /** 外部访问被阻止 */
  externalAccessBlocked: boolean;
  /** 最后检查时间（Unix 时间戳） */
  lastCheckTime: number;
  /** 是否检测到泄漏 */
  leakDetected: boolean;
  /** 泄漏类型 */
  leakType?: string;
  /** 是否自动修复 */
  autoFixApplied: boolean;
}

/**
 * 获取本地安全配置
 */
export async function getLocalSecurityConfig(): Promise<LocalSecurityConfig> {
  return await invoke<LocalSecurityConfig>('local_security_get_config');
}

/**
 * 更新本地安全配置
 */
export async function updateLocalSecurityConfig(config: LocalSecurityConfig): Promise<void> {
  await invoke('local_security_update_config', { config });
}

/**
 * 获取泄漏监控状态
 */
export async function getLocalSecurityStatus(): Promise<LeakMonitorStatus> {
  return await invoke<LeakMonitorStatus>('local_security_get_status');
}

/**
 * 立即执行安全检查
 */
export async function checkSecurityNow(port: number): Promise<LeakMonitorStatus> {
  return await invoke<LeakMonitorStatus>('local_security_check_now', { port });
}

/**
 * 检查本地绑定是否安全
 */
export async function checkLocalBinding(port: number): Promise<boolean> {
  return await invoke<boolean>('local_security_check_binding', { port });
}

/**
 * 检查端口冲突
 */
export async function checkPortConflict(port: number): Promise<boolean> {
  return await invoke<boolean>('local_security_check_port_conflict', { port });
}

/**
 * 查找可用端口
 */
export async function findAvailablePort(): Promise<number> {
  return await invoke<number>('local_security_find_available_port');
}

/**
 * 配置防火墙规则
 */
export async function configureFirewall(port: number): Promise<void> {
  await invoke('local_security_configure_firewall', { port });
}

/**
 * 删除防火墙规则
 */
export async function removeFirewall(port: number): Promise<void> {
  await invoke('local_security_remove_firewall', { port });
}

/**
 * 启动泄漏监控循环
 */
export async function startLeakMonitor(port: number): Promise<void> {
  await invoke('leak_monitor_start', { port });
}

/**
 * 停止泄漏监控循环
 */
export async function stopLeakMonitor(): Promise<void> {
  await invoke('leak_monitor_stop');
}

/**
 * 检查泄漏监控是否正在运行
 */
export async function isLeakMonitorRunning(): Promise<boolean> {
  return await invoke<boolean>('leak_monitor_is_running');
}

/**
 * 更新泄漏监控端口
 */
export async function setLeakMonitorPort(port: number): Promise<void> {
  await invoke('leak_monitor_set_port', { port });
}

/**
 * 获取泄漏监控端口
 */
export async function getLeakMonitorPort(): Promise<number> {
  return await invoke<number>('leak_monitor_get_port');
}
