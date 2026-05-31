/**
 * 本地隐蔽增强服务
 *
 * 功能：
 * 1. 进程隐蔽 - 伪装进程名
 * 2. 端口隐蔽 - 端口随机化
 * 3. 防本地发现 - 禁用 mDNS/UPnP
 */

import { invoke } from '@tauri-apps/api/core'

// ── 类型定义 ──

/** 进程隐蔽配置 */
export interface ProcessStealthConfig {
  /** 是否启用 */
  enabled: boolean
  /** 伪装的进程标题名 */
  disguise_title: string
}

/** 端口隐蔽配置 */
export interface PortStealthConfig {
  /** 是否启用端口随机化 */
  enabled: boolean
  /** 端口范围 */
  port_range: [number, number]
  /** 避免使用的常见端口列表 */
  avoid_ports: number[]
}

/** 防本地发现配置 */
export interface AntiDiscoveryConfig {
  /** 是否启用 */
  enabled: boolean
  /** 禁用 mDNS */
  disable_mdns: boolean
  /** 禁用 UPnP */
  disable_upnp: boolean
  /** 禁用 LLMNR */
  disable_llmnr: boolean
  /** 禁用 NetBIOS */
  disable_netbios: boolean
  /** 禁用 SSDP */
  disable_ssdp: boolean
}

/** 本地隐蔽总配置 */
export interface LocalStealthConfig {
  process_stealth: ProcessStealthConfig
  port_stealth: PortStealthConfig
  anti_discovery: AntiDiscoveryConfig
}

/** 隐蔽策略应用结果 */
export interface StealthApplyResult {
  process_stealth_applied: boolean
  port_stealth_applied: boolean
  allocated_port: number | null
  anti_discovery_applied: boolean
  discovery_messages: string[]
  errors: string[]
}

// ── API 调用 ──

/** 获取本地隐蔽配置 */
export async function getLocalStealthConfig(): Promise<LocalStealthConfig> {
  return await invoke<LocalStealthConfig>('local_stealth_get_config')
}

/** 更新本地隐蔽配置 */
export async function updateLocalStealthConfig(config: LocalStealthConfig): Promise<void> {
  await invoke('local_stealth_update_config', { config })
}

/** 应用所有隐蔽策略 */
export async function applyLocalStealth(): Promise<StealthApplyResult> {
  return await invoke<StealthApplyResult>('local_stealth_apply')
}

/** 恢复所有隐蔽策略 */
export async function restoreLocalStealth(): Promise<void> {
  await invoke('local_stealth_restore')
}

/** 分配随机隐蔽端口 */
export async function allocateStealthPort(): Promise<number> {
  return await invoke<number>('local_stealth_allocate_port')
}

/** 获取当前分配的隐蔽端口 */
export async function getCurrentStealthPort(): Promise<number | null> {
  return await invoke<number | null>('local_stealth_get_port')
}
