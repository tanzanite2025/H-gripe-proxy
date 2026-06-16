import type {
  AppPolicyBinding,
  AppRegistryEntry,
  AppRuntimeSessionRecord,
  AppRuntimeSessionStatus,
  AppRuntimeStateDocument,
  DnsProfile,
  NodePool,
  SecurityProfile,
} from '@/services/app-runtime'

export const emptyState: AppRuntimeStateDocument = {
  apps: [],
  nodePools: [],
  dnsProfiles: [],
  securityProfiles: [],
  policyBindings: [],
  sessions: [],
}

export function stateCountLabel(label: string, count: number) {
  return `${label}: ${count}`
}

export function selectAppLabel(app: AppRegistryEntry) {
  return `${app.name} (${app.appId})`
}

export function statusColor(
  status: string,
): 'default' | 'success' | 'warning' | 'error' {
  switch (status) {
    case 'ready':
    case 'healthy':
    case 'planned':
    case 'completed':
    case 'passed':
    case 'pass':
    case 'appMatched':
      return 'success'
    case 'degraded':
    case 'warning':
    case 'warn':
    case 'skipped':
    case 'notApplicable':
    case 'unattributed':
      return 'warning'
    case 'blocked':
    case 'rejected':
    case 'failed':
    case 'fail':
    case 'appMismatch':
      return 'error'
    default:
      return 'default'
  }
}

export function sortSessions(sessions: AppRuntimeSessionRecord[]) {
  return [...sessions].sort(
    (left, right) =>
      right.startedAt - left.startedAt ||
      right.sessionId.localeCompare(left.sessionId),
  )
}

export function upsertSession(
  sessions: AppRuntimeSessionRecord[],
  nextSession: AppRuntimeSessionRecord,
) {
  const nextSessions = sessions.filter(
    (session) => session.sessionId !== nextSession.sessionId,
  )
  nextSessions.push(nextSession)
  return sortSessions(nextSessions)
}

export type FinishableSessionStatus = Exclude<
  AppRuntimeSessionStatus,
  'planned'
>

export type RuntimeResourceKind =
  | 'apps'
  | 'nodePools'
  | 'dnsProfiles'
  | 'securityProfiles'
  | 'policyBindings'

export const resourceKindOptions = [
  { value: 'apps', label: 'Apps' },
  { value: 'nodePools', label: 'Node pools' },
  { value: 'dnsProfiles', label: 'DNS profiles' },
  { value: 'securityProfiles', label: 'Security profiles' },
  { value: 'policyBindings', label: 'Policy bindings' },
]

export const routingIntentOptions = [
  { value: 'direct', label: 'direct' },
  { value: 'proxy', label: 'proxy' },
  { value: 'reject', label: 'reject' },
  { value: 'auto', label: 'auto' },
  { value: 'fallback', label: 'fallback' },
]

export const enabledOptions = [
  { value: 'true', label: 'enabled' },
  { value: 'false', label: 'disabled' },
]

export const processMatcherKindOptions = [
  { value: 'process_name', label: 'process_name' },
  { value: 'process_path', label: 'process_path' },
  { value: 'process_name_regex', label: 'process_name_regex' },
  { value: 'process_path_regex', label: 'process_path_regex' },
  { value: 'bundle_id', label: 'bundle_id' },
]

export const newResourceValue = '__new__'

export function now() {
  return Date.now()
}

export function createAppTemplate(): AppRegistryEntry {
  return {
    appId: 'new-app',
    name: 'New App',
    launchArgs: [],
    env: [],
    processMatchers: [{ kind: 'process_name', pattern: 'new-app.exe' }],
    platformMetadata: {},
    tags: [],
    updatedAt: now(),
  }
}

export function createNodePoolTemplate(): NodePool {
  return {
    poolId: 'new-pool',
    name: 'New Node Pool',
    tags: [],
    protocols: [],
    healthConstraints: {},
    candidateNodes: [{ nodeName: 'Proxy', tags: [] }],
    updatedAt: now(),
  }
}

export function createDnsProfileTemplate(): DnsProfile {
  return {
    profileId: 'new-dns-profile',
    name: 'New DNS Profile',
    configYaml: 'nameserver:\n  - 1.1.1.1',
    testDomain: 'example.com',
    tags: [],
    updatedAt: now(),
  }
}

export function createSecurityProfileTemplate(): SecurityProfile {
  return {
    profileId: 'new-security-profile',
    name: 'New Security Profile',
    controls: {
      requireNodePool: true,
      requireDnsProfile: false,
      allowedRoutingIntents: ['proxy', 'fallback'],
    },
    tags: [],
    updatedAt: now(),
  }
}

export function createPolicyBindingTemplate(appId = ''): AppPolicyBinding {
  return {
    bindingId: 'new-binding',
    appId: appId || 'new-app',
    routingIntent: 'proxy',
    enabled: true,
    updatedAt: now(),
  }
}

export function resourceIdFor(
  kind: RuntimeResourceKind,
  resource:
    | AppRegistryEntry
    | NodePool
    | DnsProfile
    | SecurityProfile
    | AppPolicyBinding,
) {
  switch (kind) {
    case 'apps':
      return (resource as AppRegistryEntry).appId
    case 'nodePools':
      return (resource as NodePool).poolId
    case 'dnsProfiles':
      return (resource as DnsProfile).profileId
    case 'securityProfiles':
      return (resource as SecurityProfile).profileId
    case 'policyBindings':
      return (resource as AppPolicyBinding).bindingId
  }
}

export function resourceNameFor(
  kind: RuntimeResourceKind,
  resource:
    | AppRegistryEntry
    | NodePool
    | DnsProfile
    | SecurityProfile
    | AppPolicyBinding,
) {
  switch (kind) {
    case 'apps':
      return (resource as AppRegistryEntry).name
    case 'nodePools':
      return (resource as NodePool).name
    case 'dnsProfiles':
      return (resource as DnsProfile).name
    case 'securityProfiles':
      return (resource as SecurityProfile).name
    case 'policyBindings':
      return `${(resource as AppPolicyBinding).appId} → ${(resource as AppPolicyBinding).routingIntent}`
  }
}

export function collectionFor(
  state: AppRuntimeStateDocument,
  kind: RuntimeResourceKind,
) {
  switch (kind) {
    case 'apps':
      return state.apps
    case 'nodePools':
      return state.nodePools
    case 'dnsProfiles':
      return state.dnsProfiles
    case 'securityProfiles':
      return state.securityProfiles
    case 'policyBindings':
      return state.policyBindings
  }
}

export function templateFor(kind: RuntimeResourceKind, appId = '') {
  switch (kind) {
    case 'apps':
      return createAppTemplate()
    case 'nodePools':
      return createNodePoolTemplate()
    case 'dnsProfiles':
      return createDnsProfileTemplate()
    case 'securityProfiles':
      return createSecurityProfileTemplate()
    case 'policyBindings':
      return createPolicyBindingTemplate(appId)
  }
}

export function parseJsonObject<T extends object>(raw: string): T {
  const parsed: unknown = JSON.parse(raw)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('JSON 必须是对象')
  }
  return parsed as T
}

export function formatJson(value: unknown) {
  return JSON.stringify(value, null, 2)
}

export function formatTime(timestamp?: number) {
  return timestamp ? new Date(timestamp).toLocaleString() : '-'
}

export function formatBytes(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`
  }
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`
}
