import type { ProxyDetectionLocation, ProxyDetectionResult } from '@/services/cmds'
import { getIpTypeText, getResidentialStateText } from '@/services/ip-reputation'

import type { DnsStatusColor } from '@/components/setting/dns-runtime-view-model'

export function formatProxyDetectionAssessmentLabel(assessment?: string) {
  switch (assessment) {
    case 'effective':
      return 'Exit changed'
    case 'same-egress':
      return 'Same exit'
    case 'runtime-risk':
      return 'Runtime risk'
    case 'inconclusive':
      return 'Inconclusive'
    default:
      return assessment || 'Unknown'
  }
}

export function getProxyDetectionAssessmentColor(
  assessment?: string,
): DnsStatusColor {
  switch (assessment) {
    case 'effective':
      return 'success'
    case 'same-egress':
    case 'runtime-risk':
      return 'warning'
    case 'inconclusive':
      return 'info'
    default:
      return 'default'
  }
}

export function formatProxyDetectionConfidenceLabel(confidence?: string) {
  switch (confidence) {
    case 'high':
      return 'High confidence'
    case 'medium':
      return 'Medium confidence'
    case 'low':
      return 'Low confidence'
    default:
      return confidence || 'Unknown'
  }
}

export function formatProxyDetectionObservationPath(observationPath?: string) {
  switch (observationPath) {
    case 'direct-vs-core-proxy':
      return 'Direct vs core'
    case 'direct-only':
      return 'Direct only'
    case 'core-proxy-only':
      return 'Core only'
    default:
      return observationPath || 'Unknown'
  }
}

export function formatProxyDetectionRuntimeRisk(risk: string) {
  switch (risk) {
    case 'core-not-running':
      return 'Local core is not running'
    case 'direct-egress-unavailable':
      return 'Direct egress unavailable'
    case 'local-core-proxy-unreachable':
      return 'Core proxy egress unavailable'
    case 'proxy-reputation-unavailable':
      return 'Proxy reputation unavailable'
    default:
      return risk
  }
}

export function formatProxyDetectionLocation(
  location?: ProxyDetectionLocation | null,
) {
  if (!location) {
    return 'Not observed'
  }

  return [location.country, location.region, location.city]
    .filter(Boolean)
    .join(' ') || 'Unknown'
}

export function getProxyDetectionReputationRiskColor(
  riskLevel?: string,
): DnsStatusColor {
  switch (riskLevel) {
    case 'Low':
      return 'success'
    case 'Medium':
      return 'info'
    case 'High':
    case 'VeryHigh':
      return 'warning'
    default:
      return 'default'
  }
}

export function buildProxyDetectionViewModel(result: ProxyDetectionResult) {
  const reputation = result.proxy_reputation
  const summary = result.proxy_effective
    ? {
        state: 'effective',
        title: 'Proxy exit changed',
        description: [
          result.ip_changed ? 'IP changed' : null,
          result.location_changed ? 'Location changed' : null,
        ]
          .filter(Boolean)
          .join(' / '),
        colorClass: 'text-success',
      }
    : result.assessment === 'same-egress'
      ? {
          state: 'same-egress',
          title: 'Same egress observed',
          description:
            'Direct and local-core proxy paths currently look identical.',
          colorClass: 'text-warning',
        }
      : {
          state: 'incomplete',
          title: 'Observation incomplete',
          description: 'Direct and proxy paths were not both observed.',
          colorClass: 'text-info',
        }

  return {
    summary,
    assessment: {
      label: formatProxyDetectionAssessmentLabel(result.assessment),
      color: getProxyDetectionAssessmentColor(result.assessment),
    },
    confidence: {
      label: formatProxyDetectionConfidenceLabel(result.confidence),
      color: 'info' as const,
    },
    observationPath: {
      label: formatProxyDetectionObservationPath(result.observation_path),
    },
    core: {
      label: result.core_running ? 'Core running' : 'Core stopped',
      color: result.core_running ? 'success' : 'warning',
    } satisfies { label: string; color: DnsStatusColor },
    direct: {
      ip: result.direct_ip || 'Not observed',
      location: formatProxyDetectionLocation(result.direct_location),
      observed: Boolean(result.direct_ip && result.direct_location),
    },
    proxy: {
      ip: result.proxy_ip || 'Not observed',
      location: formatProxyDetectionLocation(result.proxy_location),
      observed: Boolean(result.proxy_ip && result.proxy_location),
    },
    runtimeRiskText: result.runtime_risk_type
      .map(formatProxyDetectionRuntimeRisk)
      .join('; '),
    reputation: reputation
      ? {
          label: `${getIpTypeText(reputation.ipType)} / ${reputation.confidence}`,
          color: getProxyDetectionReputationRiskColor(reputation.riskLevel),
          asnLabel: `ASN ${reputation.asn} · ${reputation.asnOrg} · ${getResidentialStateText(reputation.residentialState)}`,
        }
      : null,
  }
}
