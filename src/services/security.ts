/**
 * 安全防御服务
 */

import { invoke } from '@tauri-apps/api/core'

export interface SecurityStatus {
  compromised: boolean
  debugger_present: boolean
  memory_scanning: boolean
}

/**
 * 启动安全监控
 */
export async function securityStartMonitor(): Promise<void> {
  return invoke<void>('security_start_monitor')
}

/**
 * 停止安全监控
 */
export async function securityStopMonitor(): Promise<void> {
  return invoke<void>('security_stop_monitor')
}

/**
 * 检查安全状态
 */
export async function securityCheckStatus(): Promise<SecurityStatus> {
  return invoke<SecurityStatus>('security_check_status')
}

/**
 * 部署假配置文件
 */
export async function securityDeployDecoy(decoyPath: string): Promise<void> {
  return invoke<void>('security_deploy_decoy', { decoyPath })
}

/**
 * 清除假配置文件
 */
export async function securityCleanupDecoy(decoyPath: string): Promise<void> {
  return invoke<void>('security_cleanup_decoy', { decoyPath })
}

/**
 * 检查假配置是否被访问
 */
export async function securityCheckDecoyAccess(
  decoyPath: string,
): Promise<boolean> {
  return invoke<boolean>('security_check_decoy_access', { decoyPath })
}

/**
 * 生成加密密钥
 */
export async function securityGenerateEncryptionKey(): Promise<string> {
  return invoke<string>('security_generate_encryption_key')
}

/**
 * 加密数据
 */
export async function securityEncryptData(data: Uint8Array): Promise<Uint8Array> {
  return invoke<Uint8Array>('security_encrypt_data', { data: Array.from(data) })
}

/**
 * 解密数据
 */
export async function securityDecryptData(data: Uint8Array): Promise<Uint8Array> {
  return invoke<Uint8Array>('security_decrypt_data', { data: Array.from(data) })
}

/**
 * 检查加密密钥是否可用
 */
export async function securityCheckEncryptionKey(): Promise<boolean> {
  return invoke<boolean>('security_check_encryption_key')
}

/**
 * 触发自毁（需要确认）
 */
export async function securitySelfDestruct(
  confirmation: string,
): Promise<void> {
  return invoke<void>('security_self_destruct', { confirmation })
}
