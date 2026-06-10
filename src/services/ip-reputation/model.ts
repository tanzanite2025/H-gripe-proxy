export type IpMetadataProviderKind = 'geoLite2AsnMmdb'
export type IpMetadataProviderAvailability =
  | 'ready'
  | 'experimental'
  | 'placeholder'

export interface IpReputationConfig {
  enabled: boolean
  cacheTtl: number
  routingRules: RiskRoutingRule[]
  metadataProvider: IpMetadataProviderConfig
}

export interface IpMetadataProviderConfig {
  kind: IpMetadataProviderKind
  options: Record<string, string>
}

export interface IpMetadataProviderHealthReport {
  providerKind: IpMetadataProviderKind
  providerLabel: string
  availability: IpMetadataProviderAvailability
  targetIp: string
  healthy: boolean
  message: string
  latencyMs?: number
  asn?: string
  asnOrg?: string
  countryCode?: string
  countryName?: string
  region?: string
  city?: string
  timezone?: string
  checkedAt: number
}

export interface IpReputation {
  ip: string
  ipType: 'Datacenter' | 'Residential' | 'Mobile' | 'Education' | 'Unknown'
  asn: string
  asnOrg: string
  fraudScore: number
  riskLevel: 'Low' | 'Medium' | 'High' | 'VeryHigh'
  confidence: number
  evidence: IpReputationEvidence[]
  residentialState:
    | 'notResidential'
    | 'observedResidential'
    | 'verifiedResidential'
    | 'unknown'
  isProxy: boolean
  isVpn: boolean
  isTor: boolean
  countryCode: string
  city?: string
  timezone?: string
  checkedAt: number
}

export interface IpReputationEvidence {
  kind:
    | 'asnTable'
    | 'metadataProvider'
    | 'orgKeyword'
    | 'reservedIp'
    | 'geoIp'
    | 'default'
  label: string
  weight: number
}

export type ResidentialProxyVerificationStatus =
  | 'verified'
  | 'observed'
  | 'rejected'
  | 'needsMihomoProbe'
  | 'failed'

export interface ResidentialProxyVerification {
  proxyName: string
  status: ResidentialProxyVerificationStatus
  egressIp?: string
  reputation?: IpReputation
  probeMethod: 'directProxy' | 'mihomoCore'
  mihomoProxyName?: string
  message: string
  checkedAt: number
}

export interface RiskRoutingRule {
  domainPatterns: string[]
  enabled: boolean
  requiredIpType?: 'Datacenter' | 'Residential' | 'Mobile' | 'Education'
  maxFraudScore: number
  fallbackPolicy: 'Block' | 'Warn' | 'Allow'
  description: string
}
