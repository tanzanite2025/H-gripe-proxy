export interface IpReputationConfig {
  enabled: boolean
  cacheTtl: number
  routingRules: RiskRoutingRule[]
  metadataProvider: IpMetadataProviderConfig
}

export interface IpMetadataProviderConfig {
  kind: 'geoLite2AsnMmdb' | 'ipinfoHttpApi'
  databasePath?: string
  apiEndpoint?: string
  accessToken?: string
  options: Record<string, string>
}

export interface IpMetadataProviderField {
  kind: 'databasePath' | 'apiEndpoint' | 'accessToken' | 'options'
  label: string
  required: boolean
  description: string
}

export interface IpMetadataProviderRegistration {
  kind: 'geoLite2AsnMmdb' | 'ipinfoHttpApi'
  label: string
  transport: 'localMmdb' | 'remoteHttpApi' | 'custom'
  availability: 'ready' | 'experimental' | 'placeholder'
  description: string
  fields: IpMetadataProviderField[]
  defaultDatabaseCandidates: string[]
}

export interface IpMetadataProviderHealthReport {
  providerKind: 'geoLite2AsnMmdb' | 'ipinfoHttpApi'
  providerLabel: string
  availability: 'ready' | 'experimental' | 'placeholder'
  targetIp: string
  healthy: boolean
  message: string
  latencyMs?: number
  asn?: string
  asnOrg?: string
  countryCode?: string
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
