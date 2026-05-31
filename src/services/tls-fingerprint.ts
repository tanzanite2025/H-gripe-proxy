/**
 * TLS 指纹伪装服务
 */

import { invoke } from '@tauri-apps/api/core'

export interface TlsFingerprint {
  name: string
  description: string
  category: string
}

/**
 * 获取所有预定义指纹
 */
export async function tlsFingerprintGetAll(): Promise<TlsFingerprint[]> {
  return invoke<TlsFingerprint[]>('tls_fingerprint_get_all')
}

/**
 * 根据名称获取指纹
 */
export async function tlsFingerprintGetByName(
  name: string,
): Promise<TlsFingerprint | null> {
  return invoke<TlsFingerprint | null>('tls_fingerprint_get_by_name', { name })
}

/**
 * 获取当前指纹
 */
export async function tlsFingerprintGetCurrent(): Promise<TlsFingerprint | null> {
  return invoke<TlsFingerprint | null>('tls_fingerprint_get_current')
}

/**
 * 生成 Clash 配置
 */
export async function tlsFingerprintGenerateConfig(): Promise<any | null> {
  return invoke<any | null>('tls_fingerprint_generate_config')
}

/**
 * 清除当前指纹
 */
export async function tlsFingerprintClear(): Promise<void> {
  return invoke<void>('tls_fingerprint_clear')
}
