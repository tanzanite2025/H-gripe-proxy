import { invoke } from '@tauri-apps/api/core';

import type { ResidentialProxy } from '@/services/coordinator';

type RawRecord = Record<string, unknown>;
type IpType = IpReputation['ipType'];
type RiskLevel = IpReputation['riskLevel'];
type IpReputationEvidenceKind = IpReputationEvidence['kind'];
type ResidentialVerificationState = IpReputation['residentialState'];
type FallbackPolicy = RiskRoutingRule['fallbackPolicy'];

const IP_TYPES: IpType[] = ['Datacenter', 'Residential', 'Mobile', 'Education', 'Unknown'];
const RISK_LEVELS: RiskLevel[] = ['Low', 'Medium', 'High', 'VeryHigh'];
const EVIDENCE_KINDS: IpReputationEvidenceKind[] = [
  'asnTable',
  'orgKeyword',
  'ipPrefix',
  'reservedIp',
  'geoIp',
  'default',
];
const RESIDENTIAL_STATES: ResidentialVerificationState[] = [
  'notResidential',
  'observedResidential',
  'verifiedResidential',
  'unknown',
];
const FALLBACK_POLICIES: FallbackPolicy[] = ['Block', 'Warn', 'Allow'];
const IP_TYPE_ALIASES: Record<string, IpType> = {
  datacenter: 'Datacenter',
  residential: 'Residential',
  mobile: 'Mobile',
  education: 'Education',
  unknown: 'Unknown',
};

/**
 * IP 信誉度配置
 */
export interface IpReputationConfig {
  enabled: boolean;
  cacheTtl: number;
  routingRules: RiskRoutingRule[];
  useLocalDb: boolean;
}

/**
 * IP 信誉度信息
 */
export interface IpReputation {
  ip: string;
  ipType: 'Datacenter' | 'Residential' | 'Mobile' | 'Education' | 'Unknown';
  asn: string;
  asnOrg: string;
  fraudScore: number;
  riskLevel: 'Low' | 'Medium' | 'High' | 'VeryHigh';
  confidence: number;
  evidence: IpReputationEvidence[];
  residentialState:
    | 'notResidential'
    | 'observedResidential'
    | 'verifiedResidential'
    | 'unknown';
  isProxy: boolean;
  isVpn: boolean;
  isTor: boolean;
  countryCode: string;
  city?: string;
  checkedAt: number;
}

export interface IpReputationEvidence {
  kind: 'asnTable' | 'orgKeyword' | 'ipPrefix' | 'reservedIp' | 'geoIp' | 'default';
  label: string;
  weight: number;
}

export type ResidentialProxyVerificationStatus =
  | 'verified'
  | 'observed'
  | 'rejected'
  | 'needsMihomoProbe'
  | 'failed';

export interface ResidentialProxyVerification {
  proxyName: string;
  status: ResidentialProxyVerificationStatus;
  egressIp?: string;
  reputation?: IpReputation;
  probeMethod: 'directProxy' | 'mihomoCore';
  mihomoProxyName?: string;
  message: string;
  checkedAt: number;
}

/**
 * 风控等级路由规则
 */
export interface RiskRoutingRule {
  domainPatterns: string[];
  enabled: boolean;
  requiredIpType?: 'Datacenter' | 'Residential' | 'Mobile' | 'Education';
  maxFraudScore: number;
  fallbackPolicy: 'Block' | 'Warn' | 'Allow';
  description: string;
}

const asRecord = (value: unknown): RawRecord =>
  value && typeof value === 'object' ? (value as RawRecord) : {};

const asString = (value: unknown, fallback = ''): string =>
  typeof value === 'string' ? value : fallback;

const asNumber = (value: unknown, fallback = 0): number => {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string') {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) return parsed;
  }
  return fallback;
};

const asBoolean = (value: unknown, fallback = false): boolean =>
  typeof value === 'boolean' ? value : fallback;

const normalizeEnum = <T extends string>(
  value: unknown,
  allowed: readonly T[],
  fallback: T,
): T => (allowed.includes(value as T) ? (value as T) : fallback);

const normalizeIpType = (value: unknown): IpType => {
  if (IP_TYPES.includes(value as IpType)) return value as IpType;
  if (typeof value === 'string') {
    return IP_TYPE_ALIASES[value] ?? IP_TYPE_ALIASES[value.toLowerCase()] ?? 'Unknown';
  }
  return 'Unknown';
};

function normalizeResidentialProxyVerification(value: unknown): ResidentialProxyVerification {
  const raw = asRecord(value);
  const status = asString(raw.status, 'failed') as ResidentialProxyVerificationStatus;

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
  };
}

const normalizeResidentialState = (value: unknown): ResidentialVerificationState =>
  normalizeEnum(value, RESIDENTIAL_STATES, 'unknown');

const normalizeEvidenceKind = (value: unknown): IpReputationEvidenceKind =>
  normalizeEnum(value, EVIDENCE_KINDS, 'default');

function normalizeIpReputationEvidence(value: unknown): IpReputationEvidence {
  const raw = asRecord(value);

  return {
    kind: normalizeEvidenceKind(raw.kind),
    label: asString(raw.label),
    weight: asNumber(raw.weight, 0),
  };
}

const toTauriIpType = (value?: Exclude<IpType, 'Unknown'>): string | null =>
  value ? value.charAt(0).toLowerCase() + value.slice(1) : null;

const normalizeCheckedAt = (value: unknown): number => {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string') {
    const parsed = Date.parse(value);
    if (Number.isFinite(parsed)) return parsed;
    return asNumber(value, Date.now());
  }
  const record = asRecord(value);
  const secs =
    record.secs_since_epoch ?? record.secsSinceEpoch ?? record.secs ?? record.seconds;
  const nanos = record.nanos_since_epoch ?? record.nanosSinceEpoch ?? record.nanos;
  if (secs !== undefined) {
    return asNumber(secs, 0) * 1000 + Math.floor(asNumber(nanos, 0) / 1_000_000);
  }
  return Date.now();
};

export function normalizeIpReputation(value: unknown): IpReputation {
  const raw = asRecord(value);

  return {
    ip: asString(raw.ip),
    ipType: normalizeIpType(raw.ipType ?? raw.ip_type),
    asn: asString(raw.asn, 'Unknown'),
    asnOrg: asString(raw.asnOrg ?? raw.asn_org, 'Unknown'),
    fraudScore: asNumber(raw.fraudScore ?? raw.fraud_score, 0),
    riskLevel: normalizeEnum(raw.riskLevel ?? raw.risk_level, RISK_LEVELS, 'Medium'),
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
    checkedAt: normalizeCheckedAt(raw.checkedAt ?? raw.checked_at),
  };
}

function normalizeRiskRoutingRule(value: unknown): RiskRoutingRule {
  const raw = asRecord(value);
  const requiredIpType = raw.requiredIpType ?? raw.required_ip_type;
  const domainPatterns = raw.domainPatterns ?? raw.domain_patterns;
  const normalizedRequiredIpType = normalizeIpType(requiredIpType);

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
  };
}

export function normalizeIpReputationConfig(value: unknown): IpReputationConfig {
  const raw = asRecord(value);
  const routingRules = raw.routingRules ?? raw.routing_rules;

  return {
    enabled: asBoolean(raw.enabled, true),
    cacheTtl: asNumber(raw.cacheTtl ?? raw.cache_ttl, 3600),
    routingRules: Array.isArray(routingRules)
      ? routingRules.map(normalizeRiskRoutingRule)
      : [],
    useLocalDb: asBoolean(raw.useLocalDb ?? raw.use_local_db, true),
  };
}

function serializeRiskRoutingRule(rule: RiskRoutingRule): RawRecord {
  return {
    domain_patterns: rule.domainPatterns,
    enabled: rule.enabled,
    required_ip_type: toTauriIpType(rule.requiredIpType),
    max_fraud_score: rule.maxFraudScore,
    fallback_policy: rule.fallbackPolicy,
    description: rule.description,
  };
}

function serializeIpReputationConfig(config: IpReputationConfig): RawRecord {
  return {
    enabled: config.enabled,
    cache_ttl: config.cacheTtl,
    routing_rules: config.routingRules.map(serializeRiskRoutingRule),
    use_local_db: config.useLocalDb,
  };
}

/**
 * 获取 IP 信誉度配置
 */
export async function ipReputationGetConfig(): Promise<IpReputationConfig> {
  return normalizeIpReputationConfig(await invoke('ip_reputation_get_config'));
}

/**
 * 更新 IP 信誉度配置
 */
export async function ipReputationUpdateConfig(
  config: IpReputationConfig
): Promise<void> {
  await invoke('ip_reputation_update_config', {
    config: serializeIpReputationConfig(config),
  });
}

/**
 * 检测 IP 信誉度
 */
export async function ipReputationCheckIp(ip: string): Promise<IpReputation> {
  return normalizeIpReputation(await invoke('ip_reputation_check_ip', { ip }));
}

/**
 * 获取预定义路由规则
 */
export async function ipReputationGetPredefinedRules(): Promise<RiskRoutingRule[]> {
  const rules = await invoke<unknown[]>('ip_reputation_get_predefined_rules');
  return rules.map(normalizeRiskRoutingRule);
}

/**
 * 为域名选择节点
 */
export async function ipReputationSelectNodeForDomain(
  domain: string,
  availableNodes: [string, string][]
): Promise<string> {
  return await invoke<string>('ip_reputation_select_node_for_domain', {
    domain,
    availableNodes,
  });
}

/**
 * 清除缓存
 */
export async function ipReputationClearCache(): Promise<void> {
  await invoke('ip_reputation_clear_cache');
}

/**
 * 获取缓存统计
 */
export async function ipReputationGetCacheStats(): Promise<[number, number]> {
  return await invoke<[number, number]>('ip_reputation_get_cache_stats');
}

/**
 * 获取缓存中所有条目
 */
export async function ipReputationGetCacheEntries(): Promise<IpReputation[]> {
  const entries = await invoke<unknown[]>('ip_reputation_get_cache_entries');
  return entries.map(normalizeIpReputation);
}

export async function ipReputationVerifyResidentialProxy(
  proxy: ResidentialProxy,
): Promise<ResidentialProxyVerification> {
  return normalizeResidentialProxyVerification(
    await invoke('ip_reputation_verify_residential_proxy', { proxy }),
  );
}

/**
 * 获取 IP 类型的显示文本
 */
export function getIpTypeText(ipType: string): string {
  switch (ipType) {
    case 'Datacenter':
      return '机房 IP';
    case 'Residential':
      return '住宅特征';
    case 'Mobile':
      return '移动特征';
    case 'Education':
      return '教育网特征';
    default:
      return '未知';
  }
}

export function getResidentialStateText(state: string): string {
  switch (state) {
    case 'notResidential':
      return '非住宅';
    case 'observedResidential':
      return '观测像住宅';
    case 'verifiedResidential':
      return '已验证住宅';
    default:
      return '未确认';
  }
}

/**
 * 获取风险等级的显示文本
 */
export function getRiskLevelText(riskLevel: string): string {
  switch (riskLevel) {
    case 'Low':
      return '低风险';
    case 'Medium':
      return '中风险';
    case 'High':
      return '高风险';
    case 'VeryHigh':
      return '极高风险';
    default:
      return '未知';
  }
}

/**
 * 获取风险等级的颜色
 */
export function getRiskLevelColor(riskLevel: string): string {
  switch (riskLevel) {
    case 'Low':
      return 'text-green-600';
    case 'Medium':
      return 'text-yellow-600';
    case 'High':
      return 'text-orange-600';
    case 'VeryHigh':
      return 'text-red-600';
    default:
      return 'text-gray-600';
  }
}
