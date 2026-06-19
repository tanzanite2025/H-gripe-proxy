import { invoke } from '@tauri-apps/api/core'

import { showNotice } from '@/services/notice-service'
import { debugLog } from '@/utils/misc'

export async function copyClashEnv() {
  return invoke<void>('copy_clash_env')
}

export async function getVergeConfig() {
  return invoke<IVergeConfig>('get_verge_config')
}

export async function patchVergeConfig(payload: IVergeConfig) {
  return invoke<void>('patch_verge_config', { payload })
}

export async function authorizeStartupScript(path: string) {
  return invoke<void>('authorize_startup_script', { path })
}

export async function clearStartupScriptAuthorization() {
  return invoke<void>('clear_startup_script_authorization')
}

export async function getSystemProxy() {
  return invoke<{
    enable: boolean
    server: string
    bypass: string
  }>('get_sys_proxy')
}

export async function getAutotemProxy() {
  try {
    debugLog('[API] 开始调用 get_auto_proxy')
    const result = await invoke<{
      enable: boolean
      url: string
    }>('get_auto_proxy')
    debugLog('[API] get_auto_proxy 调用成功:', result)
    return result
  } catch (error) {
    console.error('[API] get_auto_proxy 调用失败:', error)
    return {
      enable: false,
      url: '',
    }
  }
}

export async function getAutoLaunchStatus() {
  try {
    return await invoke<boolean>('get_auto_launch_status')
  } catch (error) {
    console.error('获取自启动状态失败', error)
    return false
  }
}

export async function startCore() {
  return invoke<void>('start_core')
}

export async function stopCore() {
  return invoke<void>('stop_core')
}

export async function restartCore() {
  return invoke<void>('restart_runtime_core')
}

export async function restartApp() {
  return invoke<void>('restart_runtime_app')
}

export async function openAppDir() {
  return invoke<void>('open_app_dir').catch((err) => showNotice.error(err))
}

export async function openCoreDir() {
  return invoke<void>('open_core_dir').catch((err) => showNotice.error(err))
}

export async function openLogsDir() {
  return invoke<void>('open_logs_dir').catch((err) => showNotice.error(err))
}

export const openWebUrl = async (url: string) => {
  try {
    await invoke('open_web_url', { url })
  } catch (err: any) {
    showNotice.error(err)
  }
}

export async function invoke_uwp_tool() {
  return invoke<void>('invoke_uwp_tool').catch((err) =>
    showNotice.error(err, 1500),
  )
}

export async function getPortableFlag() {
  return invoke<boolean>('get_portable_flag')
}

export async function openDevTools() {
  if (!import.meta.env.DEV) {
    throw new Error('DevTools are only available in development builds')
  }

  return invoke('open_devtools')
}

export async function exitApp() {
  return invoke('exit_app')
}

export async function exportDiagnosticInfo() {
  return invoke('export_diagnostic_info')
}

export async function getSystemInfo() {
  return invoke<string>('get_system_info')
}

export async function downloadIconCache(url: string, name: string) {
  return invoke<string>('download_icon_cache', { url, name })
}

export async function getNetworkInterfaces() {
  return invoke<string[]>('get_network_interfaces')
}

export async function getSystemHostname() {
  return invoke<string>('get_system_hostname')
}

export async function getNetworkInterfacesInfo() {
  return invoke<INetworkInterface[]>('get_network_interfaces_info')
}

export const getRunningMode = async () => {
  return invoke<string>('get_running_mode')
}

export const getAppUptime = async () => {
  return invoke<number>('get_app_uptime')
}

export const installService = async () => {
  return invoke<void>('install_service')
}

export const uninstallService = async () => {
  return invoke<void>('uninstall_service')
}

export const reinstallService = async () => {
  return invoke<void>('reinstall_service')
}

export const repairService = async () => {
  return invoke<void>('repair_service')
}

export const isServiceAvailable = async () => {
  try {
    return await invoke<boolean>('is_service_available')
  } catch (error) {
    console.error('Service check failed:', error)
    return false
  }
}

export const isAdmin = async () => {
  try {
    return await invoke<boolean>('app_is_admin')
  } catch (error) {
    console.error('检查管理员权限失败:', error)
    return false
  }
}

export const isPortInUse = async (port: number) => {
  try {
    return await invoke<boolean>('is_port_in_use', { port })
  } catch (error) {
    console.error('检查端口使用状态失败:', error)
    return false
  }
}
