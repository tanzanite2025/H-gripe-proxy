/**
 * 反主动探测服务
 */

import { invoke } from '@tauri-apps/api/core'

export interface AntiProbeConfig {
  enabled: boolean
  secret_key: string
  time_window: number
  whitelist: string[]
  strict_mode: boolean
}

/**
 * 验证握手暗号
 */
export async function antiProbeVerifyHandshake(
  clientIp: string,
  token: string,
): Promise<boolean> {
  return invoke<boolean>('anti_probe_verify_handshake', {
    clientIp,
    token,
  })
}

/**
 * 生成握手暗号
 */
export async function antiProbeGenerateToken(): Promise<string> {
  return invoke<string>('anti_probe_generate_token')
}

/**
 * 更新配置
 */
export async function antiProbeUpdateConfig(
  config: AntiProbeConfig,
): Promise<void> {
  return invoke<void>('anti_probe_update_config', { config })
}

/**
 * 获取配置
 */
export async function antiProbeGetConfig(): Promise<AntiProbeConfig> {
  return invoke<AntiProbeConfig>('anti_probe_get_config')
}

/**
 * 清理过期缓存
 */
export async function antiProbeCleanup(): Promise<void> {
  return invoke<void>('anti_probe_cleanup')
}
