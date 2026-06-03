import { invoke } from '@tauri-apps/api/core'
import type { ClashMode } from './clash-mode'
import dayjs from 'dayjs'

import { showNotice } from '@/services/notice-service'
import { normalizeIpReputation, type IpReputation } from '@/services/ip-reputation'
import { debugLog } from '@/utils/misc'

export async function copyClashEnv() {
  return invoke<void>('copy_clash_env')
}

export async function getProfiles() {
  return invoke<IProfilesConfig>('get_profiles')
}

export async function enhanceProfiles() {
  return (
    (await invoke<ValidationOutcome>('enhance_profiles')).status === 'valid'
  )
}

export async function patchProfilesConfig(profiles: IProfilesConfig) {
  return (
    (await invoke<ValidationOutcome>('patch_profiles_config', { profiles }))
      .status === 'valid'
  )
}

export async function createProfile(
  item: Partial<IProfileItem>,
  fileData?: string | null,
) {
  return invoke<void>('create_profile', { item, fileData })
}

export async function createProfileFromLocalPath(
  item: Partial<IProfileItem>,
  path: string,
) {
  return invoke<void>('create_profile_from_local_path', { item, path })
}

export async function viewProfile(index: string) {
  return invoke<void>('view_profile', { index })
}

export async function readProfileFile(index: string) {
  return invoke<string>('read_profile_file', { index })
}

export async function saveProfileFile(index: string, fileData: string) {
  return (
    (
      await invoke<ValidationOutcome>('save_profile_file', {
        index,
        fileData,
      })
    ).status === 'valid'
  )
}

export async function importProfile(url: string, option?: IProfileOption) {
  return invoke<void>('import_profile', {
    url,
    option: option || { with_proxy: true },
  })
}

export async function reorderProfile(activeId: string, overId: string) {
  return invoke<void>('reorder_profile', {
    activeId,
    overId,
  })
}

export async function updateProfile(index: string, option?: IProfileOption) {
  return invoke<void>('update_profile', { index, option })
}

export async function deleteProfile(index: string) {
  return invoke<void>('delete_profile', { index })
}

export async function patchProfile(
  index: string,
  profile: Partial<IProfileItem>,
) {
  return invoke<void>('patch_profile', { index, profile })
}

export async function getClashInfo() {
  return invoke<IClashInfo | null>('get_clash_info')
}

// Get runtime config which controlled by verge
export async function getRuntimeConfig() {
  return invoke<IConfigData | null>('get_runtime_config')
}

export interface GeoDataUpdateTime {
  mmdb: number | null
  geoip: number | null
  asn: number | null
  geosite: number | null
}

export async function getGeoDataUpdateTime() {
  return invoke<GeoDataUpdateTime>('get_geo_data_update_time')
}

export async function getRuntimeYaml() {
  return invoke<string | null>('get_runtime_yaml')
}

export interface DnsRuntimeSnapshot {
  enhanced_mode: string | null
  ipv6: boolean | null
  nameserver_count: number
  fallback_count: number
  nameserver_policy_count: number
  use_hosts: boolean | null
  use_system_hosts: boolean | null
  respect_rules: boolean | null
}

export interface DnsRuntimeDerivedState {
  routing_mode: string | null
  domestic_dns: string[]
  foreign_dns: string[]
  default_nameserver_count: number
  default_nameserver_plain_count: number
  prefer_h3: boolean | null
  leak_protection_level: string | null
  leak_protection_security: string | null
  leak_protection_safe: boolean | null
}

export interface DnsRuntimeStatus {
  enable_dns_settings: boolean
  dns_config_exists: boolean
  dns_config_valid: boolean
  runtime_has_dns: boolean
  runtime_has_hosts: boolean
  runtime_dns_matches_saved: boolean
  runtime_hosts_matches_saved: boolean
  runtime_matches_saved: boolean
  snapshot: DnsRuntimeSnapshot
  derived: DnsRuntimeDerivedState
}

export async function getDnsRuntimeStatus() {
  return invoke<DnsRuntimeStatus>('get_dns_runtime_status')
}

export interface DnsLeakServer {
  ip: string
  hostname: string | null
  country: string | null
  city: string | null
  isp: string | null
}

export interface DnsMetrics {
  cache: {
    hit: number
    miss: number
    size: number
    hitRate: number
  }
  queries: {
    total: number
    success: number
    failed: number
    avgLatencyUs: number
    maxLatencyUs: number
  }
  servers: Array<{
    server: string
    queries: number
    successes: number
    failures: number
    avgLatencyUs: number
    lastQuery: string
    lastError?: string | null
  }>
  recent: Array<{
    domain: string
    qType: string
    server: string
    protocol: string
    proxyName?: string | null
    proxyChain?: string | null
    egress?: string | null
    rule?: string | null
    rulePayload?: string | null
    success: boolean
    error?: string | null
    latencyUs: number
    timestamp: string
  }>
  pollution: {
    totalChecked: number
    pollutedCount: number
    pollutionRate: number
    recentPolluted: Array<{
      domain: string
      ip: string
      reason: string
      timestamp: string
    }>
  }
  trust: {
    total: number
    encrypted: number
    unencrypted: number
    byTrustLevel: Record<string, number>
    servers: Array<{
      address: string
      protocol: string
      trustLevel: string
      encrypted: boolean
      description?: string | null
    }>
    leakRiskScore: number
    lastEvaluated: string
  }
}

export interface DnsLeakTestResult {
  has_leak: boolean
  observed_leak: boolean
  runtime_risk_detected: boolean
  observation_incomplete: boolean
  confidence: 'high' | 'medium' | 'low' | string
  assessment: 'safe' | 'observed-leak' | 'runtime-risk' | 'inconclusive' | string
  leak_type: string[]
  observed_leak_type: string[]
  runtime_risk_type: string[]
  warnings: string[]
  recommendations: string[]
  dns_servers: DnsLeakServer[]
  dns_metrics: DnsMetrics | null
  dns_location: string | null
  ip_location: string
  location_match: boolean
  location_comparable: boolean
  risk_level: 'safe' | 'warning' | 'danger' | string
  timestamp: number
  checked_via_core_proxy: boolean
  observation_path: 'core-proxy' | 'core-proxy-fallback-direct' | 'direct' | string
  error: string | null
}

export async function testDnsLeak() {
  return invoke<DnsLeakTestResult>('test_dns_leak')
}

export interface ProxyDetectionLocation {
  country_code: string | null
  country: string | null
  region: string | null
  city: string | null
  organization: string | null
  asn: number | null
  asn_organization: string | null
}

export interface ProxyDetectionResult {
  checked: boolean
  core_running: boolean
  direct_observed: boolean
  proxy_observed: boolean
  checked_via_core_proxy: boolean
  proxy_effective: boolean
  ip_changed: boolean
  location_changed: boolean
  observation_incomplete: boolean
  runtime_risk_detected: boolean
  confidence: 'high' | 'medium' | 'low' | string
  assessment: 'effective' | 'same-egress' | 'runtime-risk' | 'inconclusive' | string
  runtime_risk_type: string[]
  warnings: string[]
  recommendations: string[]
  direct_ip: string | null
  proxy_ip: string | null
  direct_location: ProxyDetectionLocation | null
  proxy_location: ProxyDetectionLocation | null
  proxy_reputation: IpReputation | null
  observation_path: 'direct-vs-core-proxy' | 'direct-only' | 'core-proxy-only' | string
  error: string | null
  timestamp: number
}

export async function testProxyDetection() {
  const result = await invoke<ProxyDetectionResult & { proxy_reputation?: unknown }>(
    'test_proxy_detection',
  )

  return {
    ...result,
    proxy_reputation: result.proxy_reputation
      ? normalizeIpReputation(result.proxy_reputation)
      : null,
  }
}

export type CurrentEgressIdentitySource =
  | 'mihomoEgressStatus'
  | 'mihomoConnectionMetadata'
  | 'publicIpObservation'
  | 'unavailable'

export interface CurrentEgressIdentity {
  source: CurrentEgressIdentitySource
  proxy_name: string | null
  proxy_chain: string[]
  egress_ip: string | null
  public_egress_ip: string | null
  proxy_endpoint: string | null
  destination_asn: string | null
  asn_org: string | null
  rule: string | null
  rule_payload: string | null
  egress_source: string | null
  confidence: number | null
  sample_count: number | null
  last_verified_at: string | null
  updated_at: string | null
  reputation: IpReputation | null
  message: string
}

export async function getCurrentEgressIdentity(): Promise<CurrentEgressIdentity> {
  const result = await invoke<CurrentEgressIdentity & { reputation?: unknown }>(
    'get_current_egress_identity',
  )

  return {
    ...result,
    proxy_chain: Array.isArray(result.proxy_chain) ? result.proxy_chain : [],
    reputation: result.reputation ? normalizeIpReputation(result.reputation) : null,
  }
}

export interface CurrentPublicIpInfo {
  ip: string
  country_code: string | null
  country: string | null
  region: string | null
  city: string | null
  organization: string | null
  asn: number | null
  asn_organization: string | null
}

export async function getCurrentPublicIpInfo(): Promise<CurrentPublicIpInfo> {
  return invoke<CurrentPublicIpInfo>('get_current_public_ip_info')
}

export type IdentityConsistencyLevel = 'good' | 'warning' | 'danger' | 'unknown'

export type IdentityConsistencyIssueKind =
  | 'missingPublicEgress'
  | 'lowEgressConfidence'
  | 'highIpRisk'
  | 'dnsLeak'
  | 'dnsRuntimeRisk'
  | 'randomTlsFingerprint'
  | 'missingTlsFingerprint'
  | 'observationIncomplete'

export interface IdentityConsistencyIssue {
  kind: IdentityConsistencyIssueKind
  severity: IdentityConsistencyLevel
  message: string
}

export interface IdentityConsistencyReport {
  score: number
  level: IdentityConsistencyLevel
  issues: IdentityConsistencyIssue[]
  public_egress_ip: string | null
  proxy_chain: string[]
  ip_type: string | null
  residential_state: string | null
  egress_source: string | null
  egress_confidence: number | null
  tls_fingerprint: string | null
  dns_assessment: string | null
}

export interface IdentityConsistencySnapshot {
  observed_at: string
  report: IdentityConsistencyReport
}

export type IdentityConsistencyDriftKind =
  | 'publicEgressIp'
  | 'ipType'
  | 'dnsAssessment'
  | 'tlsFingerprint'

export interface IdentityConsistencyDrift {
  kind: IdentityConsistencyDriftKind
  from: string | null
  to: string | null
  first_observed_at: string
  last_observed_at: string
}

export interface IdentityConsistencyDriftReport {
  stable: boolean
  drift_count: number
  drifts: IdentityConsistencyDrift[]
}

export async function getIdentityConsistencyReport(): Promise<IdentityConsistencyReport> {
  const result = await invoke<IdentityConsistencyReport>(
    'get_identity_consistency_report',
  )

  return {
    ...result,
    proxy_chain: Array.isArray(result.proxy_chain) ? result.proxy_chain : [],
    issues: Array.isArray(result.issues) ? result.issues : [],
  }
}

export async function getIdentityConsistencyHistory(): Promise<IdentityConsistencySnapshot[]> {
  const result = await invoke<IdentityConsistencySnapshot[]>(
    'get_identity_consistency_history',
  )

  return Array.isArray(result)
    ? result.map((snapshot) => ({
        ...snapshot,
        report: {
          ...snapshot.report,
          proxy_chain: Array.isArray(snapshot.report.proxy_chain)
            ? snapshot.report.proxy_chain
            : [],
          issues: Array.isArray(snapshot.report.issues)
            ? snapshot.report.issues
            : [],
        },
      }))
    : []
}

export async function getIdentityConsistencyDriftReport(): Promise<IdentityConsistencyDriftReport> {
  const result = await invoke<IdentityConsistencyDriftReport>(
    'get_identity_consistency_drift_report',
  )

  return {
    ...result,
    drift_count: Number.isFinite(result.drift_count) ? result.drift_count : 0,
    drifts: Array.isArray(result.drifts) ? result.drifts : [],
  }
}

export interface TorRuntimeStatus {
  enabled: boolean
  socks_host: string
  socks_port: number
  control_port: number | null
  use_bridges: boolean
  bridge_count: number
  configured_proxy_url: string
  checked: boolean
  status: 'disabled' | 'connected' | 'failed' | string
  connected: boolean
  circuit_established: boolean
  observation_incomplete: boolean
  runtime_risk_detected: boolean
  confidence: 'high' | 'medium' | 'low' | string
  assessment: 'disabled' | 'connected' | 'runtime-risk' | 'inconclusive' | string
  runtime_risk_type: string[]
  current_ip: string | null
  exit_node: string | null
  check_method: string
  observation_path: string
  observation_source: string | null
  warnings: string[]
  error: string | null
  timestamp: number
}

export async function getTorStatus() {
  return invoke<TorRuntimeStatus>('get_tor_status')
}

export async function testTorConnection() {
  return invoke<TorRuntimeStatus>('test_tor_connection')
}

export async function getRuntimeExists() {
  return invoke<string[]>('get_runtime_exists')
}

export async function getRuntimeLogs() {
  return invoke<Record<string, [string, string][]>>('get_runtime_logs')
}

export async function getRuntimeProxyChainConfig(proxyChainExitNode: string) {
  return invoke<string>('get_runtime_proxy_chain_config', {
    proxyChainExitNode,
  })
}

export async function updateProxyChainConfigInRuntime(
  proxyChainConfig: string[] | null,
) {
  return invoke<void>('update_proxy_chain_config_in_runtime', {
    proxyChainConfig,
  })
}

export async function patchClashConfig(payload: Partial<IConfigData>) {
  return invoke<void>('patch_clash_config', { payload })
}

export async function patchClashMode(payload: ClashMode) {
  return invoke<void>('patch_clash_mode', { payload })
}

export async function syncTrayProxySelection() {
  return invoke<void>('sync_tray_proxy_selection')
}


export async function getClashLogs() {
  const regex = /time="(.+?)"\s+level=(.+?)\s+msg="(.+?)"/
  const newRegex = /(.+?)\s+(.+?)\s+(.+)/
  const logs = await invoke<string[]>('get_clash_logs')

  return logs.reduce<ILogItem[]>((acc, log) => {
    const result = log.match(regex)
    if (result) {
      const [_, _time, type, payload] = result
      const time = dayjs(_time).format('MM-DD HH:mm:ss')
      acc.push({ time, type, payload })
      return acc
    }

    const result2 = log.match(newRegex)
    if (result2) {
      const [_, time, type, payload] = result2
      acc.push({ time, type, payload })
    }
    return acc
  }, [])
}

export async function clearLogs() {
  return invoke<void>('clear_logs')
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

export async function applyDnsConfig(apply: boolean) {
  return invoke<void>('apply_dns_config', { apply })
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
    console.error('获取自启动状态失败:', error)
    return false
  }
}

export async function changeClashCore(clashCore: string) {
  return invoke<string | null>('change_clash_core', { clashCore })
}

export async function startCore() {
  return invoke<void>('start_core')
}

export async function stopCore() {
  return invoke<void>('stop_core')
}

export async function restartCore() {
  return invoke<void>('restart_core')
}

export async function restartApp() {
  return invoke<void>('restart_app')
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

export async function cmdGetProxyDelay(
  name: string,
  timeout: number,
  url?: string,
) {
  // 确保URL不为空
  const testUrl = url || 'http://cp.cloudflare.com/generate_204'

  try {
    const result = await invoke<{ delay: number }>(
      'plugin:mihomo|delay_proxy_by_name',
      {
        proxyName: name,
        testUrl,
        timeout,
      },
    )

    // 验证返回结果中是否有delay字段，并且值是一个有效的数字
    if (result && typeof result.delay === 'number') {
      return result
    } else {
      // 返回一个有效的结果对象，但标记为超时
      return { delay: 1e6 }
    }
  } catch {
    // 返回一个有效的结果对象，但标记为错误
    return { delay: 1e6 }
  }
}

export async function cmdTestDelay(url: string) {
  return invoke<number>('test_delay', { url })
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

export async function createWebdavBackup() {
  return invoke<void>('create_webdav_backup')
}

export async function createLocalBackup() {
  return invoke<void>('create_local_backup')
}

export async function deleteWebdavBackup(filename: string) {
  return invoke<void>('delete_webdav_backup', { filename })
}

export async function deleteLocalBackup(filename: string) {
  return invoke<void>('delete_local_backup', { filename })
}

export async function restoreWebDavBackup(filename: string) {
  return invoke<void>('restore_webdav_backup', { filename })
}

export async function restoreLocalBackup(filename: string) {
  return invoke<void>('restore_local_backup', { filename })
}

export async function importLocalBackup(source: string) {
  return invoke<string>('import_local_backup', { source })
}

export async function exportLocalBackup(filename: string, destination: string) {
  return invoke<void>('export_local_backup', { filename, destination })
}

export async function saveWebdavConfig(
  url: string,
  username: string,
  password: string,
) {
  return invoke<void>('save_webdav_config', {
    url,
    username,
    password,
  })
}

export async function listWebDavBackup() {
  const list: IWebDavFile[] = await invoke<IWebDavFile[]>('list_webdav_backup')
  list.map((item) => {
    item.filename = item.href.split('/').pop() as string
  })
  return list
}

export async function listLocalBackup() {
  return invoke<ILocalBackupFile[]>('list_local_backup')
}

export async function scriptValidateNotice(status: string, msg: string) {
  return invoke<void>('script_validate_notice', { status, msg })
}

export async function validateScriptFile(filePath: string) {
  return invoke<ValidationOutcome>('validate_script_file', { filePath })
}

// 获取当前运行模式
export const getRunningMode = async () => {
  return invoke<string>('get_running_mode')
}

// 获取应用运行时间
export const getAppUptime = async () => {
  return invoke<number>('get_app_uptime')
}

// 安装系统服务
export const installService = async () => {
  return invoke<void>('install_service')
}

// 卸载系统服务
export const uninstallService = async () => {
  return invoke<void>('uninstall_service')
}

// 重装系统服务
export const reinstallService = async () => {
  return invoke<void>('reinstall_service')
}

// 修复系统服务
export const repairService = async () => {
  return invoke<void>('repair_service')
}

// 系统服务是否可用
export const isServiceAvailable = async () => {
  try {
    return await invoke<boolean>('is_service_available')
  } catch (error) {
    console.error('Service check failed:', error)
    return false
  }
}
export const entry_lightweight_mode = async () => {
  return invoke<void>('entry_lightweight_mode')
}

export const exit_lightweight_mode = async () => {
  return invoke<void>('exit_lightweight_mode')
}

export const isAdmin = async () => {
  try {
    return await invoke<boolean>('app_is_admin')
  } catch (error) {
    console.error('检查管理员权限失败:', error)
    return false
  }
}

export async function getNextUpdateTime(uid: string) {
  return invoke<number | null>('get_next_update_time', { uid })
}

export const isPortInUse = async (port: number) => {
  try {
    return await invoke<boolean>('is_port_in_use', { port })
  } catch (error) {
    console.error('检查端口使用状态失败:', error)
    return false
  }
}

// Security Policy commands
export async function securityPolicyGetPolicies() {
  return invoke<ISecurityPolicy[]>('security_policy_get_policies')
}

export async function securityPolicyGet(name: string) {
  return invoke<ISecurityPolicy | null>('security_policy_get', { name })
}

export async function securityPolicyUpsert(policy: ISecurityPolicy) {
  return invoke<void>('security_policy_upsert', { policy })
}

export async function securityPolicyRemove(name: string) {
  return invoke<void>('security_policy_remove', { name })
}

export async function securityPolicyApply(name: string) {
  return invoke<number[]>('security_policy_apply', { name })
}

export async function securityPolicyRevoke(name: string) {
  return invoke<void>('security_policy_revoke', { name })
}

export async function securityPolicyApplyAll() {
  return invoke<string[]>('security_policy_apply_all')
}

export async function securityPolicyRevokeAll() {
  return invoke<string[]>('security_policy_revoke_all')
}

export async function securityPolicyGetStates() {
  return invoke<IAppliedPolicyState[]>('security_policy_get_states')
}

export async function securityPolicyGetState(name: string) {
  return invoke<IAppliedPolicyState | null>('security_policy_get_state', { name })
}

export async function securityPolicyReload(policies: ISecurityPolicy[]) {
  return invoke<string[]>('security_policy_reload', { policies })
}
