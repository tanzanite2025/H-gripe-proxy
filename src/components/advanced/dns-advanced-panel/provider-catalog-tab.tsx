import { useQuery } from '@tanstack/react-query'
import { useLockFn } from 'ahooks'
import { RefreshCw, Radar } from 'lucide-react'
import { useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import type {
  DnsProtocol,
  DnsRuntimeStatus,
  DnsServerProviderHealthReport,
  DnsServerProviderKind,
  DnsServerProviderRegistration,
} from '@/services/cmds'
import {
  getDnsProviderRegistrations,
  probeDnsProvider,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

interface Props {
  runtimeStatus?: DnsRuntimeStatus
}

type RuntimeProviderMatchReasonKind =
  | 'host_alias'
  | 'bootstrap_ip'
  | 'recommended_server_exact'
  | 'recommended_server_authority'

interface RuntimeProviderMatchReason {
  kind: RuntimeProviderMatchReasonKind
  value: string
}

interface RuntimeProviderResolution {
  provider: DnsServerProviderRegistration
  reasons: RuntimeProviderMatchReason[]
}

interface RuntimeProviderMatch {
  server: string
  authority: string
  providers: RuntimeProviderResolution[]
  unmatchedReason?: string
}

const PROVIDER_STATUS_COLORS = {
  ready: 'success',
  experimental: 'warning',
  placeholder: 'default',
} as const

const HEALTH_STATUS_COLORS = {
  healthy: 'success',
  unhealthy: 'error',
  idle: 'default',
} as const

function normalizeDnsAuthority(server: string): string {
  const trimmed = server.trim().toLowerCase()
  if (!trimmed) {
    return ''
  }

  const withoutScheme = trimmed.includes('://')
    ? trimmed.split('://')[1] ?? ''
    : trimmed
  const authority = withoutScheme.split('/')[0] ?? ''

  if (authority.startsWith('[')) {
    const closing = authority.indexOf(']')
    return closing >= 0 ? authority.slice(1, closing) : authority
  }

  const colonCount = (authority.match(/:/g) ?? []).length
  if (colonCount === 1) {
    return authority.split(':')[0] ?? authority
  }

  return authority
}

function resolveProviderMatchReasons(
  provider: DnsServerProviderRegistration,
  server: string,
) {
  const normalizedServer = server.trim().toLowerCase()
  const normalizedAuthority = normalizeDnsAuthority(server)
  const reasons: RuntimeProviderMatchReason[] = []

  if (
    provider.host_aliases.some(
      (alias) => alias.toLowerCase() === normalizedAuthority,
    )
  ) {
    reasons.push({
      kind: 'host_alias',
      value: normalizedAuthority,
    })
  }

  if (
    provider.bootstrap_ips.some(
      (ip) => ip.toLowerCase() === normalizedAuthority,
    )
  ) {
    reasons.push({
      kind: 'bootstrap_ip',
      value: normalizedAuthority,
    })
  }

  provider.recommended_servers.forEach((item) => {
    const candidate = item.server.toLowerCase()

    if (candidate === normalizedServer) {
      reasons.push({
        kind: 'recommended_server_exact',
        value: item.server,
      })
      return
    }

    if (normalizeDnsAuthority(candidate) === normalizedAuthority) {
      reasons.push({
        kind: 'recommended_server_authority',
        value: item.server,
      })
    }
  })

  return reasons
}

function explainUnmatchedRuntimeServer(server: string, authority: string) {
  if (!server.trim()) {
    return 'Runtime DNS server is empty.'
  }

  if (!authority) {
    return 'Unable to extract a normalized DNS authority from this runtime server.'
  }

  return `Authority ${authority} was not found in registered host aliases, bootstrap IPs, or recommended servers.`
}

function formatMatchReason(reason: RuntimeProviderMatchReason) {
  switch (reason.kind) {
    case 'host_alias':
      return `Host alias: ${reason.value}`
    case 'bootstrap_ip':
      return `Bootstrap IP: ${reason.value}`
    case 'recommended_server_exact':
      return `Recommended server: ${reason.value}`
    case 'recommended_server_authority':
      return `Recommended authority: ${reason.value}`
    default:
      return reason.value
  }
}

function resolveRuntimeProviderLabels(
  servers: string[],
  providers: DnsServerProviderRegistration[],
) {
  const labels = new Set<string>()

  resolveRuntimeProviderMatches(servers, providers).forEach((match) => {
    match.providers.forEach((providerMatch) => {
      labels.add(providerMatch.provider.label)
    })
  })

  return Array.from(labels)
}

function resolveRuntimeProviderMatches(
  servers: string[],
  providers: DnsServerProviderRegistration[],
): RuntimeProviderMatch[] {
  return servers.map((server) => {
    const authority = normalizeDnsAuthority(server)
    const matchedProviders = providers
      .map((provider) => {
        const reasons = resolveProviderMatchReasons(provider, server)

        if (reasons.length === 0) {
          return null
        }

        return {
          provider,
          reasons,
        }
      })
      .filter((item): item is RuntimeProviderResolution => item != null)

    return {
      server,
      authority,
      providers: matchedProviders,
      unmatchedReason:
        matchedProviders.length > 0
          ? undefined
          : explainUnmatchedRuntimeServer(server, authority),
    }
  })
}

function collectMatchedProviderKinds(matches: RuntimeProviderMatch[]) {
  return new Set(
    matches.flatMap((match) =>
      match.providers.map((providerMatch) => providerMatch.provider.kind),
    ),
  )
}

function isFullyMatched(matches: RuntimeProviderMatch[]) {
  return matches.every((match) => match.providers.length > 0)
}

function getPreferredProbeProtocol(
  provider: DnsServerProviderRegistration,
): DnsProtocol | undefined {
  return (
    provider.supported_protocols.find((protocol) => protocol === 'doh') ??
    provider.supported_protocols.find((protocol) => protocol === 'dot') ??
    provider.supported_protocols.find((protocol) => protocol === 'udp') ??
    provider.supported_protocols[0]
  )
}

function formatCheckedAt(value: DnsServerProviderHealthReport['checked_at']) {
  if (typeof value === 'string') {
    const parsed = new Date(value)
    return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString()
  }

  if (typeof value === 'number') {
    return new Date(value).toLocaleString()
  }

  if (value && typeof value === 'object') {
    const secs =
      value.secs_since_epoch ??
      value.secsSinceEpoch ??
      value.secs ??
      value.seconds
    const nanos = value.nanos_since_epoch ?? value.nanosSinceEpoch ?? value.nanos
    if (typeof secs === 'number') {
      return new Date(secs * 1000 + Math.floor((nanos ?? 0) / 1_000_000)).toLocaleString()
    }
  }

  return 'Unknown'
}

export function ProviderCatalogTab({ runtimeStatus }: Props) {
  const [probeReports, setProbeReports] = useState<
    Partial<Record<DnsServerProviderKind, DnsServerProviderHealthReport>>
  >({})
  const [probingKind, setProbingKind] = useState<DnsServerProviderKind | null>(
    null,
  )

  const {
    data: providers,
    isPending,
    refetch,
  } = useQuery({
    queryKey: ['dnsProviderRegistrations'],
    queryFn: getDnsProviderRegistrations,
  })

  const handleProbe = useLockFn(async (provider: DnsServerProviderRegistration) => {
    const preferredProtocol = getPreferredProbeProtocol(provider)
    setProbingKind(provider.kind)

    try {
      const report = await probeDnsProvider(provider.kind, preferredProtocol)
      setProbeReports((current) => ({
        ...current,
        [provider.kind]: report,
      }))

      showNotice.success(
        report.healthy
          ? `${provider.label} probe succeeded`
          : `${provider.label} probe reported an unhealthy result`,
      )
    } catch (error: any) {
      showNotice.error(error)
    } finally {
      setProbingKind(null)
    }
  })

  if (isPending && !providers) {
    return (
      <div className="rounded-lg border border-border bg-card p-4">
        <div className="flex items-center gap-2 text-sm font-semibold">
          <Radar className="h-4 w-4" />
          Provider Catalog
        </div>
        <div className="mt-2 text-sm text-muted-foreground">
          Loading registered DNS providers...
        </div>
      </div>
    )
  }

  const providerList = providers ?? []
  const domesticProviderMatches = resolveRuntimeProviderMatches(
    runtimeStatus?.derived.domestic_dns ?? [],
    providerList,
  )
  const foreignProviderMatches = resolveRuntimeProviderMatches(
    runtimeStatus?.derived.foreign_dns ?? [],
    providerList,
  )
  const domesticMatches = resolveRuntimeProviderLabels(
    runtimeStatus?.derived.domestic_dns ?? [],
    providerList,
  )
  const foreignMatches = resolveRuntimeProviderLabels(
    runtimeStatus?.derived.foreign_dns ?? [],
    providerList,
  )
  const domesticProviderKinds = collectMatchedProviderKinds(domesticProviderMatches)
  const foreignProviderKinds = collectMatchedProviderKinds(foreignProviderMatches)
  const runtimeResolutionFullyMatched =
    isFullyMatched(domesticProviderMatches) && isFullyMatched(foreignProviderMatches)

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold">
              <Radar className="h-4 w-4" />
              Provider Catalog
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              Single fact source for built-in DNS provider metadata. This catalog
              does not change runtime DNS by itself.
            </div>
          </div>
          <Button
            size="small"
            variant="outlined"
            startIcon={<RefreshCw className="h-4 w-4" />}
            onClick={() => void refetch()}
          >
            Refresh
          </Button>
        </div>

        <Alert severity="info" className="mt-3 text-sm">
          Runtime DNS remains authoritative. This catalog only exposes the
          built-in provider registry, recommended endpoints, and direct
          connectivity probes.
        </Alert>

        <div className="mt-3 rounded-lg border border-border px-3 py-3">
          <div className="flex flex-wrap items-center gap-2">
            <div className="text-sm font-semibold">Current effective provider view</div>
            <Chip
              size="small"
              color={runtimeResolutionFullyMatched ? 'success' : 'warning'}
              label={
                runtimeResolutionFullyMatched
                  ? 'All runtime DNS mapped to registry'
                  : 'Runtime DNS contains unmatched/custom servers'
              }
            />
          </div>

          <div className="mt-3 grid grid-cols-1 gap-3 xl:grid-cols-2">
            <div className="rounded-lg border border-border px-3 py-2">
              <div className="text-xs font-semibold text-muted-foreground">
                Domestic effective providers
              </div>
              <div className="mt-2 space-y-2">
                {domesticProviderMatches.length > 0 ? (
                  domesticProviderMatches.map((match) => (
                    <div
                      key={`domestic-${match.server}`}
                      className="rounded-md border border-border px-3 py-2"
                    >
                      <div className="break-all font-mono text-xs">{match.server}</div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        Authority:{' '}
                        <span className="font-mono">
                          {match.authority || 'Unavailable'}
                        </span>
                      </div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {match.providers.length > 0 ? (
                          match.providers.map((providerMatch) => (
                            <Chip
                              key={`domestic-${match.server}-${providerMatch.provider.kind}`}
                              size="small"
                              color="success"
                              label={providerMatch.provider.label}
                            />
                          ))
                        ) : (
                          <Chip
                            size="small"
                            color="warning"
                            label="Unregistered / custom"
                          />
                        )}
                      </div>
                      <details className="mt-2">
                        <summary className="cursor-pointer text-xs text-muted-foreground">
                          {match.providers.length > 0
                            ? 'Resolution details'
                            : 'Why unmatched'}
                        </summary>
                        {match.providers.length > 0 ? (
                          <div className="mt-2 space-y-2">
                            {match.providers.map((providerMatch) => (
                              <div
                                key={`domestic-detail-${match.server}-${providerMatch.provider.kind}`}
                                className="rounded-md border border-border px-3 py-2"
                              >
                                <div className="text-xs font-semibold">
                                  {providerMatch.provider.label}
                                </div>
                                <div className="mt-2 flex flex-wrap gap-2">
                                  {providerMatch.reasons.map((reason) => (
                                    <Chip
                                      key={`domestic-reason-${match.server}-${providerMatch.provider.kind}-${reason.kind}-${reason.value}`}
                                      size="small"
                                      color="default"
                                      label={formatMatchReason(reason)}
                                    />
                                  ))}
                                </div>
                              </div>
                            ))}
                          </div>
                        ) : (
                          <div className="mt-2 text-xs text-muted-foreground">
                            {match.unmatchedReason}
                          </div>
                        )}
                      </details>
                    </div>
                  ))
                ) : (
                  <Chip size="small" label="No domestic runtime DNS" />
                )}
              </div>
            </div>

            <div className="rounded-lg border border-border px-3 py-2">
              <div className="text-xs font-semibold text-muted-foreground">
                Foreign effective providers
              </div>
              <div className="mt-2 space-y-2">
                {foreignProviderMatches.length > 0 ? (
                  foreignProviderMatches.map((match) => (
                    <div
                      key={`foreign-${match.server}`}
                      className="rounded-md border border-border px-3 py-2"
                    >
                      <div className="break-all font-mono text-xs">{match.server}</div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        Authority:{' '}
                        <span className="font-mono">
                          {match.authority || 'Unavailable'}
                        </span>
                      </div>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {match.providers.length > 0 ? (
                          match.providers.map((providerMatch) => (
                            <Chip
                              key={`foreign-${match.server}-${providerMatch.provider.kind}`}
                              size="small"
                              color="info"
                              label={providerMatch.provider.label}
                            />
                          ))
                        ) : (
                          <Chip
                            size="small"
                            color="warning"
                            label="Unregistered / custom"
                          />
                        )}
                      </div>
                      <details className="mt-2">
                        <summary className="cursor-pointer text-xs text-muted-foreground">
                          {match.providers.length > 0
                            ? 'Resolution details'
                            : 'Why unmatched'}
                        </summary>
                        {match.providers.length > 0 ? (
                          <div className="mt-2 space-y-2">
                            {match.providers.map((providerMatch) => (
                              <div
                                key={`foreign-detail-${match.server}-${providerMatch.provider.kind}`}
                                className="rounded-md border border-border px-3 py-2"
                              >
                                <div className="text-xs font-semibold">
                                  {providerMatch.provider.label}
                                </div>
                                <div className="mt-2 flex flex-wrap gap-2">
                                  {providerMatch.reasons.map((reason) => (
                                    <Chip
                                      key={`foreign-reason-${match.server}-${providerMatch.provider.kind}-${reason.kind}-${reason.value}`}
                                      size="small"
                                      color="default"
                                      label={formatMatchReason(reason)}
                                    />
                                  ))}
                                </div>
                              </div>
                            ))}
                          </div>
                        ) : (
                          <div className="mt-2 text-xs text-muted-foreground">
                            {match.unmatchedReason}
                          </div>
                        )}
                      </details>
                    </div>
                  ))
                ) : (
                  <Chip size="small" label="No foreign runtime DNS" />
                )}
              </div>
            </div>
          </div>
        </div>

        <div className="mt-3 grid grid-cols-1 gap-3 md:grid-cols-2">
          <div className="rounded-lg border border-border px-3 py-2">
            <div className="text-xs text-muted-foreground">Runtime domestic DNS</div>
            <div className="mt-2 flex flex-wrap gap-2">
              {(runtimeStatus?.derived.domestic_dns ?? []).length > 0 ? (
                (runtimeStatus?.derived.domestic_dns ?? []).map((server) => (
                  <Chip key={server} size="small" label={server} />
                ))
              ) : (
                <Chip size="small" label="None" />
              )}
            </div>
            <div className="mt-2 text-xs text-muted-foreground">
              Matched providers:{' '}
              {domesticMatches.length > 0 ? domesticMatches.join(', ') : 'Unmatched'}
            </div>
          </div>

          <div className="rounded-lg border border-border px-3 py-2">
            <div className="text-xs text-muted-foreground">Runtime foreign DNS</div>
            <div className="mt-2 flex flex-wrap gap-2">
              {(runtimeStatus?.derived.foreign_dns ?? []).length > 0 ? (
                (runtimeStatus?.derived.foreign_dns ?? []).map((server) => (
                  <Chip key={server} size="small" label={server} />
                ))
              ) : (
                <Chip size="small" label="None" />
              )}
            </div>
            <div className="mt-2 text-xs text-muted-foreground">
              Matched providers:{' '}
              {foreignMatches.length > 0 ? foreignMatches.join(', ') : 'Unmatched'}
            </div>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
        {providerList.map((provider) => {
          const report = probeReports[provider.kind]
          const preferredProtocol = getPreferredProbeProtocol(provider)

          return (
            <div
              key={provider.kind}
              className="rounded-lg border border-border bg-card p-4"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="flex items-center gap-2 text-sm font-semibold">
                    {provider.label}
                    <Chip
                      size="small"
                      color={PROVIDER_STATUS_COLORS[provider.availability]}
                      label={provider.availability}
                    />
                    {domesticProviderKinds.has(provider.kind) ? (
                      <Chip size="small" color="success" label="Domestic runtime" />
                    ) : null}
                    {foreignProviderKinds.has(provider.kind) ? (
                      <Chip size="small" color="info" label="Foreign runtime" />
                    ) : null}
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {provider.description}
                  </div>
                </div>
                <Button
                  size="small"
                  variant="outlined"
                  loading={probingKind === provider.kind}
                  onClick={() => void handleProbe(provider)}
                >
                  Probe {preferredProtocol?.toUpperCase() ?? 'AUTO'}
                </Button>
              </div>

              <div className="mt-3 grid grid-cols-1 gap-2 text-sm">
                <div>
                  <div className="text-xs text-muted-foreground">Canonical host</div>
                  <div className="font-mono text-xs">{provider.canonical_host}</div>
                </div>

                <div>
                  <div className="text-xs text-muted-foreground">Host aliases</div>
                  <div className="mt-1 flex flex-wrap gap-2">
                    {provider.host_aliases.map((alias) => (
                      <Chip key={alias} size="small" label={alias} />
                    ))}
                  </div>
                </div>

                <div>
                  <div className="text-xs text-muted-foreground">Bootstrap IPs</div>
                  <div className="mt-1 flex flex-wrap gap-2">
                    {provider.bootstrap_ips.map((ip) => (
                      <Chip key={ip} size="small" label={ip} />
                    ))}
                  </div>
                </div>

                <div>
                  <div className="text-xs text-muted-foreground">Protocols</div>
                  <div className="mt-1 flex flex-wrap gap-2">
                    {provider.supported_protocols.map((protocol) => (
                      <Chip
                        key={protocol}
                        size="small"
                        color="info"
                        label={protocol.toUpperCase()}
                      />
                    ))}
                  </div>
                </div>

                <div>
                  <div className="text-xs text-muted-foreground">
                    Recommended servers
                  </div>
                  <div className="mt-2 space-y-2">
                    {provider.recommended_servers.map((server) => (
                      <div
                        key={`${provider.kind}-${server.protocol}-${server.server}`}
                        className="rounded-md border border-border px-3 py-2"
                      >
                        <div className="mb-1">
                          <Chip
                            size="small"
                            color="default"
                            label={server.protocol.toUpperCase()}
                          />
                        </div>
                        <div className="break-all font-mono text-xs">
                          {server.server}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="rounded-lg border border-border px-3 py-2">
                  <div className="flex items-center gap-2">
                    <Chip
                      size="small"
                      color={
                        report
                          ? report.healthy
                            ? HEALTH_STATUS_COLORS.healthy
                            : HEALTH_STATUS_COLORS.unhealthy
                          : HEALTH_STATUS_COLORS.idle
                      }
                      label={
                        report
                          ? report.healthy
                            ? 'Healthy'
                            : 'Unhealthy'
                          : 'Idle'
                      }
                    />
                    {report?.latency_ms != null ? (
                      <span className="text-xs text-muted-foreground">
                        {report.latency_ms} ms
                      </span>
                    ) : null}
                  </div>

                  <div className="mt-2 text-xs text-muted-foreground">
                    Last probe: {report ? formatCheckedAt(report.checked_at) : 'Not yet tested'}
                  </div>
                  <div className="mt-1 text-xs break-all">
                    {report?.message ?? 'Probe this provider to verify direct connectivity.'}
                  </div>
                </div>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
