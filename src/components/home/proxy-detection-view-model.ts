import type { ProxyDetectionLocation, ProxyDetectionResult } from '@/services/cmds'

import type { DnsStatusColor } from '@/components/setting/dns-runtime-view-model'
import type { TranslationKey } from '@/types/generated/i18n-keys'

type Translate = (key: TranslationKey, options?: Record<string, unknown>) => string

const proxyDetectionKey = (path: string) =>
  `home.components.proxyDetection.${path}` as TranslationKey

export function formatProxyDetectionAssessmentLabel(
  t: Translate,
  assessment?: string,
) {
  switch (assessment) {
    case 'effective':
      return t(proxyDetectionKey('assessment.effective'))
    case 'same-egress':
      return t(proxyDetectionKey('assessment.sameEgress'))
    case 'runtime-risk':
      return t(proxyDetectionKey('assessment.runtimeRisk'))
    case 'inconclusive':
      return t(proxyDetectionKey('assessment.inconclusive'))
    default:
      return assessment || t(proxyDetectionKey('labels.unknown'))
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

export function formatProxyDetectionConfidenceLabel(
  t: Translate,
  confidence?: string,
) {
  switch (confidence) {
    case 'high':
      return t(proxyDetectionKey('confidence.high'))
    case 'medium':
      return t(proxyDetectionKey('confidence.medium'))
    case 'low':
      return t(proxyDetectionKey('confidence.low'))
    default:
      return confidence || t(proxyDetectionKey('labels.unknown'))
  }
}

export function formatProxyDetectionObservationPath(
  t: Translate,
  observationPath?: string,
) {
  switch (observationPath) {
    case 'direct-vs-core-proxy':
      return t(proxyDetectionKey('observationPath.directVsCore'))
    case 'direct-only':
      return t(proxyDetectionKey('observationPath.directOnly'))
    case 'core-proxy-only':
      return t(proxyDetectionKey('observationPath.coreOnly'))
    default:
      return observationPath || t(proxyDetectionKey('labels.unknown'))
  }
}

export function formatProxyDetectionRuntimeRisk(t: Translate, risk: string) {
  switch (risk) {
    case 'core-not-running':
      return t(proxyDetectionKey('runtimeRisk.coreNotRunning'))
    case 'direct-egress-unavailable':
      return t(proxyDetectionKey('runtimeRisk.directEgressUnavailable'))
    case 'local-core-proxy-unreachable':
      return t(proxyDetectionKey('runtimeRisk.localCoreProxyUnreachable'))
    case 'proxy-reputation-unavailable':
      return t(proxyDetectionKey('runtimeRisk.proxyReputationUnavailable'))
    default:
      return risk
  }
}

export function formatProxyDetectionLocation(
  t: Translate,
  location?: ProxyDetectionLocation | null,
) {
  if (!location) {
    return t(proxyDetectionKey('labels.notObserved'))
  }

  return [location.country, location.region, location.city]
    .filter(Boolean)
    .join(' ') || t(proxyDetectionKey('labels.unknown'))
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

export function formatProxyDetectionIpType(t: Translate, ipType: string) {
  switch (ipType) {
    case 'Datacenter':
      return t(proxyDetectionKey('ipType.datacenter'))
    case 'Residential':
      return t(proxyDetectionKey('ipType.residential'))
    case 'Mobile':
      return t(proxyDetectionKey('ipType.mobile'))
    case 'Education':
      return t(proxyDetectionKey('ipType.education'))
    default:
      return t(proxyDetectionKey('labels.unknown'))
  }
}

export function formatProxyDetectionResidentialState(
  t: Translate,
  state: string,
) {
  switch (state) {
    case 'notResidential':
      return t(proxyDetectionKey('residentialState.notResidential'))
    case 'observedResidential':
      return t(proxyDetectionKey('residentialState.observedResidential'))
    case 'verifiedResidential':
      return t(proxyDetectionKey('residentialState.verifiedResidential'))
    default:
      return t(proxyDetectionKey('residentialState.unknown'))
  }
}

function buildProxyDetectionRecommendations(
  result: ProxyDetectionResult,
  t: Translate,
) {
  const recommendations: string[] = []

  if (result.proxy_effective) {
    if (result.ip_changed) {
      recommendations.push(t(proxyDetectionKey('advice.ipChanged')))
    }
    if (result.location_changed) {
      recommendations.push(t(proxyDetectionKey('advice.locationChanged')))
    }
    if (result.proxy_reputation) {
      recommendations.push(
        t(proxyDetectionKey('advice.reputation'), {
          ipType: formatProxyDetectionIpType(
            t,
            result.proxy_reputation.ipType,
          ),
          score: result.proxy_reputation.fraudScore,
          asn: result.proxy_reputation.asn,
        }),
      )
    }
    if (!recommendations.length) {
      recommendations.push(t(proxyDetectionKey('advice.proxyEffective')))
    }
    return recommendations
  }

  result.runtime_risk_type.forEach((risk) => {
    recommendations.push(formatProxyDetectionRuntimeRisk(t, risk))
  })

  if (result.proxy_reputation) {
    recommendations.push(
      t(proxyDetectionKey('advice.reputation'), {
        ipType: formatProxyDetectionIpType(t, result.proxy_reputation.ipType),
        score: result.proxy_reputation.fraudScore,
        asn: result.proxy_reputation.asn,
      }),
    )
  }

  if (
    result.core_running &&
    result.observation_path === 'direct-vs-core-proxy'
  ) {
    recommendations.push(t(proxyDetectionKey('advice.sameEgress')))
  }

  if (result.observation_incomplete) {
    recommendations.push(t(proxyDetectionKey('advice.observationIncomplete')))
  }

  if (!recommendations.length) {
    recommendations.push(t(proxyDetectionKey('advice.noClearChange')))
  }

  return recommendations
}

export function buildProxyDetectionViewModel(
  result: ProxyDetectionResult,
  t: Translate,
) {
  const reputation = result.proxy_reputation
  const summary = result.proxy_effective
    ? {
        state: 'effective',
        title: t(proxyDetectionKey('summary.effective.title')),
        description: [
          result.ip_changed
            ? t(proxyDetectionKey('summary.effective.ipChanged'))
            : null,
          result.location_changed
            ? t(proxyDetectionKey('summary.effective.locationChanged'))
            : null,
        ]
          .filter(Boolean)
          .join(' / '),
        colorClass: 'text-success',
      }
    : result.assessment === 'same-egress'
      ? {
          state: 'same-egress',
          title: t(proxyDetectionKey('summary.sameEgress.title')),
          description: t(proxyDetectionKey('summary.sameEgress.description')),
          colorClass: 'text-warning',
        }
      : {
          state: 'incomplete',
          title: t(proxyDetectionKey('summary.incomplete.title')),
          description: t(proxyDetectionKey('summary.incomplete.description')),
          colorClass: 'text-info',
        }

  return {
    summary,
    assessment: {
      label: formatProxyDetectionAssessmentLabel(t, result.assessment),
      color: getProxyDetectionAssessmentColor(result.assessment),
    },
    confidence: {
      label: formatProxyDetectionConfidenceLabel(t, result.confidence),
      color: 'info' as const,
    },
    observationPath: {
      label: formatProxyDetectionObservationPath(t, result.observation_path),
    },
    core: {
      label: result.core_running
        ? t(proxyDetectionKey('core.running'))
        : t(proxyDetectionKey('core.stopped')),
      color: result.core_running ? 'success' : 'warning',
    } satisfies { label: string; color: DnsStatusColor },
    direct: {
      ip: result.direct_ip || t(proxyDetectionKey('labels.notObserved')),
      location: formatProxyDetectionLocation(t, result.direct_location),
      observed: Boolean(result.direct_ip && result.direct_location),
    },
    proxy: {
      ip: result.proxy_ip || t(proxyDetectionKey('labels.notObserved')),
      location: formatProxyDetectionLocation(t, result.proxy_location),
      observed: Boolean(result.proxy_ip && result.proxy_location),
    },
    runtimeRiskText: result.runtime_risk_type
      .map((risk) => formatProxyDetectionRuntimeRisk(t, risk))
      .join('; '),
    recommendations: buildProxyDetectionRecommendations(result, t),
    reputation: reputation
      ? {
          label: t(proxyDetectionKey('patterns.reputation'), {
            ipType: formatProxyDetectionIpType(t, reputation.ipType),
            confidence: reputation.confidence,
          }),
          color: getProxyDetectionReputationRiskColor(reputation.riskLevel),
          asnLabel: t(proxyDetectionKey('patterns.asn'), {
            asn: reputation.asn,
            org: reputation.asnOrg,
            residentialState: formatProxyDetectionResidentialState(
              t,
              reputation.residentialState,
            ),
          }),
        }
      : null,
  }
}
