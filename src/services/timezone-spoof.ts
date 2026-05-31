/**
 * 时区/NTP 伪装服务
 *
 * 提供时区伪装配置的 TS 类型和 Tauri 命令绑定
 */

import { invoke } from '@tauri-apps/api/core'

// ── 类型 ──────────────────────────────────────────────────────────

/** NTP 服务器选择策略 */
export type NtpStrategy = 'Auto' | 'Manual' | 'Disabled'

/** 时区/NTP 伪装配置 */
export interface TimezoneSpoofConfig {
  enabled: boolean
  ntp_strategy: NtpStrategy
  manual_ntp_server?: string
  ntp_interval_min: number
  write_to_system: boolean
  dialer_proxy?: string
}

// ── 命令 ──────────────────────────────────────────────────────────

/** 获取时区伪装配置 */
export async function timezoneSpoofGetConfig(): Promise<TimezoneSpoofConfig> {
  return invoke('timezone_spoof_get_config')
}

/** 更新时区伪装配置 */
export async function timezoneSpoofUpdateConfig(
  config: TimezoneSpoofConfig
): Promise<void> {
  return invoke('timezone_spoof_update_config', { config })
}

/** 根据国家代码获取推荐的 NTP 服务器 */
export async function timezoneSpoofGetNtpServer(
  countryCode: string
): Promise<string> {
  return invoke('timezone_spoof_get_ntp_server', { countryCode })
}

/** 根据国家代码获取时区 */
export async function timezoneSpoofGetTimezone(
  countryCode: string
): Promise<string> {
  return invoke('timezone_spoof_get_timezone', { countryCode })
}

/** 根据时区获取 locale */
export async function timezoneSpoofGetLocale(
  timezone: string
): Promise<string> {
  return invoke('timezone_spoof_get_locale', { timezone })
}
