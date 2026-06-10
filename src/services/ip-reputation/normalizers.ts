import type {
  IpMetadataProviderAvailability,
  IpMetadataProviderConfig,
  IpMetadataProviderHealthReport,
  IpReputation,
  IpReputationConfig,
  IpReputationEvidence,
  ResidentialProxyVerification,
  RiskRoutingRule,
} from './model'

type RawRecord = Record<string, unknown>
type IpType = IpReputation['ipType']
type RiskLevel = IpReputation['riskLevel']
type IpReputationEvidenceKind = IpReputationEvidence['kind']
type ResidentialVerificationState = IpReputation['residentialState']
type FallbackPolicy = RiskRoutingRule['fallbackPolicy']

const IP_TYPES: IpType[] = [
  'Datacenter',
  'Residential',
  'Mobile',
  'Education',
  'Unknown',
]
const RISK_LEVELS: RiskLevel[] = ['Low', 'Medium', 'High', 'VeryHigh']
const EVIDENCE_KINDS: IpReputationEvidenceKind[] = [
  'asnTable',
  'metadataProvider',
  'orgKeyword',
  'reservedIp',
  'geoIp',
  'default',
]
const RESIDENTIAL_STATES: ResidentialVerificationState[] = [
  'notResidential',
  'observedResidential',
  'verifiedResidential',
  'unknown',
]
const FALLBACK_POLICIES: FallbackPolicy[] = ['Block', 'Warn', 'Allow']
const METADATA_PROVIDER_AVAILABILITIES: IpMetadataProviderAvailability[] = [
  'ready',
  'experimental',
  'placeholder',
]
const IP_TYPE_ALIASES: Record<string, IpType> = {
  datacenter: 'Datacenter',
  residential: 'Residential',
  mobile: 'Mobile',
  education: 'Education',
  unknown: 'Unknown',
}

const asRecord = (value: unknown): RawRecord =>
  value && typeof value === 'object' ? (value as RawRecord) : {}

const asString = (value: unknown, fallback = ''): string =>
  typeof value === 'string' ? value : fallback

const asNumber = (value: unknown, fallback = 0): number => {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (typeof value === 'string') {
    const parsed = Number(value)
    if (Number.isFinite(parsed)) return parsed
  }
  return fallback
}

const asBoolean = (value: unknown, fallback = false): boolean =>
  typeof value === 'boolean' ? value : fallback

const normalizeEnum = <T extends string>(
  value: unknown,
  allowed: readonly T[],
  fallback: T,
): T => (allowed.includes(value as T) ? (value as T) : fallback)

const normalizeIpType = (value: unknown): IpType => {
  if (IP_TYPES.includes(value as IpType)) return value as IpType
  if (typeof value === 'string') {
    return IP_TYPE_ALIASES[value] ?? IP_TYPE_ALIASES[value.toLowerCase()] ?? 'Unknown'
  }
  return 'Unknown'
}

const normalizeCheckedAt = (value: unknown): number => {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (typeof value === 'string') {
    const parsed = Date.parse(value)
    if (Number.isFinite(parsed)) return parsed
    return asNumber(value, Date.now())
  }
  const record = asRecord(value)
  const secs =
    record.secs_since_epoch ?? record.secsSinceEpoch ?? record.secs ?? record.seconds
  const nanos = record.nanos_since_epoch ?? record.nanosSinceEpoch ?? record.nanos
  if (secs !== undefined) {
    return asNumber(secs, 0) * 1000 + Math.floor(asNumber(nanos, 0) / 1_000_000)
  }
  return Date.now()
}

const normalizeResidentialState = (value: unknown): ResidentialVerificationState =>
  normalizeEnum(value, RESIDENTIAL_STATES, 'unknown')

const normalizeEvidenceKind = (value: unknown): IpReputationEvidenceKind =>
  normalizeEnum(value, EVIDENCE_KINDS, 'default')

export const normalizeMetadataProviderConfig = (
  value: unknown,
): IpMetadataProviderConfig => {
  const raw = asRecord(value)
  const options = raw.options

  return {
    kind: 'geoLite2AsnMmdb',
    options:
      options && typeof options === 'object'
        ? Object.fromEntries(
            Object.entries(options as RawRecord).map(([key, item]) => [
              key,
              String(item),
            ]),
          )
        : {},
  }
}

export const normalizeMetadataProviderHealthReport = (
  value: unknown,
): IpMetadataProviderHealthReport => {
  const raw = asRecord(value)

  return {
    providerKind: 'geoLite2AsnMmdb',
    providerLabel: asString(
      raw.providerLabel ?? raw.provider_label,
      'GeoLite2 Local MMDB',
    ),
    availability: normalizeEnum(
      raw.availability,
      METADATA_PROVIDER_AVAILABILITIES,
      'placeholder',
    ),
    targetIp: asString(raw.targetIp ?? raw.target_ip),
    healthy: asBoolean(raw.healthy, false),
    message: asString(raw.message),
    latencyMs:
      raw.latencyMs === undefined && raw.latency_ms === undefined
        ? undefined
        : asNumber(raw.latencyMs ?? raw.latency_ms, 0),
    asn: asString(raw.asn) || undefined,
    asnOrg: asString(raw.asnOrg ?? raw.asn_org) || undefined,
    countryCode: asString(raw.countryCode ?? raw.country_code) || undefined,
    countryName: asString(raw.countryName ?? raw.country_name) || undefined,
    region: asString(raw.region) || undefined,
    city: asString(raw.city) || undefined,
    timezone: asString(raw.timezone) || undefined,
    checkedAt: normalizeCheckedAt(raw.checkedAt ?? raw.checked_at),
  }
}

export function normalizeResidentialProxyVerification(
  value: unknown,
): ResidentialProxyVerification {
  const raw = asRecord(value)
  const status = asString(raw.status, 'failed') as ResidentialProxyVerification['status']

  return {
    proxyName: asString(raw.proxyName ?? raw.proxy_name),
    status,
    egressIp: asString(raw.egressIp ?? raw.egress_ip) || undefined,
    reputation:
      raw.reputation === undefined || raw.reputation === null
        ? undefined
        : normalizeIpReputation(raw.reputation),
    probeMethod:
      asString(raw.probeMethod ?? raw.probe_method) === 'mihomoCore'
        ? 'mihomoCore'
        : 'directProxy',
    mihomoProxyName: asString(raw.mihomoProxyName ?? raw.mihomo_proxy_name) || undefined,
    message: asString(raw.message),
    checkedAt: normalizeCheckedAt(raw.checkedAt ?? raw.checked_at),
  }
}

function normalizeIpReputationEvidence(value: unknown): IpReputationEvidence {
  const raw = asRecord(value)

  return {
    kind: normalizeEvidenceKind(raw.kind),
    label: asString(raw.label),
    weight: asNumber(raw.weight, 0),
  }
}

export function normalizeIpReputation(value: unknown): IpReputation {
  const raw = asRecord(value)

  return {
    ip: asString(raw.ip),
    ipType: normalizeIpType(raw.ipType ?? raw.ip_type),
    asn: asString(raw.asn, 'Unknown'),
    asnOrg: asString(raw.asnOrg ?? raw.asn_org, 'Unknown'),
    fraudScore: asNumber(raw.fraudScore ?? raw.fraud_score, 0),
    riskLevel: normalizeEnum(
      raw.riskLevel ?? raw.risk_level,
      RISK_LEVELS,
      'Medium',
    ),
    confidence: asNumber(raw.confidence, 0),
    evidence: Array.isArray(raw.evidence)
      ? raw.evidence.map(normalizeIpReputationEvidence)
      : [],
    residentialState: normalizeResidentialState(
      raw.residentialState ?? raw.residential_state,
    ),
    isProxy: asBoolean(raw.isProxy ?? raw.is_proxy),
    isVpn: asBoolean(raw.isVpn ?? raw.is_vpn),
    isTor: asBoolean(raw.isTor ?? raw.is_tor),
    countryCode: asString(raw.countryCode ?? raw.country_code, 'Unknown'),
    city: asString(raw.city) || undefined,
    timezone: asString(raw.timezone) || undefined,
    checkedAt: normalizeCheckedAt(raw.checkedAt ?? raw.checked_at),
  }
}

export function normalizeRiskRoutingRule(value: unknown): RiskRoutingRule {
  const raw = asRecord(value)
  const requiredIpType = raw.requiredIpType ?? raw.required_ip_type
  const domainPatterns = raw.domainPatterns ?? raw.domain_patterns
  const normalizedRequiredIpType = normalizeIpType(requiredIpType)

  return {
    domainPatterns: Array.isArray(domainPatterns)
      ? domainPatterns.map((item) => String(item))
      : [],
    enabled: asBoolean(raw.enabled, true),
    requiredIpType:
      requiredIpType === undefined || requiredIpType === null
        ? undefined
        : normalizedRequiredIpType === 'Unknown'
          ? undefined
          : (normalizedRequiredIpType as Exclude<IpType, 'Unknown'>),
    maxFraudScore: asNumber(raw.maxFraudScore ?? raw.max_fraud_score, 100),
    fallbackPolicy: normalizeEnum(
      raw.fallbackPolicy ?? raw.fallback_policy,
      FALLBACK_POLICIES,
      'Warn',
    ),
    description: asString(raw.description),
  }
}

export function normalizeIpReputationConfig(value: unknown): IpReputationConfig {
  const raw = asRecord(value)
  const routingRules = raw.routingRules ?? raw.routing_rules
  const metadataProvider = raw.metadataProvider ?? raw.metadata_provider

  return {
    enabled: asBoolean(raw.enabled, true),
    cacheTtl: asNumber(raw.cacheTtl ?? raw.cache_ttl, 3600),
    routingRules: Array.isArray(routingRules)
      ? routingRules.map(normalizeRiskRoutingRule)
      : [],
    metadataProvider: normalizeMetadataProviderConfig(metadataProvider),
  }
}

const toTauriIpType = (value?: Exclude<IpType, 'Unknown'>): string | null =>
  value ? value.charAt(0).toLowerCase() + value.slice(1) : null

function serializeRiskRoutingRule(rule: RiskRoutingRule): RawRecord {
  return {
    domain_patterns: rule.domainPatterns,
    enabled: rule.enabled,
    required_ip_type: toTauriIpType(rule.requiredIpType),
    max_fraud_score: rule.maxFraudScore,
    fallback_policy: rule.fallbackPolicy,
    description: rule.description,
  }
}

export function serializeIpReputationConfig(config: IpReputationConfig): RawRecord {
  return {
    enabled: config.enabled,
    cache_ttl: config.cacheTtl,
    routing_rules: config.routingRules.map(serializeRiskRoutingRule),
    metadata_provider: {
      kind: config.metadataProvider.kind,
      options: config.metadataProvider.options,
    },
  }
}
